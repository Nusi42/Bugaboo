//! Setting and stopping an e-mailed search (QRY-07): the person's own, of a saved search they may
//! open. Setting writes the weekdays, the hour and *Only when something is new or changed*, and
//! starts an e-mailed search that waited for its person again.

use bb_core::ErrorCode;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::id::SearchMailId;
use bb_core::search_mail::{MailDays, MailHour, SEARCH_MAILS_MAX, SearchMail, next_due};
use bb_core::wording::Region;
use bb_policy::{Authorized, Place};
use serde_json::{Value, json};

use super::types::{SearchMailView, SetSearchMail, StopSearchMail};
use crate::auth::{AppError, AuthStore, Ctx, Done, by, entity, internal};
use crate::member::search_mail as member;
use crate::projects::Why;

/// What one member of an e-mailed search is in an event.
fn member_value(mail: &SearchMail, name: &str) -> Value {
    match name {
        member::DAYS => json!(mail.days.days()),
        member::HOUR => json!(mail.hour.get()),
        member::ONLY_WHEN_CHANGED => json!(mail.only_when_changed),
        _ => json!(mail.search.to_string()),
    }
}

/// The changes of an event that makes, changes or stops an e-mailed search: each member that is
/// new, different or gone.
fn changes(before: Option<&SearchMail>, after: Option<&SearchMail>) -> Vec<FieldChange> {
    member::ALL
        .into_iter()
        .filter_map(|name| {
            let was = before.map(|mail| member_value(mail, name));
            let is = after.map(|mail| member_value(mail, name));
            (was != is).then(|| FieldChange {
                field: name.to_owned(),
                before: was,
                after: is,
            })
        })
        .collect()
}

/// An e-mailed search as its person sees it; `name` is the name of its saved search while they
/// may open it.
pub(super) fn view_of(mail: &SearchMail, name: Option<String>) -> SearchMailView {
    SearchMailView {
        search: mail.search,
        name,
        days: mail.days.days(),
        hour: mail.hour.get(),
        only_when_changed: mail.only_when_changed,
        paused: mail.paused,
        last_sent_at: mail.last_sent_at,
    }
}

/// Gets a saved search by e-mail, or changes when it comes; an e-mailed search that waited for its
/// person sends again. The person must be able to open the saved search now.
///
/// # Errors
/// `request.invalid` for no weekday or an hour outside 0 to 23; `resource.not-found` for a saved
/// search the person may not open (one answer for all, ADR-0034, decision 4); `limit.reached` for a
/// person who has [`SEARCH_MAILS_MAX`] e-mailed searches already; `edit.version-changed` when another
/// change made the same e-mailed search a moment before.
pub async fn set_search_mail<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SetSearchMail,
) -> Result<Authorized<SearchMailView>, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let invalid = || AppError::Refused(ErrorCode::InvalidRequest);
    let days = MailDays::new(input.days).ok_or_else(invalid)?;
    let hour = MailHour::new(input.hour).ok_or_else(invalid)?;
    let opened = ctx.open_search(&actor, input.search).await?;
    let holder = actor.person.id;
    let zone = Region::of(&ctx.setting_layers_of(Some(holder)).await?)
        .zone()
        .clone();
    let now = ctx.now();
    let next_at = next_due(days, hour, &zone, now)
        .ok_or_else(|| AppError::Internal("no next day for an e-mailed search".to_owned()))?;
    let before = ctx
        .store()
        .search_mail_of(holder, input.search)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let id = match &before {
        Some(found) => found.id,
        None => SearchMailId::generate(&mut ctx.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?,
    };
    let mail = SearchMail {
        id,
        holder,
        search: input.search,
        days,
        hour,
        only_when_changed: input.only_when_changed,
        paused: None,
        next_at,
        last_sent_at: before.as_ref().and_then(|found| found.last_sent_at),
        created_at: before.as_ref().map_or(now, |found| found.created_at),
        updated_at: now,
    };
    let event = ctx.event(
        by(holder),
        entity(RecordKind::SearchMail, id),
        changes(before.as_ref(), Some(&mail)),
    )?;
    let why = Why::default();
    let chosen = mail.clone();
    let stop = why.clone();
    let mail = ctx
        .store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                // Under the lock of the person's saved searches, so two changes at once cannot pass
                // the limit (one lock serves both).
                S::lock_saved_searches_of(transaction, holder).await?;
                let written = match S::search_mail_of_in(transaction, holder, chosen.search).await?
                {
                    // What the jobs keep (when the last e-mail went) is read inside the change, so
                    // a job that wrote meanwhile is not undone.
                    Some(found) if found.id == chosen.id => SearchMail {
                        last_sent_at: found.last_sent_at,
                        created_at: found.created_at,
                        ..chosen
                    },
                    // Made by another change a moment ago: this one is told so.
                    Some(_) => return Err(stop.stop::<S>(ErrorCode::VersionChanged)),
                    None => {
                        let have = S::count_search_mails_of_in(transaction, holder).await?;
                        if have >= u64::try_from(SEARCH_MAILS_MAX).unwrap_or(u64::MAX) {
                            return Err(stop.stop::<S>(ErrorCode::LimitReached));
                        }
                        chosen
                    }
                };
                S::put_search_mail(transaction, &written).await?;
                Ok(written)
            })
        })
        .await
        .map_err(|error| why.said_or_failed::<S>(&error))?;
    let organization = ctx.evaluation_of(&actor, Place::Organization).await?;
    Ok(organization.authorize(view_of(&mail, Some(opened.stored.search.name))))
}

/// Stops getting a saved search by e-mail; *Undo* sets it again.
///
/// # Errors
/// `resource.not-found` when the person gets no e-mail of that saved search.
pub async fn stop_search_mail<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: StopSearchMail,
) -> Result<Done, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let holder = actor.person.id;
    let found = ctx
        .store()
        .search_mail_of(holder, input.search)
        .await
        .map_err(|error| internal::<S>(&error))?
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let event = ctx.event(
        by(holder),
        entity(RecordKind::SearchMail, found.id),
        changes(Some(&found), None),
    )?;
    let why = Why::default();
    let stop = why.clone();
    let id = found.id;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if S::delete_search_mail(transaction, id).await? {
                    Ok(())
                } else {
                    Err(stop.stop::<S>(ErrorCode::NotFound))
                }
            })
        })
        .await
        .map_err(|error| why.said_or_failed::<S>(&error))?;
    Ok(Done)
}
