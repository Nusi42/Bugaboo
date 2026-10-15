//! The jobs and the runs of changes to many issues of the in-memory store.

use bb_core::bulk::{BulkRun, BulkStatus};
use bb_core::id::{BulkRunId, JobId, UserId};
use bb_core::job::{Job, JobState};
use bb_core::technical::TechnicalName;
use jiff::{SignedDuration, Timestamp};
use serde_json::Value;

use super::fake::{FakeError, FakeStore, Tx};
use crate::bulk::BulkStore;
use crate::jobs::{JOB_ATTEMPTS_MAX, JobStore};

fn job(id: JobId, kind: &TechnicalName, payload: &Value, run_at: Timestamp, now: Timestamp) -> Job {
    Job {
        id,
        kind: kind.clone(),
        payload: payload.clone(),
        run_at,
        attempts: 0,
        max_attempts: JOB_ATTEMPTS_MAX,
        state: JobState::Queued,
        lease_owner: None,
        lease_until: None,
        last_failure: None,
        created_at: now,
    }
}

impl JobStore for FakeStore {
    async fn lease_jobs(
        &self,
        worker: &TechnicalName,
        now: Timestamp,
        lease: SignedDuration,
        limit: u64,
    ) -> Result<Vec<Job>, FakeError> {
        let mut shared = self.lock();
        let mut taken = Vec::new();
        let mut due: Vec<&mut Job> = shared
            .state
            .jobs
            .iter_mut()
            .filter(|job| {
                job.state == JobState::Queued
                    && job.run_at <= now
                    && job.lease_until.is_none_or(|until| until <= now)
                    && job.attempts < job.max_attempts
            })
            .collect();
        due.sort_by_key(|job| (job.run_at, job.id));
        for job in due
            .into_iter()
            .take(usize::try_from(limit).unwrap_or(usize::MAX))
        {
            job.lease_owner = Some(worker.clone());
            job.lease_until = now.checked_add(lease).ok();
            job.attempts += 1;
            taken.push(job.clone());
        }
        Ok(taken)
    }

    async fn complete_job(&self, id: JobId, worker: &TechnicalName) -> Result<bool, FakeError> {
        let mut shared = self.lock();
        let before = shared.state.jobs.len();
        shared
            .state
            .jobs
            .retain(|job| !(job.id == id && job.lease_owner.as_ref() == Some(worker)));
        Ok(shared.state.jobs.len() < before)
    }

    async fn fail_job(
        &self,
        id: JobId,
        worker: &TechnicalName,
        failure: &TechnicalName,
        _now: Timestamp,
    ) -> Result<bool, FakeError> {
        let mut shared = self.lock();
        let Some(found) = shared
            .state
            .jobs
            .iter_mut()
            .find(|job| job.id == id && job.lease_owner.as_ref() == Some(worker))
        else {
            return Ok(false);
        };
        found.lease_owner = None;
        found.lease_until = None;
        found.last_failure = Some(failure.clone());
        if found.attempts >= found.max_attempts {
            found.state = JobState::Dead;
        }
        Ok(true)
    }

    async fn later_job_waits(
        &self,
        kind: &TechnicalName,
        after: Timestamp,
    ) -> Result<bool, FakeError> {
        Ok(self
            .lock()
            .state
            .jobs
            .iter()
            .any(|job| job.kind == *kind && job.state == JobState::Queued && job.run_at > after))
    }

    async fn queue_job(
        &self,
        id: JobId,
        kind: &TechnicalName,
        payload: &Value,
        run_at: Timestamp,
        now: Timestamp,
    ) -> Result<(), FakeError> {
        self.lock()
            .state
            .jobs
            .push(job(id, kind, payload, run_at, now));
        Ok(())
    }

    async fn enqueue_job(
        tx: &mut Tx,
        id: JobId,
        kind: &TechnicalName,
        payload: &Value,
        run_at: Timestamp,
        now: Timestamp,
    ) -> Result<(), FakeError> {
        tx.state.jobs.push(job(id, kind, payload, run_at, now));
        Ok(())
    }
}

impl BulkStore for FakeStore {
    async fn bulk_run_in(tx: &mut Tx, id: BulkRunId) -> Result<Option<BulkRun>, FakeError> {
        Ok(tx.state.bulk_runs.iter().find(|run| run.id == id).cloned())
    }

    async fn event_with_reason(&self, reason: &str, since: Timestamp) -> Result<bool, FakeError> {
        Ok(self
            .lock()
            .events
            .iter()
            .any(|event| event.time >= since && event.reason.as_deref() == Some(reason)))
    }

    async fn save_bulk_progress(
        &self,
        run: &BulkRun,
        expected_done: i64,
    ) -> Result<bool, FakeError> {
        let mut shared = self.lock();
        let Some(found) = shared.state.bulk_runs.iter_mut().find(|found| {
            found.id == run.id
                && found.done == expected_done
                && matches!(found.status, BulkStatus::Queued | BulkStatus::Running)
        }) else {
            return Ok(false);
        };
        found.status = run.status;
        found.done = run.done;
        found.changed = run.changed;
        found.skipped.clone_from(&run.skipped);
        found.finished_at = run.finished_at;
        Ok(true)
    }

    async fn insert_bulk_run(tx: &mut Tx, run: &BulkRun) -> Result<(), FakeError> {
        tx.state.bulk_runs.push(run.clone());
        Ok(())
    }

    async fn end_bulk_runs_of(
        tx: &mut Tx,
        person: UserId,
        at: Timestamp,
    ) -> Result<u64, FakeError> {
        let mut ended = 0;
        for run in tx.state.bulk_runs.iter_mut().filter(|run| {
            run.person == person && matches!(run.status, BulkStatus::Queued | BulkStatus::Running)
        }) {
            run.status = BulkStatus::Failed;
            run.finished_at = Some(at);
            ended += 1;
        }
        Ok(ended)
    }

    async fn end_bulk_run(tx: &mut Tx, id: BulkRunId, at: Timestamp) -> Result<bool, FakeError> {
        let Some(run) = tx.state.bulk_runs.iter_mut().find(|run| {
            run.id == id && matches!(run.status, BulkStatus::Queued | BulkStatus::Running)
        }) else {
            return Ok(false);
        };
        run.status = BulkStatus::Failed;
        run.finished_at = Some(at);
        Ok(true)
    }
}
