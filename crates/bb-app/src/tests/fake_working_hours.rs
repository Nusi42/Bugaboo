//! The working-hours part of the in-memory store, and the time entries a test books, which the
//! fake of the port of time spent reads (ADR-0035, decision 4).

use bb_core::DomainEvent;
use bb_core::event::{EntityKind, RecordKind};
use bb_core::id::{EntityId, IssueId, TimeEntryId, UserId, WorkDayId};
use bb_core::technical::TechnicalName;
use bb_core::time::Minutes;
use bb_core::time::entry::TimeEntry;
use bb_core::working_time::Warning;
use bb_core::working_time::record::StoredDay;
use jiff::Timestamp;
use jiff::civil::Date;

use super::fake::{FakeError, FakeStore, Tx};
use crate::working_hours::{OldDays, WorkingHoursStore};

/// The days of a person in a state, by date.
fn days_of(days: &[StoredDay], person: UserId) -> impl Iterator<Item = &StoredDay> {
    days.iter().filter(move |day| day.day.person == person)
}

/// How many days older than `before` each person has.
fn counted(days: &[StoredDay], before: Date, person: Option<UserId>) -> Vec<OldDays> {
    let mut found: Vec<OldDays> = Vec::new();
    for day in days
        .iter()
        .filter(|day| day.day.date < before && person.is_none_or(|one| day.day.person == one))
    {
        match found.iter_mut().find(|old| old.person == day.day.person) {
            Some(old) => old.count += 1,
            None => found.push(OldDays {
                person: day.day.person,
                count: 1,
            }),
        }
    }
    found.sort_by_key(|old| old.person);
    found
}

/// Whether an event is about one of these days.
fn about(event: &DomainEvent, days: &[EntityId]) -> bool {
    event.entity.kind == EntityKind::from(RecordKind::WorkDay) && days.contains(&event.entity.id)
}

impl WorkingHoursStore for FakeStore {
    async fn work_day_in(
        tx: &mut Tx,
        person: UserId,
        date: Date,
    ) -> Result<Option<StoredDay>, FakeError> {
        Ok(days_of(&tx.state.work_days, person)
            .find(|day| day.day.date == date)
            .cloned())
    }

    async fn work_day_before_in(
        tx: &mut Tx,
        person: UserId,
        date: Date,
    ) -> Result<Option<StoredDay>, FakeError> {
        Ok(days_of(&tx.state.work_days, person)
            .filter(|day| day.day.date < date)
            .max_by_key(|day| day.day.date)
            .cloned())
    }

    async fn work_day_after_in(
        tx: &mut Tx,
        person: UserId,
        date: Date,
    ) -> Result<Option<StoredDay>, FakeError> {
        Ok(days_of(&tx.state.work_days, person)
            .filter(|day| day.day.date > date)
            .min_by_key(|day| day.day.date)
            .cloned())
    }

    async fn open_work_day_in(tx: &mut Tx, person: UserId) -> Result<Option<StoredDay>, FakeError> {
        Ok(days_of(&tx.state.work_days, person)
            .filter(|day| day.times.periods.iter().any(|period| period.end.is_none()))
            .max_by_key(|day| day.day.date)
            .cloned())
    }

    async fn last_work_day_change_in(
        tx: &mut Tx,
        person: UserId,
    ) -> Result<Option<Timestamp>, FakeError> {
        Ok(days_of(&tx.state.work_days, person)
            .map(|day| day.day.updated_at)
            .max())
    }

    async fn work_days(
        &self,
        person: UserId,
        first: Date,
        last: Date,
    ) -> Result<Vec<StoredDay>, FakeError> {
        Ok(self.read(|state| {
            let mut found: Vec<StoredDay> = days_of(&state.work_days, person)
                .filter(|day| first <= day.day.date && day.day.date <= last)
                .cloned()
                .collect();
            found.sort_by_key(|day| day.day.date);
            found
        }))
    }

    async fn work_day_by_id(&self, id: WorkDayId) -> Result<Option<StoredDay>, FakeError> {
        Ok(self.read(|state| state.work_days.iter().find(|day| day.day.id == id).cloned()))
    }

    async fn work_days_with_reasons(
        &self,
        ids: Vec<WorkDayId>,
    ) -> Result<Vec<WorkDayId>, FakeError> {
        let state = self.lock();
        Ok(ids
            .into_iter()
            .filter(|id| {
                state
                    .events
                    .iter()
                    .any(|event| about(event, &[(*id).into()]) && event.reason.is_some())
            })
            .collect())
    }

    async fn work_day_events(
        &self,
        id: WorkDayId,
        limit: u64,
    ) -> Result<Vec<DomainEvent>, FakeError> {
        let all: Vec<DomainEvent> = self
            .lock()
            .events
            .iter()
            .filter(|event| about(event, &[id.into()]))
            .cloned()
            .collect();
        let keep = usize::try_from(limit).unwrap_or(usize::MAX);
        Ok(all[all.len().saturating_sub(keep)..].to_vec())
    }

    async fn old_work_days(&self, before: Date) -> Result<Vec<OldDays>, FakeError> {
        Ok(self.read(|state| counted(&state.work_days, before, None)))
    }

    async fn lock_work_days_of(_tx: &mut Tx, _person: UserId) -> Result<(), FakeError> {
        Ok(())
    }

    async fn insert_work_day(tx: &mut Tx, day: &StoredDay) -> Result<(), FakeError> {
        let taken = tx
            .state
            .work_days
            .iter()
            .any(|known| known.day.person == day.day.person && known.day.date == day.day.date);
        if taken {
            return Err(FakeError::Duplicate);
        }
        tx.state.work_days.push(day.clone());
        Ok(())
    }

    async fn update_work_day(
        tx: &mut Tx,
        day: &StoredDay,
        version: i64,
    ) -> Result<bool, FakeError> {
        let Some(known) = tx
            .state
            .work_days
            .iter_mut()
            .find(|known| known.day.id == day.day.id && known.day.version == version)
        else {
            return Ok(false);
        };
        *known = day.clone();
        Ok(true)
    }

    async fn set_work_day_warnings(
        tx: &mut Tx,
        id: WorkDayId,
        warnings: &[Warning],
    ) -> Result<(), FakeError> {
        if let Some(known) = tx.state.work_days.iter_mut().find(|day| day.day.id == id) {
            known.day.warnings = warnings.to_vec();
        }
        Ok(())
    }

    async fn delete_old_work_days(
        tx: &mut Tx,
        before: Date,
        person: Option<UserId>,
    ) -> Result<Vec<OldDays>, FakeError> {
        let found = counted(&tx.state.work_days, before, person);
        let doomed: Vec<EntityId> = tx
            .state
            .work_days
            .iter()
            .filter(|day| day.day.date < before && person.is_none_or(|one| day.day.person == one))
            .map(|day| day.day.id.into())
            .collect();
        tx.state
            .work_days
            .retain(|day| !doomed.contains(&day.day.id.into()));
        tx.state
            .notifications
            .retain(|row| row.record.is_none_or(|record| !doomed.contains(&record)));
        Ok(found)
    }
}

impl FakeStore {
    /// Books `minutes` for `person` on `date`: a time entry, which the fake of the port of time
    /// spent (`fake_time.rs`) reads like the store.
    pub(crate) fn book(&self, person: UserId, date: Date, minutes: u32) {
        let mut shared = self.lock();
        let number = u8::try_from(shared.state.time_entries.len()).unwrap_or(u8::MAX);
        let (Ok(id), Ok(issue)) = (
            TimeEntryId::from_parts(Timestamp::UNIX_EPOCH, [number; 10]),
            IssueId::from_parts(Timestamp::UNIX_EPOCH, [7; 10]),
        ) else {
            return;
        };
        shared.state.time_entries.push(TimeEntry {
            id,
            person,
            issue,
            date,
            minutes: Minutes::new(minutes),
            note: String::new(),
            billable: false,
            started_at: None,
            created_at: Timestamp::UNIX_EPOCH,
            version: bb_core::version::FIRST_VERSION,
        });
    }
}

/// The starting roles still to make, for the fake of the policy.
pub(super) fn due_roles(tx: &Tx) -> Vec<TechnicalName> {
    tx.state.starting_roles_due.clone()
}
