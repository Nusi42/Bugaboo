//! A stretch of whole days, both ends included.

use jiff::ToSpan as _;
use jiff::civil::Date;
use serde::Serialize;

/// The days from `first` to `last`, both included (a bar covers whole days, ADR-0043, decision 3).
/// `first` is never after `last`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct Days {
    /// The first day.
    pub first: Date,
    /// The last day.
    pub last: Date,
}

impl Days {
    /// The days from `first` to `last`; `None` when `last` is before `first`.
    #[must_use]
    pub fn new(first: Date, last: Date) -> Option<Self> {
        (first <= last).then_some(Self { first, last })
    }

    /// One day.
    #[must_use]
    pub const fn one(date: Date) -> Self {
        Self {
            first: date,
            last: date,
        }
    }

    /// Whether `date` is one of the days.
    #[must_use]
    pub fn contains(self, date: Date) -> bool {
        self.first <= date && date <= self.last
    }

    /// Whether the two stretches share a day.
    #[must_use]
    pub fn overlaps(self, other: Self) -> bool {
        self.first <= other.last && other.first <= self.last
    }

    /// The same stretch, `by` days later (earlier for a negative `by`); `None` past the ends of the
    /// calendar.
    #[must_use]
    pub fn shifted(self, by: i64) -> Option<Self> {
        Some(Self {
            first: self.first.checked_add(by.days()).ok()?,
            last: self.last.checked_add(by.days()).ok()?,
        })
    }

    /// The number of days from `from` to `to`: positive when `to` is later.
    #[must_use]
    pub fn between(from: Date, to: Date) -> i64 {
        i64::from((to - from).get_days())
    }
}
