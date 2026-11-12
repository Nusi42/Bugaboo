//! A person's work times, as the reader may see them (PLAN-11, ADR-0030): their own in full, another
//! person's as *Who sees people* allows for the kind *Work times*, where an away time shows only
//! its days.

use bb_core::ErrorCode;
use bb_core::holidays::{PublicHoliday, Region as HolidayRegion};
use bb_core::people_kinds::PeopleKind;
use bb_core::time::day::date_of;
use bb_core::wording::Region;
use bb_core::work_times::UsualDays;
use bb_core::work_times::record::AwayTime;
use bb_policy::{Authorized, PeopleContext};
use jiff::civil::Date;

use super::types::{AwayView, WorkTimesQuery, WorkTimesView};
use super::whose::Whose;
use super::{AWAY_TIMES_MAX, NEXT_HOLIDAYS, RANGE_DAYS_MAX};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};

/// Whether `first` to `last` is a stretch one read may cover.
pub(super) fn checked_range(first: Date, last: Date) -> Result<(), AppError> {
    let days = first.until(last).map(|span| i64::from(span.get_days()));
    if days.is_ok_and(|days| (0..RANGE_DAYS_MAX).contains(&days)) {
        Ok(())
    } else {
        Err(ErrorCode::InvalidRequest.into())
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// An away time as the reader sees it: the person sees the note and who entered it, everybody
    /// else only the days.
    pub(super) async fn away_view(
        &self,
        whose: &Whose,
        time: &AwayTime,
    ) -> Result<AwayView, AppError> {
        let entered_by = if whose.own {
            Some(
                self.person_ref(
                    &whose.evaluation,
                    PeopleKind::WorkTimes,
                    PeopleContext::OwnWorkDay,
                    time.entered_by,
                )
                .await?,
            )
        } else {
            None
        };
        Ok(AwayView {
            id: time.id,
            first: time.first,
            last: time.last,
            first_afternoon_only: time.first_afternoon_only,
            last_morning_only: time.last_morning_only,
            note: if whose.own { time.note.clone() } else { None },
            entered_by,
            version: time.version,
        })
    }

    /// The public holidays that count for a person (their own choice, or the organization's), and
    /// the next of them from the person's today.
    pub(super) async fn holidays_of(
        &self,
        person: bb_core::id::UserId,
    ) -> Result<(HolidayRegion, Vec<PublicHoliday>), AppError> {
        let layers = self.setting_layers_of(Some(person)).await?;
        let holidays = HolidayRegion::chosen(&layers);
        let today = date_of(self.now(), Region::of(&layers).zone());
        Ok((holidays, holidays.next_holidays(today, NEXT_HOLIDAYS)))
    }
}

/// A person's work times, with the away times that touch a stretch of days.
///
/// # Errors
/// `request.invalid` for a stretch that is backwards or longer than [`RANGE_DAYS_MAX`] days, and
/// `resource.not-found` for another person the reader is shown as *Hidden* (`whose.rs`).
pub async fn work_times<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: WorkTimesQuery,
) -> Result<Authorized<WorkTimesView>, AppError> {
    let actor: Actor = ctx.work_times_actor(&input.session_token).await?;
    let whose = ctx.work_times_to_read(&actor, input.person).await?;
    checked_range(input.first, input.last)?;
    let stored = ctx
        .store()
        .usual_days(whose.person)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let usual = stored.map_or_else(UsualDays::start, |stored| stored.days);
    let mut times = ctx
        .store()
        .away_times_of(
            vec![whose.person],
            input.first,
            input.last,
            Some(AWAY_TIMES_MAX as u64 + 1),
        )
        .await
        .map_err(|error| internal::<S>(&error))?;
    let truncated = times.len() > AWAY_TIMES_MAX;
    times.truncate(AWAY_TIMES_MAX);
    let mut away = Vec::with_capacity(times.len());
    for time in &times {
        away.push(ctx.away_view(&whose, time).await?);
    }
    let shares = if whose.own {
        Some(ctx.shares_of_person(&actor, whose.person).await?)
    } else {
        None
    };
    let (holidays, next_holidays) = ctx.holidays_of(whose.person).await?;
    let view = WorkTimesView {
        person: ctx.work_times_person_of(&whose).await?,
        own: whose.own,
        may_change: whose.may_change(),
        usual: usual_of(&usual),
        usual_version: stored.map_or(0, |stored| stored.version),
        holidays,
        next_holidays,
        away,
        truncated,
        shares,
    };
    Ok(whose.evaluation.authorize(view))
}

/// The seven usual days as a list, Monday first.
pub(super) fn usual_of(usual: &UsualDays) -> Vec<bb_core::work_times::UsualDay> {
    usual.days().to_vec()
}
