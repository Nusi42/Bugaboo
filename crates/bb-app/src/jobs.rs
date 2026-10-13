//! The queue of background jobs as the use cases see it (ADR-0017): what puts a job in the queue,
//! and what a runner needs to take a job, report it done or report it failed.
//!
//! A job is a kind and a payload. The kinds are named once, here, and [`Ctx::run_due_jobs`] is the
//! one place that says which use case does the work of which kind, so a runner (`bb-jobs`) knows
//! nothing of them.

use std::future::Future;

use bb_core::id::JobId;
use bb_core::job::Job;
use bb_core::technical::TechnicalName;
use jiff::{SignedDuration, Timestamp};
use serde_json::Value;

use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::ports::ChangeStore;
use crate::store_macros::{reads, writes};

/// How many starts a job gets before it goes to the dead-letter list.
pub const JOB_ATTEMPTS_MAX: u32 = 5;

/// The kinds of job, each with the name the queue stores.
pub mod kind {
    /// One step of a change to many issues (`bulk`); the payload names the run.
    pub const BULK_STEP: &str = "bulk.step";
}

/// The code a job reports when its kind is not one any use case does.
const UNKNOWN_KIND: &str = "job.unknown_kind";

/// The code a job reports when its work failed.
const FAILED: &str = "job.failed";

/// What a job needs from storage.
pub trait JobStore: ChangeStore + 'static {
    reads! {
        /// Takes up to `limit` jobs that are due and not leased, for `worker`, until `now + lease`.
        fn lease_jobs(
            worker: &TechnicalName,
            now: Timestamp,
            lease: SignedDuration,
            limit: u64
        ) -> Vec<Job>;
        /// Reports a job as done; it leaves the queue. `false` when `worker` does not hold the
        /// lease any more.
        fn complete_job(id: JobId, worker: &TechnicalName) -> bool;
        /// Reports a job as failed, with a technical code. `false` when `worker` does not hold the
        /// lease any more.
        fn fail_job(
            id: JobId,
            worker: &TechnicalName,
            failure: &TechnicalName,
            now: Timestamp
        ) -> bool;
        /// Puts a job in the queue on its own: for the rest of a piece of work that a job did a
        /// step of (the step is bookkeeping of a run that is stored already, not a domain change).
        fn queue_job(
            id: JobId,
            kind: &TechnicalName,
            payload: &Value,
            run_at: Timestamp,
            now: Timestamp
        ) -> ();
    }

    writes! {
        /// Puts a job in the queue inside the change that needs the work done: both are written
        /// together or neither.
        fn enqueue_job(
            id: JobId,
            kind: &TechnicalName,
            payload: &Value,
            run_at: Timestamp,
            now: Timestamp
        ) -> ();
    }
}

/// The name of a kind of job as the queue stores it.
///
/// # Errors
/// An internal error when the name is not a technical name (it is one of [`kind`]).
pub(crate) fn kind_name(name: &str) -> Result<TechnicalName, AppError> {
    TechnicalName::new(name).map_err(|error| AppError::Internal(error.to_string()))
}

impl<S: AuthStore> Ctx<'_, S> {
    /// What a worker does with a job it holds: `Err` carries a technical code of the failure, and
    /// the queue tries the job again later or, after its last start, puts it on the dead-letter
    /// list.
    async fn do_job(&self, job: &Job) -> Result<(), TechnicalName> {
        let failed = |code: &str| TechnicalName::new(code).unwrap_or_else(|_| job.kind.clone());
        match job.kind.as_str() {
            kind::BULK_STEP => self.run_bulk_step(&job.payload).await.map_err(|error| {
                tracing::warn!(cause = %error, "a step of a change to many issues failed");
                failed(FAILED)
            }),
            other => {
                tracing::error!(kind = other, "a job of a kind nobody knows");
                Err(failed(UNKNOWN_KIND))
            }
        }
    }

    /// Runs the due jobs once: takes up to `limit` jobs for `worker`, does each, and reports it
    /// done or failed. Returns how many it took.
    ///
    /// # Errors
    /// The error of the store while jobs are taken.
    pub async fn run_due_jobs(
        &self,
        worker: &TechnicalName,
        lease: SignedDuration,
        limit: u64,
    ) -> Result<usize, AppError> {
        let jobs = self
            .store()
            .lease_jobs(worker, self.now(), lease, limit)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let taken = jobs.len();
        for job in jobs {
            let reported = match self.do_job(&job).await {
                Ok(()) => self.store().complete_job(job.id, worker).await,
                Err(failure) => {
                    self.store()
                        .fail_job(job.id, worker, &failure, self.now())
                        .await
                }
            };
            if let Err(error) = reported {
                tracing::error!(cause = %S::describe(&error), "a job could not be reported");
            }
        }
        Ok(taken)
    }
}
