//! The work days of a stretch of dates (TIME-08, TIME-12): one's own, or another person's on
//! *Working hours of others*, where *Totals only* leaves the week totals and nothing else (ADR-0035,
//! decision 8). Every total comes from `bb_core::working_time::Totals`, the one function of the
//! totals of a day.

use bb_core::ErrorCode;
use bb_core::id::UserId;
use bb_core::wording::Region;
use bb_core::working_time::Totals;
use bb_core::working_time::record::StoredDay;
use bb_policy::Authorized;
use jiff::Timestamp;
use jiff::civil::Date;

use super::RANGE_DAYS_MAX;
use super::types::{WeekTotal, WorkDayView, WorkDaysPage, WorkDaysQuery};
use crate::auth::{AppError, AuthStore, Ctx, internal};

/// A stored day as its reader sees it; what runs counts until `now`.
pub(super) fn view_of(day: &StoredDay, now: Timestamp) -> WorkDayView {
    WorkDayView {
        date: day.day.date,
        zone: day.day.zone.clone(),
        totals: Totals::of(&day.times, now),
        warnings: day.day.warnings.clone(),
        runs: day.times.runs(),
        times: day.times.clone(),
        version: day.day.version,
    }
}

/// Whether `first` to `last` is a stretch one read may cover.
pub(super) fn checked_range(first: Date, last: Date) -> Result<(), AppError> {
    let days = first.until(last).map(|span| i64::from(span.get_days()));
    if days.is_ok_and(|days| (0..RANGE_DAYS_MAX).contains(&days)) {
        Ok(())
    } else {
        Err(ErrorCode::InvalidRequest.into())
    }
}

/// The time worked in each week of the stretch, in the order of the weeks.
fn weeks_of(views: &[WorkDayView], first: Date, last: Date, region: &Region) -> Vec<WeekTotal> {
    let mut weeks: Vec<WeekTotal> = Vec::new();
    let mut date = first;
    while date <= last {
        if let Some(week) = region.week_of(date)
            && weeks.last().is_none_or(|known| known.first != week.first())
        {
            let worked: Totals = views
                .iter()
                .filter(|view| region.week_of(view.date) == Some(week))
                .map(|view| view.totals)
                .sum();
            weeks.push(WeekTotal {
                first: week.first(),
                worked: worked.worked,
            });
        }
        let Ok(next) = date.tomorrow() else {
            break;
        };
        date = next;
    }
    weeks
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The stored days of a person from `first` to `last`.
    pub(super) async fn stored_days(
        &self,
        person: UserId,
        first: Date,
        last: Date,
    ) -> Result<Vec<StoredDay>, AppError> {
        self.store()
            .work_days(person, first, last)
            .await
            .map_err(|error| internal::<S>(&error))
    }
}

/// The work days of a stretch of dates.
///
/// # Errors
/// `request.invalid` for a stretch that is backwards or longer than [`RANGE_DAYS_MAX`] days, and
/// `resource.not-found` for another person the reader may not see (`whose.rs`).
pub async fn work_days<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: WorkDaysQuery,
) -> Result<Authorized<WorkDaysPage>, AppError> {
    let actor = ctx.asking(&input.session_token, input.person).await?;
    let whose = ctx.days_to_read(&actor, input.person).await?;
    checked_range(input.first, input.last)?;
    let now = ctx.now();
    let views: Vec<WorkDayView> = ctx
        .stored_days(whose.person, input.first, input.last)
        .await?
        .iter()
        .map(|day| view_of(day, now))
        .collect();
    let region = Region::of(&ctx.setting_layers_of(Some(whose.person)).await?);
    let weeks = weeks_of(&views, input.first, input.last, &region);
    let totals_only = whose.totals_only();
    let page = WorkDaysPage {
        person: ctx.person_of(&whose).await?,
        totals: (!totals_only).then(|| views.iter().map(|view| view.totals).sum()),
        days: if totals_only { Vec::new() } else { views },
        weeks,
        totals_only,
        may_change: whose.may_change(),
    };
    Ok(whose.evaluation.authorize(page))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stretch_is_forwards_and_at_most_two_months() {
        let first = Date::new(2026, 10, 1).unwrap_or_default();
        assert!(checked_range(first, first).is_ok());
        let last = first
            .checked_add(jiff::Span::new().days(RANGE_DAYS_MAX - 1))
            .unwrap_or_default();
        assert!(checked_range(first, last).is_ok());
        assert!(checked_range(first, last.tomorrow().unwrap_or_default()).is_err());
        assert!(checked_range(last, first).is_err());
    }
}
