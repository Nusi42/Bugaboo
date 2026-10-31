//! Working hours as the law counts them (TIME-08, TIME-09, ADR-0035, decisions 5 and 6), worked out
//! once: the totals of a work day ([`Totals`]), the working-time rules made from the settings of the
//! registry ([`Rules`]) and the one function that checks a day against them ([`warnings`]). Pure:
//! no clock and no I/O, the caller gives the moments and, for a day that runs, the moment it is now.
//!
//! A day is its periods of work and its breaks, each a [`Span`] of two moments in UTC. Everything
//! here counts the time between moments, never the hours on a clock, so a day across midnight, a
//! day on which the clocks change and two days in different zones are counted alike; which date a
//! day belongs to is the person's day of its first start (`crate::time::day`). The stored records
//! are in [`record`].

mod check;
pub mod record;
mod rules;
mod totals;
mod warning;

pub use check::warnings;
pub use rules::{
    BreakRule, GERMANY_KEEP_YEARS_LEAST, NO_RULES_LONGEST_DAY, Preset, Rules, keep_years,
    keep_years_allowed, longest_day, preset_of, totals_only,
};
pub use totals::Totals;
pub use warning::Warning;

use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use serde::{Deserialize, Serialize};

use crate::time::day::date_of;

/// One stretch of a work day: a period of work, or a break. It has no end while it runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    /// When it started.
    pub start: Timestamp,
    /// When it ended; `None` while it runs.
    pub end: Option<Timestamp>,
}

impl Span {
    /// A stretch that has ended.
    #[must_use]
    pub const fn closed(start: Timestamp, end: Timestamp) -> Self {
        Self {
            start,
            end: Some(end),
        }
    }

    /// A stretch that runs.
    #[must_use]
    pub const fn open(start: Timestamp) -> Self {
        Self { start, end: None }
    }
}

/// The moments of one work day: its periods of work and its breaks. A break lies inside a period;
/// the time between two periods counts as a break too (ADR-0035, decision 5).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Times {
    /// The periods of work, in the order they started.
    pub periods: Vec<Span>,
    /// The breaks, in the order they started.
    pub breaks: Vec<Span>,
}

impl Times {
    /// The first start of the day: the start of its first period.
    #[must_use]
    pub fn first_start(&self) -> Option<Timestamp> {
        self.periods.iter().map(|period| period.start).min()
    }

    /// The last end of the day; `None` while a period runs or before the day has a period.
    #[must_use]
    pub fn last_end(&self) -> Option<Timestamp> {
        if self.periods.iter().any(|period| period.end.is_none()) {
            return None;
        }
        self.periods.iter().filter_map(|period| period.end).max()
    }

    /// Whether a period or a break of the day runs.
    #[must_use]
    pub fn runs(&self) -> bool {
        self.periods
            .iter()
            .chain(&self.breaks)
            .any(|span| span.end.is_none())
    }
}

/// Whether a day of the person is past: from the next day on, in the zone of the day (ADR-0035,
/// decision 7). A change of a past day asks for a reason.
#[must_use]
pub fn is_past(date: Date, zone: &TimeZone, now: Timestamp) -> bool {
    date_of(now, zone) > date
}

#[cfg(test)]
mod tests;
