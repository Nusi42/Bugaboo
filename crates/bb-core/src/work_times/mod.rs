//! Work times (PLAN-11, PLAN-15, ADR-0030, decision 2 and amendment M8): when and how much a person
//! works. The pure rules, worked out once: a person's usual work days with the hours of each
//! ([`UsualDays`]), the share of the hours that belongs to one project ([`Share`]), the away times
//! ([`record::AwayTime`], whole days and a half day at either end) and the one function that says
//! how many minutes a person can work on a day ([`minutes_on`]). Pure: no clock and no I/O; the
//! public holidays come from [`crate::holidays`], and the caller says whether a date is one.
//!
//! No reason is stored for an away time: Bugaboo does not know whether it is a holiday, a course or
//! an illness (GDPR Art. 9). The stored records are in [`record`].

mod away;
pub mod record;
mod share;
mod usual;

pub use away::{AWAY_TIME_DAYS_MAX, AwayProblem, Halves, away_cutoff};
pub use share::{Share, over_100, sum_of_shares};
pub use usual::{StoredUsualDays, UsualDay, UsualDays, WORK_MINUTES_MAX};

use jiff::civil::Date;

use self::record::AwayTime;
use crate::holidays::Holiday;

/// The most characters of the note of an away time.
pub const NOTE_CHARS_MAX: usize = 500;

/// What one day holds for a person in a project: what the plan of the timeline is worked out from
/// (`crate::plan`), as the reader may know it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DayMinutes {
    /// The day.
    pub date: Date,
    /// The minutes the person can work on it in the project ([`minutes_on`]).
    pub minutes: u32,
    /// Which halves of the day the person is away; none for a reader who may not know the person's
    /// work times.
    pub away: Halves,
    /// The public holiday that counts for the person on this day; none for such a reader.
    pub holiday: Option<Holiday>,
}

/// How many minutes a person can work on `date` in a project: the work hours of the weekday, times
/// the share, and half of that when an away time covers one half of the day. Nothing on a day the
/// person does not usually work, on a public holiday (`holiday`) and when the away times cover both
/// halves. Whole minutes: a half day gives the lower half, so the answer never promises more than
/// the day holds.
#[must_use]
pub fn minutes_on(
    usual: &UsualDays,
    share: Share,
    date: Date,
    away: &[AwayTime],
    holiday: bool,
) -> u32 {
    if holiday {
        return 0;
    }
    let shared = share.of(usual.minutes_on(date.weekday()));
    match away
        .iter()
        .fold(Halves::NONE, |halves, time| halves.and(time.away_on(date)))
        .count()
    {
        0 => shared,
        1 => shared / 2,
        _ => 0,
    }
}

#[cfg(test)]
mod tests;
