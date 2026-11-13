//! The usual work days of a person and the work hours of each (PLAN-11, PLAN-15, ADR-0030).

use jiff::civil::Weekday;
use serde::Serialize;

/// The most work minutes of one day: all 24 hours.
pub const WORK_MINUTES_MAX: u16 = 24 * 60;

/// The minutes a person works on a usual work day at the start: 8 hours.
const START_MINUTES: u16 = 8 * 60;

/// One weekday: whether the person usually works on it, and the hours they work then. The hours
/// are kept when the day is not ticked, so ticking it again brings them back.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct UsualDay {
    /// Whether the person usually works on this weekday.
    pub works: bool,
    /// The work hours of the day, in minutes, at most [`WORK_MINUTES_MAX`].
    pub minutes: u16,
}

/// The seven weekdays of a person, Monday first. At the start Monday to Friday, 8 hours each.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct UsualDays([UsualDay; 7]);

impl UsualDays {
    /// The days a person has until they say otherwise.
    #[must_use]
    pub const fn start() -> Self {
        const WORKS: UsualDay = UsualDay {
            works: true,
            minutes: START_MINUTES,
        };
        const FREE: UsualDay = UsualDay {
            works: false,
            minutes: START_MINUTES,
        };
        Self([WORKS, WORKS, WORKS, WORKS, WORKS, FREE, FREE])
    }

    /// The seven days, Monday first; `None` when a day has more minutes than a day holds.
    #[must_use]
    pub fn new(days: [UsualDay; 7]) -> Option<Self> {
        days.iter()
            .all(|day| day.minutes <= WORK_MINUTES_MAX)
            .then_some(Self(days))
    }

    /// The seven days, Monday first.
    #[must_use]
    pub const fn days(&self) -> &[UsualDay; 7] {
        &self.0
    }

    /// One weekday.
    #[must_use]
    pub fn day(&self, weekday: Weekday) -> UsualDay {
        self.0[usize::from(weekday.to_monday_zero_offset().unsigned_abs())]
    }

    /// The minutes a person works on this weekday: nothing on a day they do not usually work.
    #[must_use]
    pub fn minutes_on(&self, weekday: Weekday) -> u32 {
        let day = self.day(weekday);
        if day.works { u32::from(day.minutes) } else { 0 }
    }
}

/// The usual work days of a person as they are stored, with the version that counts up with every
/// change, for the check that nobody changed them meanwhile (ADR-0019). A person who never changed
/// them has none stored: version 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoredUsualDays {
    /// The seven days.
    pub days: UsualDays,
    /// Counts up with every change.
    pub version: i64,
}

impl Default for UsualDays {
    fn default() -> Self {
        Self::start()
    }
}
