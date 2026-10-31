//! What makes one work day (ADR-0035, decision 5), without storage: what a tap does to the day
//! that runs, and the checks every change of a day passes. A day is never ended by Bugaboo: only
//! the person's tap or answer ends it.

use bb_core::ErrorCode;
use bb_core::time::day::date_of;
use bb_core::working_time::{Span, Times};
use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};

use super::{BREAKS_PER_DAY_MAX, WORK_DAY_HOURS_MAX};

/// A tap on the day that runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Tap {
    /// *Start break*.
    StartBreak,
    /// *End break*.
    EndBreak,
    /// *End work day*: the break that runs ends too.
    End,
}

/// The day after *Start work day* at `now`: a new day, or a new period of the day of the same
/// date that was ended already.
pub(super) fn started(times: Option<&Times>, now: Timestamp) -> Times {
    let mut times = times.cloned().unwrap_or_default();
    times.periods.push(Span::open(now));
    times
}

/// The day after `tap` at `now`.
///
/// # Errors
/// `working-hours.day-not-running` when no period runs, `working-hours.break-running` for a second
/// break, `working-hours.no-break-running` to end a break that does not run.
pub(super) fn tapped(times: &Times, tap: Tap, now: Timestamp) -> Result<Times, ErrorCode> {
    let mut times = times.clone();
    let running = times
        .periods
        .iter()
        .position(|period| period.end.is_none())
        .ok_or(ErrorCode::WorkDayNotRunning)?;
    let on_break = times.breaks.iter().position(|span| span.end.is_none());
    match (tap, on_break) {
        (Tap::StartBreak, Some(_)) => return Err(ErrorCode::BreakRunning),
        (Tap::StartBreak, None) => times.breaks.push(Span::open(now)),
        (Tap::EndBreak, None) => return Err(ErrorCode::NoBreakRunning),
        (Tap::EndBreak, Some(at)) => times.breaks[at].end = Some(now),
        (Tap::End, on_break) => {
            if let Some(at) = on_break {
                times.breaks[at].end = Some(now);
            }
            times.periods[running].end = Some(now);
        }
    }
    Ok(times)
}

/// Whether the spans follow each other without overlapping, each ends after it starts, and only the
/// last one runs.
fn in_a_row(spans: &[Span]) -> bool {
    let last = spans.len().saturating_sub(1);
    spans.iter().enumerate().all(|(at, span)| match span.end {
        Some(end) => end > span.start && spans.get(at + 1).is_none_or(|next| end <= next.start),
        None => at == last,
    })
}

/// Whether a break lies inside a period: it starts in it and ends in it, and one that runs lies in
/// the period that runs.
fn inside(spell: &Span, period: &Span) -> bool {
    let starts_in = period.start <= spell.start && period.end.is_none_or(|end| spell.start < end);
    let ends_in = match (spell.end, period.end) {
        (Some(end), Some(period_end)) => end <= period_end,
        (_, None) => true,
        (None, Some(_)) => false,
    };
    starts_in && ends_in
}

/// The times of a day as they are kept: periods and breaks in the order they started.
pub(super) fn in_order(mut times: Times) -> Times {
    times.periods.sort_by_key(|span| span.start);
    times.breaks.sort_by_key(|span| span.start);
    times
}

/// Checks that `times` (in order) are one work day of `date` in `zone`, between the day before
/// and the day after it.
///
/// # Errors
/// `limit.reached` for more than [`BREAKS_PER_DAY_MAX`] breaks; `working-hours.not-a-work-day`
/// when a day has no period, its periods or breaks overlap, a break lies outside the work, it
/// starts on another date in its zone, it lasts longer than [`WORK_DAY_HOURS_MAX`] hours, or it
/// overlaps the day before or after it.
pub(super) fn check(
    times: &Times,
    date: Date,
    zone: &TimeZone,
    before: Option<&Times>,
    after: Option<&Times>,
) -> Result<(), ErrorCode> {
    if times.breaks.len() > BREAKS_PER_DAY_MAX {
        return Err(ErrorCode::LimitReached);
    }
    let first_start = times.first_start().ok_or(ErrorCode::NotAWorkDay)?;
    let shaped = in_a_row(&times.periods)
        && in_a_row(&times.breaks)
        && times
            .breaks
            .iter()
            .all(|spell| times.periods.iter().any(|period| inside(spell, period)));
    let on_its_date = date_of(first_start, zone) == date;
    let longest = SignedDuration::from_hours(WORK_DAY_HOURS_MAX);
    let short_enough = times
        .last_end()
        .is_none_or(|end| end.duration_since(first_start) <= longest);
    let after_before = before
        .and_then(Times::last_end)
        .is_none_or(|end| end <= first_start);
    let before_after = after
        .and_then(Times::first_start)
        .is_none_or(|next| times.last_end().is_some_and(|end| end <= next));
    if shaped && on_its_date && short_enough && after_before && before_after {
        Ok(())
    } else {
        Err(ErrorCode::NotAWorkDay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> Timestamp {
        text.parse().unwrap_or(Timestamp::UNIX_EPOCH)
    }

    fn day(periods: &[(&str, Option<&str>)], breaks: &[(&str, Option<&str>)]) -> Times {
        let span = |(start, end): &(&str, Option<&str>)| Span {
            start: at(start),
            end: end.map(at),
        };
        Times {
            periods: periods.iter().map(span).collect(),
            breaks: breaks.iter().map(span).collect(),
        }
    }

    const MONDAY: (i16, i8, i8) = (2026, 10, 5);

    fn monday() -> Date {
        Date::new(MONDAY.0, MONDAY.1, MONDAY.2).unwrap_or_default()
    }

    #[test]
    fn time_08_a_tap_records_the_moment_and_follows_the_day() {
        let started = started(None, at("2026-10-05T07:00:00Z"));
        assert_eq!(
            tapped(&started, Tap::EndBreak, at("2026-10-05T08:00:00Z")),
            Err(ErrorCode::NoBreakRunning)
        );
        let on_break = tapped(&started, Tap::StartBreak, at("2026-10-05T11:00:00Z"));
        assert!(on_break.is_ok());
        let on_break = on_break.unwrap_or_default();
        assert_eq!(
            tapped(&on_break, Tap::StartBreak, at("2026-10-05T11:01:00Z")),
            Err(ErrorCode::BreakRunning)
        );
        // *End work day* ends the break that runs too.
        let ended = tapped(&on_break, Tap::End, at("2026-10-05T11:30:00Z")).unwrap_or_default();
        assert_eq!(ended.last_end(), Some(at("2026-10-05T11:30:00Z")));
        assert!(!ended.runs());
        assert_eq!(
            tapped(&ended, Tap::StartBreak, at("2026-10-05T12:00:00Z")),
            Err(ErrorCode::WorkDayNotRunning)
        );
        // A second start reopens the day with a new period.
        let reopened = super::started(Some(&ended), at("2026-10-05T13:00:00Z"));
        assert_eq!(reopened.periods.len(), 2);
        assert!(reopened.runs());
    }

    #[test]
    fn time_08_the_times_of_a_day_hold_together() {
        let utc = TimeZone::UTC;
        let good = day(
            &[("2026-10-05T07:00:00Z", Some("2026-10-05T16:00:00Z"))],
            &[("2026-10-05T12:00:00Z", Some("2026-10-05T12:30:00Z"))],
        );
        assert_eq!(check(&good, monday(), &utc, None, None), Ok(()));
        // A break outside the work.
        let outside = day(
            &[("2026-10-05T07:00:00Z", Some("2026-10-05T16:00:00Z"))],
            &[("2026-10-05T16:30:00Z", Some("2026-10-05T17:00:00Z"))],
        );
        assert_eq!(
            check(&outside, monday(), &utc, None, None),
            Err(ErrorCode::NotAWorkDay)
        );
        // Two periods that overlap.
        let overlap = day(
            &[
                ("2026-10-05T07:00:00Z", Some("2026-10-05T12:00:00Z")),
                ("2026-10-05T11:00:00Z", Some("2026-10-05T16:00:00Z")),
            ],
            &[],
        );
        assert_eq!(
            check(&overlap, monday(), &utc, None, None),
            Err(ErrorCode::NotAWorkDay)
        );
        // Longer than 24 hours.
        let long = day(
            &[("2026-10-05T07:00:00Z", Some("2026-10-06T07:01:00Z"))],
            &[],
        );
        assert_eq!(
            check(&long, monday(), &utc, None, None),
            Err(ErrorCode::NotAWorkDay)
        );
        // On another date in its zone.
        assert_eq!(
            check(
                &good,
                monday().tomorrow().unwrap_or_default(),
                &utc,
                None,
                None
            ),
            Err(ErrorCode::NotAWorkDay)
        );
        // Over the end of the day before, or the start of the day after.
        let before = day(
            &[("2026-10-04T22:00:00Z", Some("2026-10-05T08:00:00Z"))],
            &[],
        );
        assert_eq!(
            check(&good, monday(), &utc, Some(&before), None),
            Err(ErrorCode::NotAWorkDay)
        );
        let after = day(&[("2026-10-05T15:00:00Z", None)], &[]);
        assert_eq!(
            check(&good, monday(), &utc, None, Some(&after)),
            Err(ErrorCode::NotAWorkDay)
        );
        // No period at all.
        assert_eq!(
            check(&Times::default(), monday(), &utc, None, None),
            Err(ErrorCode::NotAWorkDay)
        );
    }

    #[test]
    fn time_08_a_day_has_at_most_twenty_breaks() {
        let utc = TimeZone::UTC;
        let start = at("2026-10-05T07:00:00Z");
        let mut times = Times {
            periods: vec![Span::closed(start, at("2026-10-05T19:00:00Z"))],
            breaks: Vec::new(),
        };
        for minute in 0..=i64::try_from(BREAKS_PER_DAY_MAX).unwrap_or(0) {
            let from = start + SignedDuration::from_mins(10 + minute * 20);
            times
                .breaks
                .push(Span::closed(from, from + SignedDuration::from_mins(5)));
        }
        assert_eq!(
            check(&times, monday(), &utc, None, None),
            Err(ErrorCode::LimitReached)
        );
        times.breaks.pop();
        assert_eq!(check(&times, monday(), &utc, None, None), Ok(()));
    }
}
