//! The outbox reader that builds the notifications (NTF-01, SEC-06, ADR-0007, ADR-0033 §1–§2).
//!
//! It reads the events it has not finished, works out for each one who hears of it, and stores one
//! row for each person (`store_notifications`), moving its place on in the same step. It is
//! idempotent: a row is made once for each person and event (a mention once for each person and
//! comment), so reading an event again, after a restart or when two readers catch up at the same
//! moment, changes nothing, and it resumes from the place it recorded.
//!
//! **Checked when built (§2).** A person who may not see the issue hears nothing, and the row
//! keeps only the fields that person could see: a hidden field is never in a row. A follower who
//! lost access is dropped from the followers (M3-F132). The row is checked again when it is shown
//! or sent (`read.rs`, `mail.rs`).
//!
//! It takes the context for the policy engine, and is no use case of a person (no actor, no
//! domain event of its own besides dropping a follower, by Bugaboo itself), so it is not a command
//! of ARCH-EVENTS.

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use bb_core::comment::Mention;
use bb_core::event::{Actor as EventActor, DomainEvent, EntityKind, FieldChange, RecordKind};
use bb_core::id::{CommentId, EntityId, IssueId, JobId, NotificationId, UserId};
use bb_core::issue::Issue;
use bb_core::notification::{
    BATCH, FIELDS_MAX, MailState, Notification, NotificationKind, Once, Via, mail_batch,
};
use bb_core::person::Person;
use bb_core::settings::Layers;
use serde_json::{Value, json};

use super::mail::MailJob;
use super::plan::{edit_note_of, followed_kind, newly_assigned};
use super::prefs::wants;
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, Stops, change_failed, entity, internal};
use crate::history::{issue_in, visible_changes};
use crate::issues::Where;
use crate::jobs::{QueuedJob, kind, kind_name};
use crate::member::{comment, follower};

/// What one batch of events looks up once: people, their settings, the followers that lost
/// access.
struct Round {
    /// The people looked up so far; `None` for a person who is not there.
    people: HashMap<UserId, Option<Person>>,
    /// The values of the settings of the people looked up so far.
    layers: HashMap<UserId, Layers>,
    /// Whether the organization has switched e-mail on.
    mail_on: bool,
    /// The rows made so far.
    rows: Vec<Notification>,
    /// The followers who may not see the issue any more.
    drops: Vec<(IssueId, UserId)>,
}

/// Who might hear of an event, and in what order of importance (the first kind the person wants
/// is the one they hear it as).
struct Hears {
    person: UserId,
    /// The ways the person is involved, most important first: the kind, or `None` for the kind
    /// that follows from the changes they may see (a follower).
    kinds: Vec<Option<NotificationKind>>,
    /// Whether the person hears of it because they follow the issue (they are dropped from the
    /// followers when they may not see it any more).
    follows: bool,
    /// What makes the row once for the person; the event unless it is a mention.
    once: Once,
}

/// What an event is about, for the rows it makes.
struct Subject<'a> {
    event: &'a DomainEvent,
    issue: Option<&'a Issue>,
    /// The record that is not the issue: the comment, the run.
    record: Option<EntityId>,
    /// What the rows keep of the changes of the event for a person: the fields they may see.
    keeps: Keeps,
}

/// Which of the changes of an event a row keeps.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Keeps {
    /// The changes to fields of the issue that the person may see.
    VisibleFields,
    /// The changes as they are: they are no fields of an issue (a note about a lock, the end of a
    /// run).
    All,
    /// None: the row is about a comment, which is read when it is shown.
    Nothing,
}

/// The person who did it, if a person did.
const fn doer(event: &DomainEvent) -> Option<UserId> {
    match event.actor {
        EventActor::Person(person) => Some(person),
        EventActor::System => None,
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The person with this ID, looked up once for the round; `None` for a person who is not
    /// there.
    async fn round_person(
        &self,
        round: &mut Round,
        id: UserId,
    ) -> Result<Option<Person>, AppError> {
        if let Some(known) = round.people.get(&id) {
            return Ok(known.clone());
        }
        let found = self
            .store()
            .person(id)
            .await
            .map_err(|error| internal::<S>(&error))?;
        round.people.insert(id, found.clone());
        Ok(found)
    }

    /// The values of the settings of a person, read once for the round.
    async fn round_layers<'r>(
        &self,
        round: &'r mut Round,
        id: UserId,
    ) -> Result<&'r Layers, AppError> {
        match round.layers.entry(id) {
            Entry::Occupied(known) => Ok(known.into_mut()),
            Entry::Vacant(slot) => Ok(slot.insert(self.setting_layers_of(Some(id)).await?)),
        }
    }

    /// Catches the notifications up with every event not yet read: the job runner does it at every
    /// round, and a reader of notifications does it first. Returns how many events it read.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub async fn catch_up_notifications(&self) -> Result<u64, AppError> {
        let mut read = 0_u64;
        loop {
            let events = self.notification_batch(BATCH).await?;
            read = read.saturating_add(events);
            if events < BATCH {
                return Ok(read);
            }
        }
    }

    /// Reads at most `batch` events the reader has not finished and makes their rows. Returns how
    /// many events it read: 0 when it has caught up.
    async fn notification_batch(&self, batch: u64) -> Result<u64, AppError> {
        let failed = |error| internal::<S>(&error);
        let events = self
            .store()
            .notification_events(batch)
            .await
            .map_err(failed)?;
        let Some(last) = events.last().map(|(position, _)| *position) else {
            return Ok(0);
        };
        let mut round = Round {
            people: HashMap::new(),
            layers: HashMap::new(),
            mail_on: self.mail_is_on().await?,
            rows: Vec::new(),
            drops: Vec::new(),
        };
        for (_, event) in &events {
            self.notifications_of_event(&mut round, event).await?;
        }
        let jobs = self.mail_jobs_for(&round.rows)?;
        self.store()
            .store_notifications(&round.rows, &jobs, last, self.now())
            .await
            .map_err(failed)?;
        self.drop_followers(&round.drops).await?;
        Ok(u64::try_from(events.len()).unwrap_or(u64::MAX))
    }

    /// The jobs that send the e-mail of the rows that wait: one for each person and issue, to run
    /// when the changes to the issue have had five minutes to come (M4-Q6).
    fn mail_jobs_for(&self, rows: &[Notification]) -> Result<Vec<QueuedJob>, AppError> {
        let mut jobs: Vec<QueuedJob> = Vec::new();
        let mut seen: Vec<(UserId, Option<IssueId>)> = Vec::new();
        for row in rows.iter().filter(|row| row.mail == MailState::Waiting) {
            let key = (row.recipient, row.issue);
            if seen.contains(&key) {
                continue;
            }
            seen.push(key);
            let run_at = row
                .at
                .checked_add(mail_batch())
                .map_err(|error| AppError::Internal(error.to_string()))?;
            jobs.push(QueuedJob {
                id: JobId::generate(&mut self.ids())
                    .map_err(|error| AppError::Internal(error.to_string()))?,
                kind: kind_name(kind::MAIL_SEND)?,
                payload: MailJob::Changes {
                    recipient: row.recipient,
                    issue: row.issue,
                }
                .to_value(),
                run_at,
            });
        }
        Ok(jobs)
    }

    /// The rows an event makes.
    async fn notifications_of_event(
        &self,
        round: &mut Round,
        event: &DomainEvent,
    ) -> Result<(), AppError> {
        let kind_is = |kind: RecordKind| event.entity.kind == EntityKind::from(kind);
        if event
            .changes
            .iter()
            .any(|change| change.field == crate::member::edit_lock::FIELD)
        {
            return self.edit_note_event(round, event).await;
        }
        if kind_is(RecordKind::Issue) {
            return self.issue_event(round, event).await;
        }
        if kind_is(RecordKind::Comment) {
            return self.comment_event(round, event).await;
        }
        if kind_is(RecordKind::ChangeRun) {
            return self.run_event(round, event).await;
        }
        Ok(())
    }

    /// A change to an issue: the people it was assigned to, and the people who follow it.
    async fn issue_event(&self, round: &mut Round, event: &DomainEvent) -> Result<(), AppError> {
        let Ok(id) = IssueId::from_bytes(event.entity.id.to_bytes()) else {
            return Ok(());
        };
        let Some(issue) = self
            .store()
            .issue(id)
            .await
            .map_err(|error| internal::<S>(&error))?
        else {
            return Ok(());
        };
        let mut hears: Vec<Hears> = newly_assigned(&event.changes)
            .into_iter()
            .map(|person| Hears {
                person,
                kinds: vec![Some(NotificationKind::Assigned)],
                follows: false,
                once: Once::Event(event.id),
            })
            .collect();
        let followers = self
            .store()
            .followers_of(id)
            .await
            .map_err(|error| internal::<S>(&error))?;
        // Only the people who followed the issue when it happened: the reader may be late.
        for found in followers
            .into_iter()
            .filter(|found| found.since <= event.time)
        {
            match hears.iter_mut().find(|hear| hear.person == found.person) {
                Some(hear) => {
                    hear.kinds.push(None);
                    hear.follows = true;
                }
                None => hears.push(Hears {
                    person: found.person,
                    kinds: vec![None],
                    follows: true,
                    once: Once::Event(event.id),
                }),
            }
        }
        let subject = Subject {
            event,
            issue: Some(&issue),
            record: None,
            keeps: Keeps::VisibleFields,
        };
        self.deliver(round, &subject, hears).await
    }

    /// A comment: the people it mentions, and the people who follow the issue. An edit of a
    /// comment tells only the people it mentions now and did not before.
    async fn comment_event(&self, round: &mut Round, event: &DomainEvent) -> Result<(), AppError> {
        let Ok(id) = CommentId::from_bytes(event.entity.id.to_bytes()) else {
            return Ok(());
        };
        let failed = |error| internal::<S>(&error);
        let Some(found) = self.store().comment(id).await.map_err(failed)? else {
            return Ok(());
        };
        if found.deleted_at.is_some() {
            return Ok(());
        }
        let wrote = !event.counted_version();
        // Only the event that wrote the comment, and one that changed its text, can mention
        // somebody new.
        if !wrote
            && !event
                .changes
                .iter()
                .any(|change| change.field == comment::TEXT)
        {
            return Ok(());
        }
        let issue_id = issue_in(&event.changes, comment::ISSUE, true).unwrap_or(found.issue);
        let Some(issue) = self.store().issue(issue_id).await.map_err(failed)? else {
            return Ok(());
        };
        let mentions: Vec<Mention> = self.store().mentions_in_comment(id).await.map_err(failed)?;
        let mut hears: Vec<Hears> = mentions
            .iter()
            .map(|mention| Hears {
                person: mention.recipient,
                kinds: vec![Some(NotificationKind::Mentioned)],
                follows: false,
                once: Once::Mention(id),
            })
            .collect();
        if wrote {
            let followers = self.store().followers_of(issue_id).await.map_err(failed)?;
            for found in followers
                .into_iter()
                .filter(|found| found.since <= event.time)
            {
                if !hears.iter().any(|hear| hear.person == found.person) {
                    hears.push(Hears {
                        person: found.person,
                        kinds: vec![Some(NotificationKind::FollowedActivity)],
                        follows: true,
                        once: Once::Event(event.id),
                    });
                }
            }
        }
        let subject = Subject {
            event,
            issue: Some(&issue),
            record: Some(id.into()),
            keeps: Keeps::Nothing,
        };
        self.deliver(round, &subject, hears).await
    }

    /// The note about an edit lock (*Ask … to finish*, *Edit anyway*): the person who was asked or
    /// who lost the editing hears of it (M3-F75).
    async fn edit_note_event(
        &self,
        round: &mut Round,
        event: &DomainEvent,
    ) -> Result<(), AppError> {
        let Some(note) = edit_note_of(&event.changes) else {
            return Ok(());
        };
        let failed = |error| internal::<S>(&error);
        let kind_is = |kind: RecordKind| event.entity.kind == EntityKind::from(kind);
        let issue_id = if kind_is(RecordKind::Issue) {
            IssueId::from_bytes(event.entity.id.to_bytes()).ok()
        } else if kind_is(RecordKind::Comment) {
            match CommentId::from_bytes(event.entity.id.to_bytes()) {
                Ok(id) => self
                    .store()
                    .comment(id)
                    .await
                    .map_err(failed)?
                    .map(|found| found.issue),
                Err(_) => None,
            }
        } else {
            None
        };
        let Some(issue_id) = issue_id else {
            return Ok(());
        };
        let Some(issue) = self.store().issue(issue_id).await.map_err(failed)? else {
            return Ok(());
        };
        let subject = Subject {
            event,
            issue: Some(&issue),
            record: Some(event.entity.id),
            keeps: Keeps::All,
        };
        let hears = vec![Hears {
            person: note.person,
            kinds: vec![Some(NotificationKind::EditNotice)],
            follows: false,
            once: Once::Event(event.id),
        }];
        self.deliver(round, &subject, hears).await
    }

    /// The end of a run of many changes: the person who asked for it hears of it (M3-F125).
    async fn run_event(&self, round: &mut Round, event: &DomainEvent) -> Result<(), AppError> {
        let Ok(id) = bb_core::id::ManyChangeRunId::from_bytes(event.entity.id.to_bytes()) else {
            return Ok(());
        };
        let Some(run) = self
            .store()
            .many_changes_run(id)
            .await
            .map_err(|error| internal::<S>(&error))?
        else {
            return Ok(());
        };
        let ended = event.changes.iter().any(|change| {
            change.field == "status"
                && matches!(
                    change.after.as_ref().and_then(Value::as_str),
                    Some(word) if word == bb_core::many_changes::ManyChangeStatus::Done.code()
                        || word == bb_core::many_changes::ManyChangeStatus::Failed.code()
                        || word == bb_core::many_changes::ManyChangeStatus::Stopped.code()
                )
        });
        if !ended {
            return Ok(());
        }
        let subject = Subject {
            event,
            issue: None,
            record: Some(event.entity.id),
            keeps: Keeps::All,
        };
        let hears = vec![Hears {
            person: run.person,
            kinds: vec![Some(NotificationKind::RunEnded)],
            follows: false,
            once: Once::Event(event.id),
        }];
        self.deliver(round, &subject, hears).await
    }

    /// Makes the row of each person who hears of the event and wants to: never for the person who
    /// did it, never for a person who may not see the issue, and with only the fields that person
    /// may see.
    async fn deliver(
        &self,
        round: &mut Round,
        subject: &Subject<'_>,
        hears: Vec<Hears>,
    ) -> Result<(), AppError> {
        let by = doer(subject.event);
        for hear in hears {
            if Some(hear.person) == by {
                continue;
            }
            let Some(person) = self.round_person(round, hear.person).await? else {
                continue;
            };
            let mut visible: Vec<FieldChange> = Vec::new();
            if let Some(issue) = subject.issue {
                let actor = Actor { person, key: None };
                let Some(held) = self.seen_in(&actor, issue, Where::Live).await? else {
                    if hear.follows {
                        round.drops.push((issue.id, hear.person));
                    }
                    continue;
                };
                if subject.keeps == Keeps::VisibleFields {
                    let mask = held.field_mask_for(Some(issue.issue_type));
                    visible = visible_changes(&|field| mask.can_see(field), &subject.event.changes)
                        .into_iter()
                        .take(FIELDS_MAX)
                        .cloned()
                        .collect();
                }
            }
            let kept = match subject.keeps {
                Keeps::VisibleFields => visible,
                Keeps::All => subject.event.changes.clone(),
                Keeps::Nothing => Vec::new(),
            };
            for kind in &hear.kinds {
                let kind = match kind {
                    Some(kind) => Some(*kind),
                    None => followed_kind(&kept.iter().collect::<Vec<_>>()),
                };
                let Some(kind) = kind else { continue };
                if self
                    .push_row(round, subject, &hear, kind, kept.clone())
                    .await?
                {
                    break;
                }
            }
        }
        Ok(())
    }

    /// Makes the row of one person for a kind of event, if they want to hear of it in the app or
    /// by e-mail. Returns whether it made one.
    async fn push_row(
        &self,
        round: &mut Round,
        subject: &Subject<'_>,
        hear: &Hears,
        kind: NotificationKind,
        changes: Vec<FieldChange>,
    ) -> Result<bool, AppError> {
        let mail_on = round.mail_on;
        let layers = self.round_layers(round, hear.person).await?;
        let in_app = wants(layers, kind, Via::App);
        let by_mail = mail_on && wants(layers, kind, Via::Mail);
        if !in_app && !by_mail {
            return Ok(false);
        }
        let id = NotificationId::generate(&mut self.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?;
        round.rows.push(Notification {
            id,
            recipient: hear.person,
            kind,
            issue: subject.issue.map(|issue| issue.id),
            event: subject.event.id,
            once: hear.once,
            record: subject.record,
            by: doer(subject.event),
            at: subject.event.time,
            changes,
            in_app,
            mail: if by_mail {
                MailState::Waiting
            } else {
                MailState::Never
            },
            read_at: None,
        });
        Ok(true)
    }

    /// Takes the followers out who may not see their issue any more (M3-F132): Bugaboo itself
    /// removes them, with an event of its own, and only those who still follow.
    async fn drop_followers(&self, drops: &[(IssueId, UserId)]) -> Result<(), AppError> {
        let mut done: Vec<(IssueId, UserId)> = Vec::new();
        for &(issue, person) in drops {
            if done.contains(&(issue, person)) {
                continue;
            }
            done.push((issue, person));
            let still = self
                .store()
                .followers_of(issue)
                .await
                .map_err(|error| internal::<S>(&error))?
                .iter()
                .any(|found| found.person == person);
            if !still {
                continue;
            }
            let event = self.event(
                EventActor::System,
                entity(RecordKind::Follower, issue),
                vec![FieldChange {
                    field: follower::PERSON.to_owned(),
                    before: Some(json!(person.to_string())),
                    after: None,
                }],
            )?;
            self.store()
                .record_change(&event, move |transaction| {
                    Box::pin(async move {
                        S::remove_follower(transaction, issue, person)
                            .await
                            .map(|_| ())
                    })
                })
                .await
                .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
        }
        Ok(())
    }
}
