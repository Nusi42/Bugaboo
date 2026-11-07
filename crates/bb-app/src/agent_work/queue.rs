//! Queueing the jobs of agent work inside the change that needs them.

use jiff::Timestamp;
use serde_json::Value;

use crate::auth::{AppError, AuthStore, Ctx};
use crate::jobs::{JobKind, QueuedJob};

impl<S: AuthStore> Ctx<'_, S> {
    /// A job of `kind` that runs at `run_at`, ready to be queued inside a change.
    pub(crate) fn queued(
        &self,
        kind: JobKind,
        payload: Value,
        run_at: Timestamp,
    ) -> Result<QueuedJob, AppError> {
        Ok(QueuedJob {
            id: bb_core::id::JobId::generate(&mut self.ids())
                .map_err(|error| AppError::Internal(error.to_string()))?,
            kind: kind.name()?,
            payload,
            run_at,
        })
    }
}

/// Queues the job inside the change.
pub(crate) async fn queue<S: AuthStore>(
    transaction: &mut S::Transaction,
    job: &QueuedJob,
    now: Timestamp,
) -> Result<(), S::Error> {
    S::enqueue_job(
        transaction,
        job.id,
        &job.kind,
        &job.payload,
        job.run_at,
        now,
    )
    .await
}
