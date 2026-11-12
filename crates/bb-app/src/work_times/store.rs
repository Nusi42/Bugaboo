//! The port of work times: what the use cases of work times, and the plan through them, need from
//! storage (PLAN-11, PLAN-15, ADR-0030, decision 2).
//!
//! Like the other ports, reads take `&self` and **writes** are associated functions that run inside
//! `record_change`, so a change and its domain event are committed together (ARCH-EVENTS). The
//! person's public holidays are the setting *Holidays*, read and written through the registry
//! (`SettingsStore`), never by this port.
//!
//! The plan (WP-06) reads work times through [`super::work_minutes`], which asks this port for the
//! records of the people and the span of days, and works the minutes out with
//! `bb_core::work_times::minutes_on`: a fake of this port is all a test of the plan needs.

use std::future::Future;

use bb_core::id::{AwayTimeId, EntityId, ProjectId, UserId};
use bb_core::work_times::record::AwayTime;
use bb_core::work_times::{Share, StoredUsualDays, UsualDays};
use jiff::civil::Date;

use crate::settings::SettingsStore;
use crate::store_macros::{queries, reads, writes};

/// What the use cases of work times need from storage.
pub trait WorkTimesStore: SettingsStore + 'static {
    queries! {
        /// The usual work days of a person with their version, if they have changed them; a person
        /// who has not has [`UsualDays::start`] and version 0.
        fn usual_days(person: UserId) -> Option<StoredUsualDays> = usual_days_in;
        /// The share of a person in a project, if one was set; a person with none has
        /// [`Share::START`].
        fn share(person: UserId, project: ProjectId) -> Option<Share> = share_in;
        /// The away time with this ID, with its note.
        fn away_time(id: AwayTimeId) -> Option<AwayTime> = away_time_in;
    }

    reads! {
        /// The usual work days of these people; a person who never changed them is left out.
        fn usual_days_of(people: Vec<UserId>) -> Vec<(UserId, UsualDays)>;
        /// The shares that were set for these people in all projects.
        fn shares_of(people: Vec<UserId>) -> Vec<(UserId, ProjectId, Share)>;
        /// The shares that were set in one project for these people.
        fn shares_in(project: ProjectId, people: Vec<UserId>) -> Vec<(UserId, Share)>;
        /// The away times of these people that touch the days from `first` to `last`, by first day,
        /// with their notes (the use case decides who reads a note), at most `limit` of them.
        fn away_times_of(
            people: Vec<UserId>,
            first: Date,
            last: Date,
            limit: Option<u64>
        ) -> Vec<AwayTime>;
    }

    writes! {
        /// Writes the seven usual work days of a person, in place of the ones they had, and counts
        /// the version up, if they are still at `version` (0 while the person has none of their
        /// own); says whether they were.
        fn put_usual_days(person: UserId, days: &UsualDays, version: i64) -> bool;
        /// Writes the share of a person in a project, in place of the one they had.
        fn put_share(person: UserId, project: ProjectId, share: Share) -> ();
        /// Stores a new away time.
        fn insert_away_time(time: &AwayTime) -> ();
        /// Writes the days, the half days, the note and the version of a stored away time, and says
        /// whether the away time was still at `version`.
        fn update_away_time(time: &AwayTime, version: i64) -> bool;
        /// Deletes an away time if it is still at `version`, and says whether it was.
        fn delete_away_time(id: AwayTimeId, version: i64) -> bool;
        /// Deletes the away times with a last day before `before`, with the events of their
        /// changes, the outbox entries of those and the notifications about them; the people who
        /// lost some, with how many.
        fn delete_old_away_times(before: Date) -> Vec<(UserId, u64)>;
        /// Deletes everything of work times that belongs to a person: usual work days, shares and
        /// away times with their events, the outbox entries and the notifications about them. The
        /// person's choice of *Holidays* is a setting and goes through the registry; `setting_record`
        /// is the record its events are about, which this deletes too.
        fn delete_work_times_of(person: UserId, setting_record: EntityId) -> ();
    }
}
