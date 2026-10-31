//! One's own week compared with the time booked on issues (TIME-04, ADR-0035, decisions 4 and 8):
//! the only place where working hours and time entries meet. The time booked is read through the
//! port of time spent (`TimeSpentStore::booked_per_date`), for the person themselves only: nobody
//! sees this comparison for another person.

use std::collections::BTreeMap;

use bb_core::ErrorCode;
use bb_core::time::Minutes;
use bb_core::wording::Region;
use bb_core::working_time::Totals;
use bb_policy::{Authorized, Place};

use super::read::view_of;
use super::types::{BookedDay, BookedWeek, BookedWeekQuery};
use crate::auth::{AppError, AuthStore, Ctx, internal};

/// The person's own week with the time booked on issues each day.
///
/// # Errors
/// `request.invalid` for a date at the very ends of the calendar.
pub async fn booked_week<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: BookedWeekQuery,
) -> Result<Authorized<BookedWeek>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let person = actor.person.id;
    let region = Region::of(&ctx.setting_layers_of(Some(person)).await?);
    let week = region
        .week_of(input.date)
        .ok_or(ErrorCode::InvalidRequest)?;
    let dates = week.days();
    let (Some(first), Some(last)) = (dates.first().copied(), dates.last().copied()) else {
        return Err(ErrorCode::InvalidRequest.into());
    };
    let now = ctx.now();
    let worked: BTreeMap<_, Totals> = ctx
        .stored_days(person, first, last)
        .await?
        .iter()
        .map(|day| (day.day.date, view_of(day, now).totals))
        .collect();
    let booked: BTreeMap<_, Minutes> = ctx
        .store()
        .booked_per_date(person, first, last)
        .await
        .map_err(|error| internal::<S>(&error))?
        .into_iter()
        .collect();
    let days = dates
        .into_iter()
        .map(|date| {
            let worked = worked
                .get(&date)
                .map(|totals| totals.worked)
                .unwrap_or_default();
            let booked = booked.get(&date).copied().unwrap_or_default();
            BookedDay {
                date,
                worked,
                booked,
                not_booked: i64::from(worked) - i64::from(booked),
            }
        })
        .collect();
    Ok(ctx
        .evaluation_of(&actor, Place::Organization)
        .await?
        .authorize(BookedWeek { days }))
}
