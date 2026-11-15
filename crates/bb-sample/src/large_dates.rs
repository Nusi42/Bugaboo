//! The dates of the issues of the biggest project in the large dataset (M8, PERF-01): the timeline
//! of that project shows a page of rows, so a share of its issues has a start date, a due date and
//! an estimate. They come from a stream of their own, so the dates change nothing else about the
//! dataset.

use bb_core::time::Minutes;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{Span, Timestamp};

use crate::large_ids::rng_for;
use crate::large_state::Bank;

/// The kind of random numbers the dates of an issue draw.
const DATED: u64 = 3;
/// The minutes of a work day that an estimate counts in.
const WORK_DAY_MINUTES: u32 = 480;

/// The start date, the due date and the estimate of issue number `index` in the place `place_at`,
/// or none when it is not one of the dated issues of the biggest project (the first of the sample,
/// with its sub-projects). The dates start within the days after `end`, in seconds since 1970.
pub(crate) fn planned_dates(
    bank: &Bank,
    seed: u64,
    end: i64,
    index: u32,
    place_at: usize,
) -> Option<(Date, Date, Minutes)> {
    let plan = &bank.plan;
    let big = bank.names.projects.first()?;
    if bank.families.get(place_at)? != &big.short_name {
        return None;
    }
    let mut random = rng_for(seed, DATED, u64::from(index));
    if !random.chance(plan.dated_percent) {
        return None;
    }
    let first = Timestamp::from_second(end)
        .ok()?
        .to_zoned(TimeZone::UTC)
        .date();
    let ahead = i64::try_from(random.below(plan.dated_days as usize)).ok()?;
    let length = i64::try_from(random.below(plan.dated_longest_days as usize)).ok()?;
    let start = first.checked_add(Span::new().days(ahead)).ok()?;
    let due = start.checked_add(Span::new().days(length)).ok()?;
    let work_days = u32::try_from(length).ok()?.saturating_add(1);
    Some((start, due, Minutes::new(work_days * WORK_DAY_MINUTES)))
}
