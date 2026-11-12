//! The work-times part of the in-memory store (PLAN-11, PLAN-15): the usual work days, the shares and
//! the away times, which the plan of WP-06 can read in its tests the way it reads them in the store.

use bb_core::id::{AwayTimeId, EntityId, ProjectId, UserId};
use bb_core::notification::NotificationKind;
use bb_core::work_times::record::AwayTime;
use bb_core::work_times::{Share, UsualDays};
use jiff::civil::Date;

use super::fake::{FakeError, FakeStore, Tx};
use crate::work_times::WorkTimesStore;

impl WorkTimesStore for FakeStore {
    async fn usual_days_in(tx: &mut Tx, person: UserId) -> Result<Option<UsualDays>, FakeError> {
        Ok(tx
            .state
            .usual_days
            .iter()
            .find(|(known, _)| *known == person)
            .map(|(_, days)| *days))
    }

    async fn share_in(
        tx: &mut Tx,
        person: UserId,
        project: ProjectId,
    ) -> Result<Option<Share>, FakeError> {
        Ok(tx
            .state
            .shares
            .iter()
            .find(|(who, which, _)| *who == person && *which == project)
            .map(|(_, _, share)| *share))
    }

    async fn away_time_in(tx: &mut Tx, id: AwayTimeId) -> Result<Option<AwayTime>, FakeError> {
        Ok(tx
            .state
            .away_times
            .iter()
            .find(|time| time.id == id)
            .cloned())
    }

    async fn usual_days_of(
        &self,
        people: Vec<UserId>,
    ) -> Result<Vec<(UserId, UsualDays)>, FakeError> {
        Ok(self.read(|state| {
            state
                .usual_days
                .iter()
                .filter(|(person, _)| people.contains(person))
                .copied()
                .collect()
        }))
    }

    async fn shares_of(
        &self,
        people: Vec<UserId>,
    ) -> Result<Vec<(UserId, ProjectId, Share)>, FakeError> {
        Ok(self.read(|state| {
            let mut found: Vec<_> = state
                .shares
                .iter()
                .filter(|(person, _, _)| people.contains(person))
                .copied()
                .collect();
            found.sort_by_key(|(person, project, _)| (*person, *project));
            found
        }))
    }

    async fn shares_in(
        &self,
        project: ProjectId,
        people: Vec<UserId>,
    ) -> Result<Vec<(UserId, Share)>, FakeError> {
        Ok(self.read(|state| {
            state
                .shares
                .iter()
                .filter(|(person, which, _)| *which == project && people.contains(person))
                .map(|(person, _, share)| (*person, *share))
                .collect()
        }))
    }

    async fn away_times_of(
        &self,
        people: Vec<UserId>,
        first: Date,
        last: Date,
    ) -> Result<Vec<AwayTime>, FakeError> {
        Ok(self.read(|state| {
            let mut found: Vec<AwayTime> = state
                .away_times
                .iter()
                .filter(|time| {
                    people.contains(&time.person) && time.first <= last && first <= time.last
                })
                .cloned()
                .collect();
            found.sort_by_key(|time| (time.first, time.id));
            found
        }))
    }

    async fn put_usual_days(
        tx: &mut Tx,
        person: UserId,
        days: &UsualDays,
    ) -> Result<(), FakeError> {
        tx.state.usual_days.retain(|(known, _)| *known != person);
        tx.state.usual_days.push((person, *days));
        Ok(())
    }

    async fn put_share(
        tx: &mut Tx,
        person: UserId,
        project: ProjectId,
        share: Share,
    ) -> Result<(), FakeError> {
        tx.state
            .shares
            .retain(|(who, which, _)| !(*who == person && *which == project));
        tx.state.shares.push((person, project, share));
        Ok(())
    }

    async fn insert_away_time(tx: &mut Tx, time: &AwayTime) -> Result<(), FakeError> {
        tx.state.away_times.push(time.clone());
        Ok(())
    }

    async fn update_away_time(
        tx: &mut Tx,
        time: &AwayTime,
        version: i64,
    ) -> Result<bool, FakeError> {
        let Some(known) = tx
            .state
            .away_times
            .iter_mut()
            .find(|known| known.id == time.id && known.version == version)
        else {
            return Ok(false);
        };
        *known = time.clone();
        Ok(true)
    }

    async fn delete_away_time(
        tx: &mut Tx,
        id: AwayTimeId,
        version: i64,
    ) -> Result<bool, FakeError> {
        let before = tx.state.away_times.len();
        tx.state
            .away_times
            .retain(|time| !(time.id == id && time.version == version));
        Ok(tx.state.away_times.len() < before)
    }

    async fn delete_old_away_times(
        tx: &mut Tx,
        before: Date,
    ) -> Result<Vec<(UserId, u64)>, FakeError> {
        let mut found: Vec<(UserId, u64)> = Vec::new();
        for time in tx.state.away_times.iter().filter(|time| time.last < before) {
            match found.iter_mut().find(|(person, _)| *person == time.person) {
                Some((_, count)) => *count += 1,
                None => found.push((time.person, 1)),
            }
        }
        found.sort_by_key(|(person, _)| *person);
        let doomed: Vec<EntityId> = tx
            .state
            .away_times
            .iter()
            .filter(|time| time.last < before)
            .map(|time| time.id.into())
            .collect();
        tx.state.away_times.retain(|time| time.last >= before);
        tx.state.notifications.retain(|row| {
            row.kind != NotificationKind::WorkTimesChanged
                || row.record.is_none_or(|record| !doomed.contains(&record))
        });
        Ok(found)
    }

    async fn delete_work_times_of(tx: &mut Tx, person: UserId) -> Result<(), FakeError> {
        tx.state.usual_days.retain(|(known, _)| *known != person);
        tx.state.shares.retain(|(known, _, _)| *known != person);
        tx.state.away_times.retain(|time| time.person != person);
        tx.state.notifications.retain(|row| {
            row.kind != NotificationKind::WorkTimesChanged || row.recipient != person
        });
        Ok(())
    }
}
