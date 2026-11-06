//! The port of working hours: what the use cases of work days need from storage (TIME-08,
//! ADR-0035, decisions 5 to 8).
//!
//! Like the other ports, reads take `&self` and **writes** are associated functions that run inside
//! `record_change`, so a change and its domain event are committed together (ARCH-EVENTS). Every
//! change of a person's days takes the lock of that person's days first, so two changes at the
//! same moment are judged one after the other on what is stored.

use std::future::Future;

use bb_core::DomainEvent;
use bb_core::id::{UserId, WorkDayId};
use bb_core::working_time::Warning;
use bb_core::working_time::record::StoredDay;
use jiff::Timestamp;
use jiff::civil::Date;

use crate::settings::SettingsStore;
use crate::store_macros::{queries, reads, writes};
use crate::time::spent::TimeSpentStore;

/// How many work days of one person are older than a date.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OldDays {
    /// The person.
    pub person: UserId,
    /// How many of their days.
    pub count: u64,
}

/// What the use cases of working hours need from storage. The time booked on issues is read only
/// through [`TimeSpentStore`], never from the tables of time entries (ADR-0035, decision 4).
pub trait WorkingHoursStore: SettingsStore + TimeSpentStore + 'static {
    queries! {
        /// The work day of a person on a date, with its periods and breaks.
        fn work_day(person: UserId, date: Date) -> Option<StoredDay> = work_day_in;
        /// The person's last work day before `date`.
        fn work_day_before(person: UserId, date: Date) -> Option<StoredDay> = work_day_before_in;
        /// The person's first work day after `date`.
        fn work_day_after(person: UserId, date: Date) -> Option<StoredDay> = work_day_after_in;
        /// The person's last work day, if a period of it runs: a day that runs, or one that was
        /// left open.
        fn open_work_day(person: UserId) -> Option<StoredDay> = open_work_day_in;
        /// When a work day of the person was last changed.
        fn last_work_day_change(person: UserId) -> Option<Timestamp> = last_work_day_change_in;
    }

    reads! {
        /// The work days of a person from `first` to `last` (both included), by date.
        fn work_days(person: UserId, first: Date, last: Date) -> Vec<StoredDay>;
        /// The work day with this ID.
        fn work_day_by_id(id: WorkDayId) -> Option<StoredDay>;
        /// The newest `limit` events of the changes of a work day, oldest first.
        fn work_day_events(id: WorkDayId, limit: u64) -> Vec<DomainEvent>;
        /// Which of these work days have a change with a reason: the days that show *Changed*.
        /// One grouped read for the whole week, never one read for each day.
        fn work_days_with_reasons(ids: Vec<WorkDayId>) -> Vec<WorkDayId>;
        /// How many work days each person has with a date before `before`, by person; a person
        /// without such days is left out. Counted when asked: no job counts them.
        fn old_work_days(before: Date) -> Vec<OldDays>;
    }

    writes! {
        /// Takes the lock of one person's work days for the rest of the change.
        fn lock_work_days_of(person: UserId) -> ();
        /// Stores a new work day with its periods and breaks; the unique key of the person and
        /// the date refuses a second one.
        fn insert_work_day(day: &StoredDay) -> ();
        /// Writes the periods, breaks, warnings, zone and time of change of a stored day and its
        /// version, and says whether the day was still at `version`.
        fn update_work_day(day: &StoredDay, version: i64) -> bool;
        /// Writes the warnings of a day that did not change itself: the day after a changed one,
        /// whose rest the change moved.
        fn set_work_day_warnings(id: WorkDayId, warnings: &[Warning]) -> ();
        /// Deletes the work days with a date before `before`, of one person or of everybody, with
        /// their periods, breaks, the events of their changes and the notifications about them;
        /// how many of whose were deleted.
        fn delete_old_work_days(before: Date, person: Option<UserId>) -> Vec<OldDays>;
    }
}
