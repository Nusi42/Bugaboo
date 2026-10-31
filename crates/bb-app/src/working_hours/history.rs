//! The history of one work day (TIME-10, ADR-0035, decision 7): every change with the times before
//! and after it, who made it and why, read from the events of the day. The person sees who changed
//! their day; another person's history is for the holders of *See working hours of others*, and
//! *Totals only* hides it like the days themselves.

use bb_core::ErrorCode;
use bb_core::event::{Actor as EventActor, DomainEvent};
use bb_core::people_kinds::PeopleKind;
use bb_core::working_time::{Span, Times};
use bb_policy::{Authorized, PersonRef};
use serde::Deserialize;
use serde_json::Value;

use super::HISTORY_CHANGES_MAX;
use super::types::{WorkDayChange, WorkDayHistory, WorkDayHistoryQuery};
use super::whose::Whose;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::member::work_day as member;

/// The times of a day as its events hold them.
#[derive(Deserialize)]
struct StoredTimes {
    periods: Vec<Span>,
    breaks: Vec<Span>,
}

/// The times an event holds on one side of the change.
fn times_in(value: Option<&Value>) -> Option<Times> {
    let stored: StoredTimes = serde_json::from_value(value?.clone()).ok()?;
    Some(Times {
        periods: stored.periods,
        breaks: stored.breaks,
    })
}

impl<S: AuthStore> Ctx<'_, S> {
    /// One change of the history, as the reader may see who made it.
    async fn change_of(
        &self,
        whose: &Whose,
        event: &DomainEvent,
    ) -> Result<WorkDayChange, AppError> {
        let times = event
            .changes
            .iter()
            .find(|change| change.field == member::TIMES);
        let by = match event.actor {
            EventActor::Person(person) => {
                self.person_ref(
                    &whose.evaluation,
                    PeopleKind::LastChangedBy,
                    whose.context(),
                    person,
                )
                .await?
            }
            EventActor::System => PersonRef::Someone,
        };
        Ok(WorkDayChange {
            at: event.time,
            by,
            reason: event.reason.clone(),
            before: times.and_then(|change| times_in(change.before.as_ref())),
            after: times.and_then(|change| times_in(change.after.as_ref())),
        })
    }
}

/// The history of a work day.
///
/// # Errors
/// `resource.not-found` for a date without a work day and for another person the reader may not
/// see, `working-hours.week-totals-only` while *Totals only* hides another person's days.
pub async fn work_day_history<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: WorkDayHistoryQuery,
) -> Result<Authorized<WorkDayHistory>, AppError> {
    let actor = ctx.asking(&input.session_token, input.person).await?;
    let whose = ctx.days_to_read(&actor, input.person).await?;
    if whose.totals_only() {
        return Err(ErrorCode::TotalsOnly.into());
    }
    let failed = |error| internal::<S>(&error);
    let day = ctx
        .store()
        .work_day(whose.person, input.date)
        .await
        .map_err(failed)?
        .ok_or(ErrorCode::NotFound)?;
    let events = ctx
        .store()
        .work_day_events(day.day.id)
        .await
        .map_err(failed)?;
    let skipped = events.len().saturating_sub(HISTORY_CHANGES_MAX);
    let mut changes = Vec::with_capacity(events.len() - skipped);
    for event in &events[skipped..] {
        changes.push(ctx.change_of(&whose, event).await?);
    }
    Ok(whose.evaluation.authorize(WorkDayHistory {
        changes,
        truncated: skipped > 0,
    }))
}
