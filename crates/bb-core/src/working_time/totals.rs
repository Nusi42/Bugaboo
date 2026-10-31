//! The totals of a work day and of a week: the one function every answer about working hours uses
//! (TIME-08). A day's work is its periods without its breaks; its *Breaks* are its breaks and the
//! gaps between its periods (ADR-0035, decision 5).

use jiff::Timestamp;
use serde::Serialize;

use super::{Span, Times};
use crate::time::Minutes;

/// What a day or a week adds up to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Totals {
    /// The time worked, in whole minutes.
    pub worked: Minutes,
    /// The breaks, the gaps between periods included, in whole minutes.
    pub breaks: Minutes,
}

/// One piece of work: from a start to an end, with no break in it.
pub(super) type Piece = (Timestamp, Timestamp);

/// The longest length there is, where a sum would run over.
const LONGEST: Minutes = Minutes::new(u32::MAX);

/// The end of a stretch as it is counted: its end, or `until` for one that runs (never before its
/// start); with no `until` a running stretch counts as nothing yet.
fn end_of(span: &Span, until: Option<Timestamp>) -> Timestamp {
    span.end
        .or(until)
        .map_or(span.start, |end| end.max(span.start))
}

/// The pieces of work of a day in order: each period without the breaks that lie in it.
pub(super) fn pieces(times: &Times, until: Option<Timestamp>) -> Vec<Piece> {
    let mut breaks: Vec<Piece> = times
        .breaks
        .iter()
        .map(|span| (span.start, end_of(span, until)))
        .collect();
    breaks.sort_unstable();
    let mut pieces = Vec::new();
    for period in &times.periods {
        let (mut from, to) = (period.start, end_of(period, until));
        for (start, end) in &breaks {
            if *end <= from || *start >= to {
                continue;
            }
            if *start > from {
                pieces.push((from, *start));
            }
            from = from.max(*end);
        }
        if to > from {
            pieces.push((from, to));
        }
    }
    pieces.sort_unstable();
    pieces
}

/// The gaps between pieces that follow each other: the breaks of the day as the rules see them.
pub(super) fn gaps(pieces: &[Piece]) -> impl Iterator<Item = Minutes> + '_ {
    pieces
        .windows(2)
        .map(|pair| minutes_between(pair[0].1, pair[1].0))
}

/// The whole minutes from `start` to `end`; nothing when `end` is not later.
pub(super) fn minutes_between(start: Timestamp, end: Timestamp) -> Minutes {
    let seconds = end.duration_since(start).as_secs().max(0);
    Minutes::from_seconds(seconds).unwrap_or(LONGEST)
}

/// The sum of lengths, at most the longest a length can be.
pub(super) fn sum(lengths: impl Iterator<Item = Minutes>) -> Minutes {
    lengths.fold(Minutes::default(), |total, length| {
        total.checked_add(length).unwrap_or(LONGEST)
    })
}

/// The whole minutes of the pieces together.
pub(super) fn worked(pieces: &[Piece]) -> Minutes {
    let seconds: i64 = pieces
        .iter()
        .map(|(start, end)| end.duration_since(*start).as_secs())
        .sum();
    Minutes::from_seconds(seconds).unwrap_or(LONGEST)
}

impl Totals {
    /// The totals of a day; a period or a break that runs counts until `now`. The breaks are the
    /// time of the periods that is not work, and the time between the periods.
    #[must_use]
    pub fn of(times: &Times, now: Timestamp) -> Self {
        Self::until(times, Some(now))
    }

    /// The totals of what has ended: a period or a break that runs counts as nothing yet. What a
    /// total that others read says, so it never tells whether the person works right now.
    #[must_use]
    pub fn ended(times: &Times) -> Self {
        Self::until(times, None)
    }

    /// The totals with what runs counted until `until`, or as nothing without it.
    fn until(times: &Times, until: Option<Timestamp>) -> Self {
        let worked = worked(&pieces(times, until));
        let mut periods: Vec<Piece> = times
            .periods
            .iter()
            .map(|period| (period.start, end_of(period, until)))
            .collect();
        periods.sort_unstable();
        let length = sum(periods
            .iter()
            .map(|(start, end)| minutes_between(*start, *end)));
        let inside = length.checked_sub(worked).unwrap_or_default();
        Self {
            worked,
            breaks: sum(std::iter::once(inside).chain(gaps(&periods))),
        }
    }

    /// The totals of two stretches of days together.
    #[must_use]
    pub fn plus(self, other: Self) -> Self {
        Self {
            worked: sum([self.worked, other.worked].into_iter()),
            breaks: sum([self.breaks, other.breaks].into_iter()),
        }
    }
}

impl std::iter::Sum for Totals {
    fn sum<I: Iterator<Item = Self>>(days: I) -> Self {
        days.fold(Self::default(), Self::plus)
    }
}
