//! Reading time entries (TIME-01, TIME-02, PRIV-09): a person's own entries of some dates, and the
//! entries on an issue.
//!
//! On an issue each person is shown as *Who sees people* allows for the kind *Time entries*, and the
//! length and the note only to a reader who may read *Time spent* on that issue. Nothing here
//! orders or ranks people: a person's own list is by date, the list of an issue by when the entries
//! were made (ADR-0035, decision 9). The values of own fields and the people are read once for a
//! page (PERF-N1).

use bb_core::ErrorCode;
use bb_core::event::RecordKind;
use bb_core::fields::Field;
use bb_core::id::{EntityId, TimeEntryId};
use bb_core::paging::{Cut, page_len, read_len};
use bb_core::people_kinds::PeopleKind;
use bb_policy::{Authorized, PeopleContext, Permission, Place};

use super::store::DayKey;
use super::types::{
    EntryDefaults, EntryDefaultsQuery, IssueTimeEntries, IssueTimeEntriesQuery, IssueTimeEntryView,
    MyTimeEntriesQuery, TimeEntryPage,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::input::cursor_or_invalid;
use bb_core::time::day::Week;
use jiff::civil::Date;

impl<S: AuthStore> Ctx<'_, S> {
    /// The first and the last date of the actor's week of today, by their *Region*.
    async fn this_week(&self, actor: &Actor) -> Result<(Date, Date), AppError> {
        let (today, region) = self.day_of(actor.person.id, self.now()).await?;
        let days = Week::of(today, region.first_day())
            .map(|week| week.days())
            .unwrap_or_default();
        match (days.first(), days.last()) {
            (Some(first), Some(last)) => Ok((*first, *last)),
            _ => Err(ErrorCode::InvalidRequest.into()),
        }
    }
}

/// The entry a page starts after, from the `next_cursor` of the page before.
fn entry_cursor(cursor: Option<&str>) -> Result<Option<TimeEntryId>, AppError> {
    cursor_or_invalid(cursor, |text| text.parse().ok())
}

/// The person's own time entries from one date to another (both included), by date, then in the
/// order they were made.
///
/// # Errors
/// `request.invalid` for a last date before the first or a cursor that is none.
pub async fn my_time_entries<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: MyTimeEntriesQuery,
) -> Result<Authorized<TimeEntryPage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    // Time is a person's own: an agent never reads it (M7-Q7, ADR-0035, decision 9).
    actor.ensure_not_an_agent()?;
    let (from, to) = match (input.from, input.to) {
        (Some(from), Some(to)) => (from, to),
        (None, None) => ctx.this_week(&actor).await?,
        _ => return Err(ErrorCode::InvalidRequest.into()),
    };
    if to < from {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let after = entry_cursor(input.cursor.as_deref())?;
    let len = page_len(Some(input.limit));
    let found = ctx
        .store()
        .entries_of_person(actor.person.id, from, to, after, read_len(len))
        .await
        .map_err(|error| internal::<S>(&error))?;
    let cut = Cut::of(found, len);
    let items = ctx.entry_views(&actor, &cut.items).await?;
    let next_cursor = cut.next_cursor(|entry| entry.id.to_string());
    ctx.own_answer(&actor, TimeEntryPage { items, next_cursor })
        .await
}

/// The time entries on an issue the reader sees, newest first: each person as *Who sees people*
/// allows for the kind *Time entries*, the length and the note only for a reader who may read
/// *Time spent*.
///
/// # Errors
/// `resource.not-found` for an issue the reader does not see, `request.invalid` for a cursor that
/// is none.
pub async fn issue_time_entries<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: IssueTimeEntriesQuery,
) -> Result<Authorized<IssueTimeEntries>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (issue, held) = ctx.visible_issue(&actor, &input.issue).await?;
    let sees_moments = ctx
        .evaluation_of(&actor, Place::Organization)
        .await?
        .allows(Permission::SeeWorkingHoursOfOthers);
    // The order of the list and the place of a cursor in it tell when entries were saved, so a
    // reader who may not see that moment gets the list by date, in no order of making.
    let (entries, next_cursor) = if sees_moments {
        let after = entry_cursor(input.cursor.as_deref())?;
        let len = page_len(Some(input.limit));
        let found = ctx
            .store()
            .entries_of_issue(issue.id, after, read_len(len))
            .await
            .map_err(|error| internal::<S>(&error))?;
        let cut = Cut::of(found, len);
        let next = cut.next_cursor(|entry| entry.id.to_string());
        (cut.items, next)
    } else {
        let after = cursor_or_invalid(input.cursor.as_deref(), |text| text.parse::<DayKey>().ok())?;
        let len = page_len(Some(input.limit));
        let found = ctx
            .store()
            .entries_of_issue_by_date(issue.id, after, read_len(len))
            .await
            .map_err(|error| internal::<S>(&error))?;
        let cut = Cut::of(found, len);
        let next = cut.next_cursor(|(_, key)| key.to_string());
        (
            cut.items.into_iter().map(|(entry, _)| entry).collect(),
            next,
        )
    };
    let spent = held
        .field_mask_for(Some(issue.issue_type))
        .can_see(Field::TimeSpent);
    let ids: Vec<EntityId> = entries.iter().map(|entry| entry.id.into()).collect();
    let values = ctx.load_values(RecordKind::TimeEntry, &ids).await?;
    let people: Vec<bb_core::id::UserId> = entries.iter().map(|entry| entry.person).collect();
    ctx.preload_people(&people).await?;
    let mut items = Vec::with_capacity(entries.len());
    for entry in &entries {
        items.push(IssueTimeEntryView {
            id: entry.id,
            moment_visible: sees_moments || entry.person == actor.person.id,
            person: ctx
                .person_ref(
                    &held,
                    PeopleKind::TimeEntries,
                    PeopleContext::Ordinary,
                    entry.person,
                )
                .await?,
            date: entry.date,
            minutes: spent.then_some(entry.minutes),
            note: spent.then(|| entry.note.clone()),
            own_fields: values.shown(&held, Some(issue.issue_type), entry.id.into()),
        });
    }
    Ok(held.authorize(IssueTimeEntries { items, next_cursor }))
}

/// What a new time entry on an issue starts with: whether *Billable* is offered and the defaults of
/// the own fields of time entries, as the person may see them (ADR-0035, decision 1). The *Stop
/// timer* window and *Add time* show these, and the entry they save takes the same defaults for a
/// field they leave out.
///
/// # Errors
/// `resource.not-found` for an issue the person does not see, `access.forbidden` without *Book
/// time* there.
pub async fn time_entry_defaults<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: EntryDefaultsQuery,
) -> Result<Authorized<EntryDefaults>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (issue, held) = ctx.bookable_issue(&actor, &input.issue).await?;
    let item = ctx.entry_item(&issue, None).await?;
    let record = EntityId::from(issue.id);
    let on_issue = ctx.load_values(RecordKind::Issue, &[record]).await?;
    let defaults = ctx
        .entry_defaults_from(&issue, &item.chain, &on_issue)
        .await?;
    let own_fields =
        on_issue
            .holding(record, defaults)
            .shown(&held, Some(issue.issue_type), record);
    let billable_offered = ctx.billable_offered().await?;
    Ok(held.authorize(EntryDefaults {
        billable_offered,
        own_fields,
    }))
}
