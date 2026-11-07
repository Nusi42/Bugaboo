//! The port of the module of time entries: what the timer and the time entries need from storage
//! beyond the port of time spent, which the readers outside the module use.
//!
//! Like the other ports, reads take `&self` and **writes** are associated functions that run inside
//! `record_change`, so a change and its domain event are committed together (ARCH-EVENTS).

use std::future::Future;

use bb_core::id::{IssueId, TimeEntryId, UserId};
use bb_core::time::entry::{StarterFieldMark, TimeEntry, Timer};
use bb_core::version::Version;
use jiff::Timestamp;
use jiff::civil::Date;

use super::spent::TimeSpentStore;
use crate::fields::FieldStore;
use crate::hashed_key::HashedKey;
use crate::store_macros::{queries, reads, writes};

/// Where a page of time entries starts: after the entry with this ID, in the order of the page.
pub type EntryCursor = Option<TimeEntryId>;

/// Where a page of the entries of an issue starts for a reader who may not see the moment they were
/// saved: after the entry of this date and hash. It is all a cursor of such a page holds.
pub type DayKey = HashedKey<Date>;

/// What the timer and the time entries need from storage.
pub trait TimeStore: TimeSpentStore + FieldStore + 'static {
    queries! {
        /// The timer of a person, running or not; none for a person who never started one.
        fn timer(person: UserId) -> Option<Timer> = timer_in;
        /// A time entry.
        fn time_entry(id: TimeEntryId) -> Option<TimeEntry> = time_entry_in;
        /// The marks of the starter fields that were made.
        fn starter_fields() -> Vec<StarterFieldMark> = starter_fields_in;
        /// How many time entries a person has on a date.
        fn entries_on_date(person: UserId, date: Date) -> u64 = entries_on_date_in;
        /// The end of the person's last time entry that has a start (its start plus its length);
        /// none when no entry of theirs has one.
        fn last_timed_end(person: UserId) -> Option<Timestamp> = last_timed_end_in;
    }

    reads! {
        /// A person's own time entries from `first` to `last` (both included), by date and then
        /// in the order they were made, a page after `after`.
        fn entries_of_person(
            person: UserId,
            first: Date,
            last: Date,
            after: EntryCursor,
            limit: u64
        ) -> Vec<TimeEntry>;
        /// The time entries on an issue, newest first, a page after `after`.
        fn entries_of_issue(issue: IssueId, after: EntryCursor, limit: u64) -> Vec<TimeEntry>;
        /// The time entries on an issue by date, the latest date first and, among entries of one
        /// date, by the hash of their IDs, a page after `after`, each with its place: the order of a
        /// reader who may not see the moment the entries were saved.
        fn entries_of_issue_by_date(
            issue: IssueId,
            after: Option<DayKey>,
            limit: u64
        ) -> Vec<(TimeEntry, DayKey)>;
    }

    writes! {
        /// Writes the timer of a person: a new row when `version` is none (and says false when the
        /// person has one already), otherwise the row that is still at `version` (false when it is
        /// not).
        fn put_timer(timer: &Timer, version: Option<Version>) -> bool;
        /// Holds the timer row of a person until the change ends (making one that stands still
        /// when there is none), so the changes of one person's time come one after the other.
        fn hold_time_of(person: UserId) -> ();
        /// Adds a time entry.
        fn insert_time_entry(entry: &TimeEntry) -> ();
        /// Writes the date, length, note and *Billable* of an entry, counts its version up, and
        /// says whether it was still at `version`.
        fn update_time_entry(entry: &TimeEntry, version: Version) -> bool;
        /// Deletes an entry with the values of its own fields, and says whether it was still at
        /// `version`.
        fn delete_time_entry(id: TimeEntryId, version: Version) -> bool;
        /// Writes the mark of a starter field, and says false when the mark is there already.
        fn insert_starter_field(mark: &StarterFieldMark) -> bool;
    }
}
