//! Working hours (TIME-08, TIME-09, TIME-10, TIME-12, PRIV-09, ADR-0035, decisions 5 to 8): when a
//! person worked, as the law asks every employer to record it. Kept apart from time on issues on
//! purpose (`crate::time`): neither module writes the other's tables or makes a record from the
//! other's, and the comparison of the two reads time spent only through `TimeSpentStore`.
//!
//! - `day.rs`: what makes one work day (pure): the taps, the checks of the times and the limits.
//! - `change.rs`: every change of a day, by a tap or typed afterwards, in one change with its
//!   event, the old times and the reason; the rules checked for the day and the day after it.
//! - `read.rs`: the days of a stretch of dates, one's own or another person's (*Working hours of
//!   others*, as *Totals only* allows).
//! - `history.rs`: the changes of one day, who and why.
//! - `booked.rs`: one's own week compared with the time booked on issues.
//! - `old.rs`: the days older than the keep period, counted and deleted only when an administrator
//!   says so.
//!
//! Who may see another person's days is one rule (`whose.rs`): the person themselves, and a holder
//! of *See working hours of others*; everybody else is answered as if the person did not exist.

mod booked;
mod change;
mod day;
mod history;
mod old;
mod read;
mod store;
mod types;
mod whose;

pub use booked::booked_week;
pub use change::{change_work_day, start_work_day, tap_work_day};
pub use history::work_day_history;
pub use old::{delete_old_work_days, old_work_days};
pub use read::work_days;
pub use store::{OldDays, WorkingHoursStore};
pub use types::*;

/// The longest a work day lasts, from its first start to its last end (ADR-0035, decision 5).
pub const WORK_DAY_HOURS_MAX: i64 = 24;

/// The longest reason of a change, in characters (ADR-0035, decision 7).
pub const REASON_CHARS_MAX: usize = 500;

/// The most breaks of one work day.
pub const BREAKS_PER_DAY_MAX: usize = 20;

/// How many changes of a person's work days are taken in one second at most: a second tap in the
/// same second is refused (`limit.reached`), so a button pressed twice records one moment.
pub const CHANGES_PER_SECOND: i64 = 1;

/// The most changes the history of one day shows: the newest ones, and the answer says that older
/// ones were left out.
pub const HISTORY_CHANGES_MAX: usize = 200;

/// The most people the count of old work days names: the answer says when more have old days.
pub const OLD_PEOPLE_MAX: usize = 500;

/// How far after now a typed moment may lie, in seconds: the clock of a device that is a little
/// ahead. A moment later than that has not happened, so a day cannot record it.
pub const FUTURE_SLACK_SECONDS: i64 = 60;

/// The most dates one read of work days covers: two months, the longest stretch a timesheet shows.
pub const RANGE_DAYS_MAX: i64 = 62;
