//! The jobs of e-mailed searches (QRY-07, ADR-0033, amendment 2026-11-24; ADR-0017).
//!
//! - `search-mail.find-due`, every hour: queues a `search-mail.send` job for each e-mailed search
//!   whose hour has come, at its own fixed place in the hour (`place_in_hour`), and moves its next
//!   e-mail on, in one change each.
//! - `search-mail.send`: one e-mail. It is the one place that decides whether an e-mail goes. Nothing
//!   goes while e-mail or `search-mail.allowed` is off (and it goes on by itself when they are on
//!   again); a person who is switched off, or who can no longer open the saved search, makes it
//!   wait for them. The search runs as the person at that moment, through the one search, and the
//!   e-mail shows each field only where the person may read it. Logs name IDs only, never an
//!   address (DATA-LOGS).

use bb_core::ErrorCode;
use bb_core::app_paths;
use bb_core::fields::Field;
use bb_core::id::{JobId, SavedSearchId, UserId};
use bb_core::job::Job;
use bb_core::saved_search::SavedSearch;
use bb_core::search_mail::{
    MailPause, SEARCH_MAIL_ISSUES_MAX, SearchMail, next_due, place_in_hour,
};
use bb_core::settings::MAIL_SHORT;
use bb_core::wording::Region;
use bb_searchlang::{Direction, FieldRef, SortKey, Span, check};
use jiff::{SignedDuration, Timestamp};
use serde::{Deserialize, Serialize};

use super::sending;
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::issues::ReadOnRequest;
use crate::jobs::{JobKind, QueuedJob};
use crate::mailer::MailMessage;
use crate::notifications::{MailedIssue, SearchFound, search_mail_text, send_failure_for_job};
use crate::search::{FieldTarget, Prepared, SearchScope, readable_of, stopped};

/// How many due e-mailed searches one read of the hourly round takes.
const DUE_PAGE: u64 = 200;

/// The payload of a `search-mail.send` job: whose e-mailed search of which saved search, and the
/// moment its e-mail was due; IDs only, never a word of the person. It names the e-mailed search by
/// its person and its saved search, so one stopped and set again (*Undo*) still gets the e-mail its
/// job was queued for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct SendPayload {
    holder: UserId,
    search: SavedSearchId,
    due: Timestamp,
}

/// What a search found for its e-mail.
struct Found {
    issues: Vec<MailedIssue>,
    total: u64,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// One round of `search-mail.find-due`: queues the e-mails whose hour starts before the next
    /// round, each at its place in the hour (none before now), and moves each on to its next day.
    /// How many it queued.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn queue_due_search_mails(
        &self,
        next_round: Timestamp,
    ) -> Result<u64, AppError> {
        let failed = |error| internal::<S>(&error);
        let kind = JobKind::SearchMailSend.name()?;
        let mut queued = 0;
        loop {
            let due = self
                .store()
                .search_mails_due(next_round, DUE_PAGE)
                .await
                .map_err(failed)?;
            let page = due.len();
            for mail in due {
                if self.queue_one(&mail, &kind, next_round).await? {
                    queued += 1;
                }
            }
            if u64::try_from(page).unwrap_or(u64::MAX) < DUE_PAGE {
                return Ok(queued);
            }
        }
    }

    /// Queues the e-mail of one e-mailed search that is due and moves it on to its next day after
    /// the next round, in the person's time zone now. An e-mail that was due while the server did
    /// not run goes once, now, and stands for every hour up to the next round, so a late one is not
    /// followed by another within the hour.
    async fn queue_one(
        &self,
        mail: &SearchMail,
        kind: &bb_core::technical::TechnicalName,
        next_round: Timestamp,
    ) -> Result<bool, AppError> {
        let now = self.now();
        let zone = Region::of(&self.setting_layers_of(Some(mail.holder)).await?)
            .zone()
            .clone();
        let after = mail.next_at.max(next_round);
        let next = next_due(mail.days, mail.hour, &zone, after)
            .ok_or_else(|| AppError::Internal("no next day for an e-mailed search".to_owned()))?;
        let run_at = mail
            .next_at
            .checked_add(place_in_hour(mail.id))
            .map_err(|error| AppError::Internal(error.to_string()))?
            .max(now);
        let job = QueuedJob {
            id: JobId::generate(&mut self.ids())
                .map_err(|error| AppError::Internal(error.to_string()))?,
            kind: kind.clone(),
            payload: serde_json::to_value(SendPayload {
                holder: mail.holder,
                search: mail.search,
                due: mail.next_at,
            })
            .map_err(|error| AppError::Internal(error.to_string()))?,
            run_at,
        };
        self.store()
            .queue_search_mail(mail.id, mail.next_at, next, &job, now)
            .await
            .map_err(|error| internal::<S>(&error))
    }

    /// The job `search-mail.send`: the e-mail of one e-mailed search, or nothing when it may not go
    /// now (the one rule of when nothing goes, see the module).
    ///
    /// # Errors
    /// A refusal for a payload that names nothing or an address the mail server turns away for
    /// good; an internal error when another try may work.
    pub(crate) async fn run_search_mail(&self, job: &Job) -> Result<(), AppError> {
        let SendPayload {
            holder,
            search,
            due,
        } = serde_json::from_value(job.payload.clone())
            .map_err(|_| AppError::Refused(ErrorCode::InvalidRequest))?;
        let Some(mail) = self
            .store()
            .search_mail_of(holder, search)
            .await
            .map_err(|error| internal::<S>(&error))?
        else {
            // Stopped since it was queued.
            return Ok(());
        };
        if mail.paused.is_some() || !self.still_due(&mail, due).await? {
            return Ok(());
        }
        let layers = self.setting_layers_of(None).await?;
        if !sending(&layers) {
            return Ok(());
        }
        let Some(server) = self.mail_server_if_on(&layers).await? else {
            return Ok(());
        };
        // A removed person has no e-mailed search left; one being removed gets nothing.
        let Some(known) = self.person_with_standing(mail.holder).await? else {
            return Ok(());
        };
        if !known.active() {
            return self.pause(&mail, MailPause::SwitchedOff).await;
        }
        let actor = Actor {
            person: known.person,
            key: None,
        };
        let search = match self.open_search(&actor, mail.search).await {
            Ok(opened) => opened.stored.search,
            Err(AppError::Refused(ErrorCode::NotFound)) => {
                return self.pause(&mail, MailPause::NoLongerOpens).await;
            }
            Err(error) => return Err(error),
        };
        let ran_at = self.now();
        let since = mail.last_sent_at.filter(|_| mail.only_when_changed);
        let found = match self.found_for_mail(&actor, &search, since).await {
            Ok(Some(found)) => found,
            // Nothing new or changed since the last e-mail.
            Ok(None) => return Ok(()),
            // The search does not check for the person any more (a field or a name they lost).
            Err(AppError::Mistakes(_)) => {
                return self.pause(&mail, MailPause::NoLongerOpens).await;
            }
            Err(error) => return Err(error),
        };
        let search_path = app_paths::saved_search(&search.id.to_string());
        let composed = search_mail_text(
            &SearchFound {
                name: &search.name,
                search_path: &search_path,
                issues: &found.issues,
                total: found.total,
            },
            layers.yes(&MAIL_SHORT),
            &self.services.public_url,
        );
        let message = MailMessage {
            to: actor.person.email,
            subject: composed.subject,
            text: composed.text,
        };
        self.send_claimed(&mail, (due, ran_at), &server, &message)
            .await
    }

    /// Whether `due` is still a time the e-mailed search goes at, by the choice as it is now in the
    /// person's time zone: a job queued for an hour the person moved away from sends nothing.
    async fn still_due(&self, mail: &SearchMail, due: Timestamp) -> Result<bool, AppError> {
        let zone = Region::of(&self.setting_layers_of(Some(mail.holder)).await?)
            .zone()
            .clone();
        let just_before = due
            .checked_sub(SignedDuration::from_secs(1))
            .map_err(|error| AppError::Internal(error.to_string()))?;
        Ok(next_due(mail.days, mail.hour, &zone, just_before) == Some(due))
    }

    /// Sends the e-mail of the due time `due` when this job claims it: each due time goes at most
    /// once, however many jobs were queued for it (saved again, stopped and set again). A claim
    /// whose e-mail did not go is given back, so another try of the job may send it.
    async fn send_claimed(
        &self,
        mail: &SearchMail,
        (due, ran_at): (Timestamp, Timestamp),
        server: &crate::mailer::MailServer,
        message: &MailMessage,
    ) -> Result<(), AppError> {
        let failed = |error| internal::<S>(&error);
        let store = self.store();
        if !store
            .claim_search_mail(mail.id, due)
            .await
            .map_err(failed)?
        {
            return Ok(());
        }
        match self.services.mailer.send(server, message).await {
            Ok(()) => {
                let schedule = mail.id;
                tracing::info!(%schedule, "an e-mailed search went");
                store
                    .search_mail_sent(mail.id, ran_at)
                    .await
                    .map_err(failed)
            }
            Err(failure) => {
                store
                    .give_back_search_mail(mail.id, due, mail.last_due)
                    .await
                    .map_err(failed)?;
                // A refusal for good ends the job; another try may work for the rest.
                Err(send_failure_for_job(failure))
            }
        }
    }

    /// Makes an e-mailed search wait for its person; nothing goes.
    async fn pause(&self, mail: &SearchMail, pause: MailPause) -> Result<(), AppError> {
        let schedule = mail.id;
        tracing::info!(%schedule, pause = pause.code(), "an e-mailed search waits for its person");
        self.store()
            .pause_search_mail(mail.id, pause)
            .await
            .map_err(|error| internal::<S>(&error))
    }

    /// What the saved search finds for the person now: its first issues in its order and how many
    /// it finds, each issue as the person may see it. `None` when `since` is given and no issue it
    /// finds changed after it.
    async fn found_for_mail(
        &self,
        actor: &Actor,
        search: &SavedSearch,
        since: Option<Timestamp>,
    ) -> Result<Option<Found>, AppError> {
        let mut prepared = self
            .prepare_search(actor, &search.text, SearchScope::Everywhere)
            .await?;
        if let Some(since) = since
            && !self.changed_after(&prepared, since).await?
        {
            return Ok(None);
        }
        let most = u32::try_from(SEARCH_MAIL_ISSUES_MAX).unwrap_or(u32::MAX);
        let page = self
            .issues_page(actor, &mut prepared, None, most, ReadOnRequest::default())
            .await?;
        let total = self
            .store()
            .search_count(&prepared.request)
            .await
            .map_err(stopped)?;
        let issues = page
            .items
            .iter()
            .map(|(held, view)| {
                let mask = held.field_mask_for(Some(view.issue_type.id));
                MailedIssue {
                    key: view.key.clone(),
                    status: mask
                        .can_see(Field::Status)
                        .then(|| view.status.name.clone()),
                    title: mask.can_see(Field::Title).then(|| view.title.clone()),
                }
            })
            .collect();
        Ok(Some(Found { issues, total }))
    }

    /// Whether an issue the prepared search finds changed after `since`: the same search, sorted by
    /// *Changed* newest first, one issue. A person who may not search by *Changed* is sent the
    /// e-mail, as if something changed.
    async fn changed_after(&self, prepared: &Prepared, since: Timestamp) -> Result<bool, AppError> {
        let Some(word) = prepared
            .looked
            .fields
            .iter()
            .find(|each| each.target == FieldTarget::Changed)
            .map(|each| each.field.word.clone())
        else {
            return Ok(true);
        };
        let span = Span::default();
        let mut search = prepared.request.search.clone().into_search();
        search.sort = vec![SortKey {
            field: FieldRef { word, span },
            direction: Some(Direction::NewestFirst),
            span,
        }];
        let mut request = prepared.request.clone();
        request.search =
            check(search, &readable_of(&prepared.looked.fields)).map_err(AppError::Mistakes)?;
        request.after = None;
        request.limit = 1;
        let newest = self.store().search_page(&request).await.map_err(stopped)?;
        Ok(newest
            .first()
            .is_some_and(|hit| hit.issue.updated_at > since))
    }
}
