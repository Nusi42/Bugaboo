//! The queue of background jobs as the use cases see it (ADR-0017): what puts a job in the queue,
//! and what a runner needs to take a job, report it done or report it failed.
//!
//! A job is a kind and a payload. The kinds are named once, here, and [`Ctx::run_due_jobs`] is the
//! one place that says which use case does the work of which kind, so a runner (`bb-jobs`) knows
//! nothing of them.

use std::future::Future;

use bb_core::id::JobId;
use bb_core::job::{FailureKind, FailureOutcome, Job};
use bb_core::technical::TechnicalName;
use jiff::{SignedDuration, Timestamp};
use serde_json::Value;

use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::editing::{PurgeDraftsQuery, purge_drafts};
use crate::ports::ChangeStore;
use crate::store_macros::{reads, writes};

/// How many starts a job gets before it goes to the dead-letter list.
pub const JOB_ATTEMPTS_MAX: u32 = 5;

/// The kinds of job, each with the name the queue stores.
pub mod kind {
    /// One step of a change to many issues (`many-changes`); the payload names the run.
    pub const MANY_CHANGES_STEP: &str = "many-changes.step";
    /// One step of giving the issues of a project new IDs after its mode of IDs changed
    /// (`issues::ids`); the payload names the project and the person who changed the mode.
    pub const RENEW_IDS: &str = "issues.renew-ids";
    /// Clearing what outlived its keep period (drafts, notes for people who were editing, lock
    /// rows that ran out): the use case `purge_drafts`. It comes back every day.
    pub const PURGE_DRAFTS: &str = "editing.purge-drafts";
    /// The history catching up with the events (M3-F71): `Ctx::catch_up_history`. It comes back
    /// every day.
    pub const CATCH_UP_HISTORY: &str = "history.catch-up";
    /// The kinds that come back every day, see [`super::DAILY`].
    pub const DAILY_JOBS: [&str; 2] = [CATCH_UP_HISTORY, PURGE_DRAFTS];
}

/// How long after a daily job its next round is due.
pub const DAILY: SignedDuration = SignedDuration::from_hours(24);

/// How long a worker holds the jobs it took before another may take them: a step of a job is far
/// shorter, and a worker that stops loses its jobs to another after this.
const LEASE: SignedDuration = SignedDuration::from_secs(120);

/// How many jobs a round takes.
const JOBS_AT_A_TIME: u64 = 5;

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
        /// Reports a job as failed, with a technical code, and says what happened to it: it runs
        /// again later or it is on the dead-letter list (after its last start, or at once for a
        /// failure that is permanent). `None` when `worker` does not hold the lease any more.
        fn fail_job(
            id: JobId,
            worker: &TechnicalName,
            failure: &TechnicalName,
            kind: FailureKind,
            now: Timestamp
        ) -> Option<FailureOutcome>;
        /// Whether a job of this kind waits in the queue (not dead) for a time after `after`.
        fn later_job_waits(kind: &TechnicalName, after: Timestamp) -> bool;
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

/// How a job failed: the technical code the queue keeps, and whether another start can help.
struct JobFailure {
    code: TechnicalName,
    kind: FailureKind,
}

impl JobFailure {
    /// The failure of `error`: a refusal (the payload names nothing, the request is not allowed)
    /// can never succeed as it was asked for, so it is not tried again; every other error (the
    /// store broke) may pass.
    fn of(error: &AppError, code: &str, job: &Job) -> Self {
        Self {
            code: TechnicalName::new(code).unwrap_or_else(|_| job.kind.clone()),
            kind: match error {
                AppError::Refused(_) => FailureKind::Permanent,
                _ => FailureKind::Temporary,
            },
        }
    }

    fn unknown_kind(job: &Job) -> Self {
        Self {
            code: TechnicalName::new(UNKNOWN_KIND).unwrap_or_else(|_| job.kind.clone()),
            kind: FailureKind::Permanent,
        }
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// What a worker does with a job it holds: `Err` says how it failed, and the queue tries the
    /// job again later or, after its last start (or at once, for a failure that is permanent),
    /// puts it on the dead-letter list.
    async fn do_job(&self, job: &Job) -> Result<(), JobFailure> {
        match job.kind.as_str() {
            kind::MANY_CHANGES_STEP => Box::pin(self.run_many_changes_step(&job.payload))
                .await
                .map_err(|error| {
                    tracing::warn!(cause = %error, "a step of a change to many issues failed");
                    JobFailure::of(&error, FAILED, job)
                }),
            kind::RENEW_IDS => Box::pin(self.renew_ids_step(&job.payload))
                .await
                .map_err(|error| {
                    tracing::warn!(cause = %error, "a step of renewing issue IDs failed");
                    JobFailure::of(&error, FAILED, job)
                }),
            kind::PURGE_DRAFTS => Box::pin(self.run_daily(kind::PURGE_DRAFTS, async {
                purge_drafts(self, PurgeDraftsQuery).await.map(|drafts| {
                    tracing::info!(drafts, "old drafts were cleared");
                })
            }))
            .await
            .map_err(|error| {
                tracing::warn!(cause = %error, "old drafts could not be cleared");
                JobFailure::of(&error, FAILED, job)
            }),
            kind::CATCH_UP_HISTORY => Box::pin(self.run_daily(kind::CATCH_UP_HISTORY, async {
                self.catch_up_history().await.map(|events| {
                    tracing::info!(events, "the history caught up with the events");
                })
            }))
            .await
            .map_err(|error| {
                tracing::warn!(cause = %error, "the history could not catch up");
                JobFailure::of(&error, FAILED, job)
            }),
            other => {
                tracing::error!(kind = other, "a job of a kind nobody knows");
                Err(JobFailure::unknown_kind(job))
            }
        }
    }

    /// What happens when a job is on the dead-letter list: the work that people can see ends in
    /// a state they can see. A change to many issues ends as failed with what it did so far; the
    /// renewal of issue IDs has no record of its own, so its death is only logged (M3-F150).
    async fn job_gave_up(&self, job: &Job) {
        let ended = match job.kind.as_str() {
            kind::MANY_CHANGES_STEP => self.fail_many_changes_run(&job.payload).await,
            kind::RENEW_IDS => {
                tracing::error!("renewing issue IDs gave up; the job is on the dead-letter list");
                Ok(())
            }
            _ => Ok(()),
        };
        if let Err(error) = ended {
            tracing::error!(cause = %error, "the work of a dead job could not be ended");
        }
    }

    /// Does the work of a job of a daily kind. Its next round is queued first, so a round that
    /// fails (and is tried again by the queue) or a worker that stops does not end the daily
    /// rounds; nothing is queued when a round for a later time waits already, so a server that
    /// starts again within the day (see [`Ctx::start_daily_jobs`]) does not add a second chain.
    async fn run_daily(
        &self,
        name: &str,
        work: impl Future<Output = Result<(), AppError>>,
    ) -> Result<(), AppError> {
        let kind = kind_name(name)?;
        let now = self.now();
        let waits = self
            .store()
            .later_job_waits(&kind, now)
            .await
            .map_err(|error| internal::<S>(&error))?;
        if !waits {
            let next = now
                .checked_add(DAILY)
                .map_err(|error| AppError::Internal(error.to_string()))?;
            self.queue_daily(&kind, next).await?;
        }
        work.await
    }

    async fn queue_daily(&self, kind: &TechnicalName, at: Timestamp) -> Result<(), AppError> {
        let id = JobId::generate(&mut self.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?;
        self.store()
            .queue_job(id, kind, &Value::Null, at, self.now())
            .await
            .map_err(|error| internal::<S>(&error))
    }

    /// Queues a round of every daily job at once: the server calls it when it starts, so old drafts
    /// are cleared and the history is caught up at every start, and the rounds after that come
    /// every [`DAILY`] from the jobs themselves.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub async fn start_daily_jobs(&self) -> Result<(), AppError> {
        for name in kind::DAILY_JOBS {
            self.queue_daily(&kind_name(name)?, self.now()).await?;
        }
        Ok(())
    }

    /// Runs the due jobs once: takes up to [`JOBS_AT_A_TIME`] jobs for a worker that has a name of
    /// its own for this round, does each, and reports it done or failed. Returns how many it took.
    ///
    /// # Errors
    /// The error of the store while jobs are taken.
    pub async fn run_due_jobs(&self) -> Result<usize, AppError> {
        let worker = self.worker_name()?;
        let jobs = self
            .store()
            .lease_jobs(&worker, self.now(), LEASE, JOBS_AT_A_TIME)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let taken = jobs.len();
        for job in jobs {
            // `true` when the worker still held the lease and the report counted.
            let reported = match self.do_job(&job).await {
                Ok(()) => self.store().complete_job(job.id, &worker).await,
                Err(failure) => {
                    let outcome = self
                        .store()
                        .fail_job(job.id, &worker, &failure.code, failure.kind, self.now())
                        .await;
                    if matches!(outcome, Ok(Some(FailureOutcome::Dead))) {
                        self.job_gave_up(&job).await;
                    }
                    outcome.map(|found| found.is_some())
                }
            };
            match reported {
                Ok(true) => {}
                // The lease ran out and another worker has the job: it reports, not this one.
                Ok(false) => tracing::warn!("a job was reported after its lease ran out"),
                Err(error) => {
                    tracing::error!(cause = %S::describe(&error), "a job could not be reported");
                }
            }
        }
        Ok(taken)
    }

    /// The name a worker holds jobs under for one round: a new one each time, so two rounds, also
    /// of two servers on one database, never report each other's jobs.
    fn worker_name(&self) -> Result<TechnicalName, AppError> {
        let id = JobId::generate(&mut self.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?;
        kind_name(&format!("worker.{}", id.to_string().replace('-', "_")))
    }
}
