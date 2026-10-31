//! The time part of the in-memory store: timers, time entries, the marks of the starter fields, and
//! the port of time spent, summed in memory.

use bb_core::event::{EntityKind, RecordKind};
use bb_core::id::{EntityId, IssueId, TimeEntryId, UserId};
use bb_core::time::Minutes;
use bb_core::time::entry::{StarterFieldMark, TimeEntry, Timer};
use bb_core::version::Version;
use jiff::civil::Date;
use jiff::{SignedDuration, Timestamp};

use super::fake::{FakeError, FakeStore, State, Tx};
use crate::search::{SearchFailure, SearchRequest};
use crate::time::spent::TimeSpentStore;
use crate::time::{EntryCursor, TimeStore};

/// The minutes booked on an issue.
fn spent_on(state: &State, issue: IssueId) -> Minutes {
    state
        .time_entries
        .iter()
        .filter(|entry| entry.issue == issue)
        .fold(Minutes::default(), |sum, entry| {
            sum.checked_add(entry.minutes).unwrap_or(sum)
        })
}

fn cut(mut found: Vec<TimeEntry>, limit: u64) -> Vec<TimeEntry> {
    found.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
    found
}

impl TimeSpentStore for FakeStore {
    async fn spent_on_issues(
        &self,
        issues: &[IssueId],
    ) -> Result<Vec<(IssueId, Minutes)>, FakeError> {
        Ok(self.read(|state| {
            issues
                .iter()
                .map(|issue| (*issue, spent_on(state, *issue)))
                .collect()
        }))
    }

    async fn spent_on_search(&self, request: &SearchRequest) -> Result<Minutes, SearchFailure> {
        // The fake does not search: it sums the issues a test put among the hits.
        let mut shared = self.lock();
        shared.searches.push(request.clone());
        let hits: Vec<IssueId> = shared.search_hits.iter().map(|hit| hit.issue.id).collect();
        Ok(hits.iter().fold(Minutes::default(), |sum, issue| {
            sum.checked_add(spent_on(&shared.state, *issue))
                .unwrap_or(sum)
        }))
    }

    async fn booked_per_date(
        &self,
        person: UserId,
        first: Date,
        last: Date,
    ) -> Result<Vec<(Date, Minutes)>, FakeError> {
        Ok(self.read(|state| {
            let mut dates: Vec<(Date, Minutes)> = Vec::new();
            for entry in state
                .time_entries
                .iter()
                .filter(|entry| entry.person == person && first <= entry.date && entry.date <= last)
            {
                match dates.iter_mut().find(|(date, _)| *date == entry.date) {
                    Some((_, sum)) => *sum = sum.checked_add(entry.minutes).unwrap_or(*sum),
                    None => dates.push((entry.date, entry.minutes)),
                }
            }
            dates.sort_by_key(|(date, _)| *date);
            dates
        }))
    }
}

impl TimeStore for FakeStore {
    async fn timer_in(tx: &mut Tx, person: UserId) -> Result<Option<Timer>, FakeError> {
        Ok(tx
            .state
            .timers
            .iter()
            .find(|timer| timer.person == person)
            .cloned())
    }

    async fn time_entry_in(tx: &mut Tx, id: TimeEntryId) -> Result<Option<TimeEntry>, FakeError> {
        Ok(tx
            .state
            .time_entries
            .iter()
            .find(|entry| entry.id == id)
            .cloned())
    }

    async fn starter_fields_in(tx: &mut Tx) -> Result<Vec<StarterFieldMark>, FakeError> {
        Ok(tx.state.starter_fields.clone())
    }

    async fn entries_on_date_in(tx: &mut Tx, person: UserId, date: Date) -> Result<u64, FakeError> {
        let count = tx
            .state
            .time_entries
            .iter()
            .filter(|entry| entry.person == person && entry.date == date)
            .count();
        Ok(u64::try_from(count).unwrap_or(u64::MAX))
    }

    async fn last_timed_end_in(
        tx: &mut Tx,
        person: UserId,
    ) -> Result<Option<Timestamp>, FakeError> {
        Ok(tx
            .state
            .time_entries
            .iter()
            .filter(|entry| entry.person == person)
            .filter_map(|entry| {
                entry
                    .started_at?
                    .checked_add(SignedDuration::from_secs(entry.minutes.as_seconds()))
                    .ok()
            })
            .max())
    }

    async fn entries_of_person(
        &self,
        person: UserId,
        first: Date,
        last: Date,
        after: EntryCursor,
        limit: u64,
    ) -> Result<Vec<TimeEntry>, FakeError> {
        Ok(self.read(|state| {
            let mut found: Vec<TimeEntry> = state
                .time_entries
                .iter()
                .filter(|entry| entry.person == person && first <= entry.date && entry.date <= last)
                .cloned()
                .collect();
            found.sort_by_key(|entry| (entry.date, entry.id));
            if let Some(after) = after.and_then(|id| found.iter().position(|entry| entry.id == id))
            {
                found.drain(..=after);
            }
            cut(found, limit)
        }))
    }

    async fn entries_of_issue(
        &self,
        issue: IssueId,
        after: EntryCursor,
        limit: u64,
    ) -> Result<Vec<TimeEntry>, FakeError> {
        Ok(self.read(|state| {
            let mut found: Vec<TimeEntry> = state
                .time_entries
                .iter()
                .filter(|entry| entry.issue == issue && after.is_none_or(|after| entry.id < after))
                .cloned()
                .collect();
            found.sort_by_key(|entry| std::cmp::Reverse(entry.id));
            cut(found, limit)
        }))
    }

    async fn put_timer(
        tx: &mut Tx,
        timer: &Timer,
        version: Option<Version>,
    ) -> Result<bool, FakeError> {
        let found = tx
            .state
            .timers
            .iter_mut()
            .find(|stored| stored.person == timer.person);
        match (found, version) {
            (None, None) => {
                tx.state.timers.push(timer.clone());
                Ok(true)
            }
            (Some(stored), Some(version)) if stored.version == version.get() => {
                *stored = timer.clone();
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    async fn hold_time_of(_tx: &mut Tx, _person: UserId) -> Result<(), FakeError> {
        // The in-memory store runs one change at a time.
        Ok(())
    }

    async fn insert_time_entry(tx: &mut Tx, entry: &TimeEntry) -> Result<(), FakeError> {
        tx.state.time_entries.push(entry.clone());
        Ok(())
    }

    async fn update_time_entry(
        tx: &mut Tx,
        entry: &TimeEntry,
        version: Version,
    ) -> Result<bool, FakeError> {
        match tx
            .state
            .time_entries
            .iter_mut()
            .find(|stored| stored.id == entry.id && stored.version == version.get())
        {
            Some(stored) => {
                *stored = entry.clone();
                Ok(true)
            }
            None => Ok(false),
        }
    }

    async fn delete_time_entry(
        tx: &mut Tx,
        id: TimeEntryId,
        version: Version,
    ) -> Result<bool, FakeError> {
        let before = tx.state.time_entries.len();
        tx.state
            .time_entries
            .retain(|entry| !(entry.id == id && entry.version == version.get()));
        if tx.state.time_entries.len() == before {
            return Ok(false);
        }
        let (kind, entity) = (EntityKind::from(RecordKind::TimeEntry), EntityId::from(id));
        tx.state
            .field_values
            .retain(|value| !(value.kind == kind && value.entity == entity));
        Ok(true)
    }

    async fn insert_starter_field(tx: &mut Tx, mark: &StarterFieldMark) -> Result<bool, FakeError> {
        if tx
            .state
            .starter_fields
            .iter()
            .any(|known| known.starter == mark.starter)
        {
            return Ok(false);
        }
        tx.state.starter_fields.push(*mark);
        Ok(true)
    }
}
