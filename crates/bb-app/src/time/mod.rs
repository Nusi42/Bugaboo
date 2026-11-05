//! Time on issues (ADR-0035, decisions 1 to 4): the timer, time entries, the starter fields
//! *Activity* and *Cost center*, the port that the readers outside this module use to read time
//! spent, and the totals of an issue and a project.
//!
//! - `timer.rs`: one timer per person, kept on the server; start, stop, switch, change the start,
//!   throw away.
//! - `entries.rs`: making an entry (by the timer or by hand), changing and deleting one's own.
//! - `read.rs`: one's own entries of some dates, and the entries on an issue.
//! - `starter.rs`: the starter fields of time entries, made at every start.
//! - `limits.rs`: the limits, each with its refusal.
//! - `totals.rs`: the time of an issue with its sub-issues and of a project (TIME-03).
//!
//! Booking needs *Book time* where the issue is (ADR-0035, decision 2); one's own entries are
//! changed and deleted only by their person.

mod entries;
mod limits;
mod read;
pub mod spent;
mod starter;
mod store;
mod timer;
mod types;

pub use entries::{add_time, change_time_entry, delete_time_entry};
pub use limits::{
    LONG_TIMER_HOURS, TIME_ENTRIES_PER_DAY_MAX, TIME_NOTE_CHARS_MAX, TIMER_CHANGES_PER_SECOND,
};
pub use read::{issue_time_entries, my_time_entries, time_entry_defaults};
pub use starter::{CreateStarterTimeFields, create_starter_time_fields};
pub use store::{EntryCursor, TimeStore};
pub use timer::{
    change_start_time, start_timer, stop_timer, switch_timer, throw_away_timer, timer,
};
pub use types::*;
pub mod totals;
