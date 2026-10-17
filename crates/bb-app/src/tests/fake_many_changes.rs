//! The jobs and the runs of changes to many issues of the in-memory store.

use bb_core::id::{JobId, ManyChangeRunId};
use bb_core::job::{FailureKind, FailureOutcome, Job, JobState};
use bb_core::many_changes::{ManyChangeRun, ManyChangeStatus, RunsToEnd};
use bb_core::technical::TechnicalName;
use jiff::{SignedDuration, Timestamp};
use serde_json::Value;

use super::fake::{FakeError, FakeStore, Tx};
use crate::jobs::{JOB_ATTEMPTS_MAX, JobStore};
use crate::many_changes::ManyChangeStore;

pub(super) fn job(
    id: JobId,
    kind: &TechnicalName,
    payload: &Value,
    run_at: Timestamp,
    now: Timestamp,
) -> Job {
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
        kind: FailureKind,
        now: Timestamp,
    ) -> Result<Option<FailureOutcome>, FakeError> {
        let mut shared = self.lock();
        let Some(found) = shared
            .state
            .jobs
            .iter_mut()
            .find(|job| job.id == id && job.lease_owner.as_ref() == Some(worker))
        else {
            return Ok(None);
        };
        found.lease_owner = None;
        found.lease_until = None;
        found.last_failure = Some(failure.clone());
        if kind == FailureKind::Permanent || found.attempts >= found.max_attempts {
            found.state = JobState::Dead;
            Ok(Some(FailureOutcome::Dead))
        } else {
            // The store waits before the next start; thirty seconds is its first wait.
            let again = now
                .checked_add(SignedDuration::from_secs(30))
                .unwrap_or(now);
            found.run_at = again;
            Ok(Some(FailureOutcome::Retry(again)))
        }
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

    async fn clear_dead_job_payloads(&self, before: Timestamp) -> Result<u64, FakeError> {
        let mut emptied = 0;
        for job in &mut self.lock().state.jobs {
            if job.state == JobState::Dead && job.run_at < before && !job.payload.is_null() {
                job.payload = Value::Null;
                emptied += 1;
            }
        }
        Ok(emptied)
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

impl ManyChangeStore for FakeStore {
    async fn many_changes_run_in(
        tx: &mut Tx,
        id: ManyChangeRunId,
    ) -> Result<Option<ManyChangeRun>, FakeError> {
        Ok(tx
            .state
            .many_changes_runs
            .iter()
            .find(|run| run.id == id)
            .cloned())
    }

    async fn event_with_reason(&self, reason: &str, since: Timestamp) -> Result<bool, FakeError> {
        Ok(self
            .lock()
            .events
            .iter()
            .any(|event| event.time >= since && event.reason.as_deref() == Some(reason)))
    }

    async fn save_many_changes_progress(
        &self,
        run: &ManyChangeRun,
        expected_done: i64,
    ) -> Result<bool, FakeError> {
        let mut shared = self.lock();
        if shared.progress_failures > 0 {
            shared.progress_failures -= 1;
            return Err(FakeError::Failed);
        }
        let Some(found) = shared.state.many_changes_runs.iter_mut().find(|found| {
            found.id == run.id
                && found.done == expected_done
                && matches!(
                    found.status,
                    ManyChangeStatus::Queued | ManyChangeStatus::Running
                )
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

    async fn insert_many_changes_run(tx: &mut Tx, run: &ManyChangeRun) -> Result<(), FakeError> {
        tx.state.many_changes_runs.push(run.clone());
        Ok(())
    }

    async fn many_changes_run_is_over(&self, id: ManyChangeRunId) -> Result<bool, FakeError> {
        Ok(!self.lock().state.many_changes_runs.iter().any(|run| {
            run.id == id
                && matches!(
                    run.status,
                    ManyChangeStatus::Queued | ManyChangeStatus::Running
                )
        }))
    }

    async fn end_many_changes_runs(
        tx: &mut Tx,
        which: &RunsToEnd,
        status: ManyChangeStatus,
        at: Timestamp,
    ) -> Result<u64, FakeError> {
        let mut ended = 0;
        for run in tx.state.many_changes_runs.iter_mut().filter(|run| {
            matches!(
                run.status,
                ManyChangeStatus::Queued | ManyChangeStatus::Running
            ) && match which {
                RunsToEnd::One(id) => run.id == *id,
                RunsToEnd::OfPerson(person) => run.person == *person,
            }
        }) {
            run.status = status;
            run.finished_at = Some(at);
            ended += 1;
        }
        Ok(ended)
    }
}
