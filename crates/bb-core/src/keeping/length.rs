//! The length of a keep period, and the two ways it is used: the moment before which a record is
//! past it, and the dates a record is kept through.

use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Span, Timestamp};

use crate::time::HOURS_PER_DAY;

words! {
    /// What a keep period counts in.
    pub enum LengthUnit {
        /// Hours of 60 minutes.
        Hours => "hours",
        /// Days of 24 hours.
        Days => "days",
        /// Calendar years.
        Years => "years",
    }
    messages name = "keep-length."
}

/// How long a kind of data is kept: a count and what it counts in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Length {
    /// How many.
    pub count: i64,
    /// Of what.
    pub unit: LengthUnit,
}

impl Length {
    /// A number of hours.
    #[must_use]
    pub const fn hours(count: i64) -> Self {
        Self {
            count,
            unit: LengthUnit::Hours,
        }
    }

    /// A number of days.
    #[must_use]
    pub const fn days(count: i64) -> Self {
        Self {
            count,
            unit: LengthUnit::Days,
        }
    }

    /// A number of years.
    #[must_use]
    pub const fn years(count: i64) -> Self {
        Self {
            count,
            unit: LengthUnit::Years,
        }
    }

    /// The moment before which a record is past the period, at `now`. Hours and days are counted
    /// as lengths of time (a day is 24 hours, `crate::time::days`); years on the calendar of
    /// `zone`, so a year is a year in a leap year too. `None` only at the start of time.
    #[must_use]
    pub fn before(self, now: Timestamp, zone: &TimeZone) -> Option<Timestamp> {
        match self.unit {
            LengthUnit::Hours => now.checked_sub(SignedDuration::from_hours(self.count)).ok(),
            LengthUnit::Days => now.checked_sub(crate::time::days(self.count)).ok(),
            LengthUnit::Years => now
                .to_zoned(zone.clone())
                .checked_sub(Span::new().try_years(self.count).ok()?)
                .ok()
                .map(|zoned| zoned.timestamp()),
        }
    }

    /// The period on the calendar and one day more, for a record that has a date (see
    /// `KeepRule::kept_from`): hours are rounded up to whole days, so a record is never deleted
    /// early.
    fn on_the_calendar(self) -> Option<Span> {
        let hours_per_day = i64::from(HOURS_PER_DAY);
        match self.unit {
            LengthUnit::Hours => {
                let days = self.count.div_euclid(hours_per_day)
                    + i64::from(self.count.rem_euclid(hours_per_day) != 0);
                Span::new().try_days(days.checked_add(1)?).ok()
            }
            LengthUnit::Days => Span::new().try_days(self.count.checked_add(1)?).ok(),
            LengthUnit::Years => Span::new().try_years(self.count).ok()?.try_days(1).ok(),
        }
    }

    /// The first date that is still inside the period on `today`: an earlier date is past it.
    #[must_use]
    pub fn first_kept(self, today: Date) -> Option<Date> {
        today.checked_sub(self.on_the_calendar()?).ok()
    }

    /// The last day a record dated `date` is kept: on the day after it, [`Self::first_kept`] is
    /// later than `date`.
    #[must_use]
    pub fn last_kept(self, date: Date) -> Option<Date> {
        date.checked_add(self.on_the_calendar()?).ok()
    }
}
