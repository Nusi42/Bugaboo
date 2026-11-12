//! Work times (PLAN-11, PLAN-15, PRIV-09, ADR-0030, decision 2 and amendment M8): when and how much
//! each person works. Usual work days and work hours, a share in each project, away times and the
//! public holidays that count for them. The data is guarded as employee data: a person reads and
//! enters their own, somebody with *Enter work times for others* enters usual work days, work hours
//! and away times for others and never reads or writes the note of an away time, and everybody
//! else sees an away time only as *Away* with its days, as *Who sees people* allows for the kind
//! *Work times*.
//!
//! - `whose.rs`: whose work times a request is about, and what the reader may do with them (one
//!   rule).
//! - `read.rs`: a person's work times, as the reader may see them.
//! - `usual.rs`: changing the usual work days and work hours.
//! - `away.rs`: entering, changing and removing away times.
//! - `shares.rs`: the share in a project, the sum of a person's shares and what a project lead sees.
//! - `minutes.rs`: the port of the plan: the minutes each person can work on each day in a project.
//!   The plan reads work times only through it, so the sum of minutes is worked out once, in
//!   `bb_core::work_times::minutes_on`.
//! - `old.rs`: the daily job that deletes away times one year after their last day.
//! - `leave.rs`: all work times of a person who leaves the organization.
//!
//! Nothing of work times reaches a search, an export, a log or an AI tool (COMP-10, PRIV-09).

mod away;
mod leave;
mod minutes;
mod old;
mod read;
mod shares;
mod store;
mod types;
mod usual;
mod whose;

pub use away::{change_away_time, enter_away_time, remove_away_time};
pub use leave::{DeleteWorkTimesOfPerson, delete_work_times_in, delete_work_times_of_person};
pub use minutes::{DayMinutes, PersonMinutes, WorkMinutes, WorkMinutesQuery};
pub use old::delete_old_away_times;
pub use read::work_times;
pub use shares::{project_shares, set_share};
pub use store::WorkTimesStore;
pub use types::*;
pub use usual::change_usual_days;

/// The most people one read of minutes for the plan covers: a page of the timeline holds fewer.
pub const PLAN_PEOPLE_MAX: usize = 200;

/// The longest stretch of days one read of work times or minutes covers: a year and a day, so a
/// year-long away time fits in a read.
pub const RANGE_DAYS_MAX: i64 = 366;

/// How many of the coming public holidays a read of work times names (UX §4.13: "Next: …").
pub const NEXT_HOLIDAYS: usize = 3;

/// The most away times of one person that a read lists.
pub const AWAY_TIMES_MAX: usize = 500;
