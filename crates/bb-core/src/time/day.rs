//! The day of a person: which date a moment is in the person's time zone, which moments a date
//! covers there, and the week a date is in by the person's *First day of the week*. A time entry, a
//! work day and the time words of a search all ask these questions; this is the one answer.
//!
//! A date of a zone is not always 24 hours: on the day the clocks go forward it has 23, on the day
//! they go back 25, and where midnight is skipped the day begins at the first moment after it.

use jiff::civil::{Date, Weekday};
use jiff::tz::TimeZone;
use jiff::{Span, Timestamp};

use crate::settings::{FIRST_DAY_SATURDAY, FIRST_DAY_SUNDAY};

/// The days in a week.
pub const DAYS_PER_WEEK: usize = 7;

/// The date a moment is in for a person whose clock is in `zone`.
#[must_use]
pub fn date_of(moment: Timestamp, zone: &TimeZone) -> Date {
    moment.to_zoned(zone.clone()).date()
}

/// The first moment of a date in a zone: midnight, or the first moment after it where the clocks
/// skip midnight. `None` only outside the moments a timestamp can hold.
#[must_use]
pub fn start_of(date: Date, zone: &TimeZone) -> Option<Timestamp> {
    date.to_zoned(zone.clone())
        .ok()
        .map(|start| start.timestamp())
}

/// The moments of one date in one zone: from its first moment, until the first moment of the next
/// date (not included).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DaySpan {
    /// The first moment of the date.
    pub start: Timestamp,
    /// The first moment of the next date.
    pub end: Timestamp,
}

impl DaySpan {
    /// The moments `date` covers in `zone`; `None` for the last date of the calendar, which has no
    /// next date to end on.
    #[must_use]
    pub fn of(date: Date, zone: &TimeZone) -> Option<Self> {
        let start = start_of(date, zone)?;
        let end = start_of(date.tomorrow().ok()?, zone)?;
        Some(Self { start, end })
    }

    /// Whether a moment is in the date.
    #[must_use]
    pub fn holds(&self, moment: Timestamp) -> bool {
        self.start <= moment && moment < self.end
    }
}

/// The day a week starts on: the values of the setting *First day of the week* that a server can
/// know. A person who left it on *device* is counted from Monday, the international rule.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FirstDay {
    /// Monday to Sunday.
    #[default]
    Monday,
    /// Sunday to Saturday.
    Sunday,
    /// Saturday to Friday.
    Saturday,
}

impl FirstDay {
    /// The first day a value of the setting names; any other value is Monday.
    #[must_use]
    pub fn of_choice(value: Option<&str>) -> Self {
        match value {
            Some(FIRST_DAY_SUNDAY) => Self::Sunday,
            Some(FIRST_DAY_SATURDAY) => Self::Saturday,
            _ => Self::Monday,
        }
    }

    const fn weekday(self) -> Weekday {
        match self {
            Self::Monday => Weekday::Monday,
            Self::Sunday => Weekday::Sunday,
            Self::Saturday => Weekday::Saturday,
        }
    }
}

/// A week of a person: seven dates that start on their *First day of the week*.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Week {
    first: Date,
}

impl Week {
    /// The week `date` is in; `None` at the very ends of the calendar, where a week has no room.
    #[must_use]
    pub fn of(date: Date, first_day: FirstDay) -> Option<Self> {
        let back = date.weekday().since(first_day.weekday());
        let first = date.checked_sub(Span::new().days(back)).ok()?;
        // The last day must exist too, so `days` never runs off the calendar.
        last_of(first)?;
        Some(Self { first })
    }

    /// The first date of the week.
    #[must_use]
    pub const fn first(&self) -> Date {
        self.first
    }

    /// The seven dates of the week, in order.
    #[must_use]
    pub fn days(&self) -> Vec<Date> {
        self.first
            .series(Span::new().days(1))
            .take(DAYS_PER_WEEK)
            .collect()
    }
}

/// The last date of a week that starts on `first`.
fn last_of(first: Date) -> Option<Date> {
    let rest = i64::try_from(DAYS_PER_WEEK - 1).ok()?;
    first.checked_add(Span::new().days(rest)).ok()
}

#[cfg(test)]
mod tests;
