//! The away times of a person: whole days and a half day at either end (PLAN-11, ADR-0030,
//! decision 2). Bugaboo stores no reason.

use jiff::ToSpan as _;
use jiff::civil::Date;

use super::record::AwayTime;

/// The longest an away time is, from its first to its last day: a year and a day, so a year-long
/// leave fits in one entry.
pub const AWAY_TIME_DAYS_MAX: i32 = 366;

/// How long an away time is kept after its last day, in years (ADR-0017, amendment M8).
const AWAY_TIME_KEPT_FOR_YEARS: i64 = 1;

/// What is wrong with the days of an away time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AwayProblem {
    /// The last day is before the first.
    LastBeforeFirst,
    /// The time is longer than [`AWAY_TIME_DAYS_MAX`] days.
    TooLong,
    /// A time of one day that is both *Afternoon only* and *Morning only* has no time left.
    NoTimeLeft,
}

/// Which halves of one day a person is away: the morning, the afternoon, both or neither.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Halves {
    /// Away in the morning.
    pub morning: bool,
    /// Away in the afternoon.
    pub afternoon: bool,
}

impl Halves {
    /// Not away.
    pub const NONE: Self = Self {
        morning: false,
        afternoon: false,
    };

    /// Away the whole day.
    pub const BOTH: Self = Self {
        morning: true,
        afternoon: true,
    };

    /// Away in every half where either of the two is.
    #[must_use]
    pub const fn and(self, other: Self) -> Self {
        Self {
            morning: self.morning || other.morning,
            afternoon: self.afternoon || other.afternoon,
        }
    }

    /// Away in every half where both are.
    const fn both(self, other: Self) -> Self {
        Self {
            morning: self.morning && other.morning,
            afternoon: self.afternoon && other.afternoon,
        }
    }

    /// How many halves, 0 to 2.
    #[must_use]
    pub const fn count(self) -> u8 {
        self.morning as u8 + self.afternoon as u8
    }
}

impl AwayTime {
    /// Checks the days of an away time before it is stored.
    ///
    /// # Errors
    /// What is wrong with them.
    pub fn check(
        first: Date,
        last: Date,
        first_afternoon_only: bool,
        last_morning_only: bool,
    ) -> Result<(), AwayProblem> {
        if last < first {
            return Err(AwayProblem::LastBeforeFirst);
        }
        if (last - first).get_days() >= AWAY_TIME_DAYS_MAX {
            return Err(AwayProblem::TooLong);
        }
        if first == last && first_afternoon_only && last_morning_only {
            return Err(AwayProblem::NoTimeLeft);
        }
        Ok(())
    }

    /// The halves of `date` on which the person is away because of this time.
    #[must_use]
    pub fn away_on(&self, date: Date) -> Halves {
        if date < self.first || self.last < date {
            return Halves::NONE;
        }
        let from_first = if date == self.first && self.first_afternoon_only {
            Halves {
                morning: false,
                afternoon: true,
            }
        } else {
            Halves::BOTH
        };
        let to_last = if date == self.last && self.last_morning_only {
            Halves {
                morning: true,
                afternoon: false,
            }
        } else {
            Halves::BOTH
        };
        from_first.both(to_last)
    }
}

/// The date before which an away time is old: its last day is before it. An away time is kept for
/// one year after its last day, so on the day a year has passed it is still there, and the daily
/// job deletes it the day after. `None` only at the start of the calendar.
#[must_use]
pub fn away_cutoff(today: Date) -> Option<Date> {
    today.checked_sub(AWAY_TIME_KEPT_FOR_YEARS.years()).ok()
}
