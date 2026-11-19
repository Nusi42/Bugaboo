//! The daily rounds that have a function of their own, so `do_daily_job` stays short.

use super::{AuthStore, Ctx, FAILED, Job, JobFailure, JobKind};

impl<S: AuthStore> Ctx<'_, S> {
    /// The daily round, or a further step, of a job that deletes what outlived its keep period:
    /// the one shape of deleting by itself (`Ctx::keep_step`, ADR-0045, decision 5).
    pub(super) async fn keep_daily(&self, job: &Job, kind: JobKind) -> Result<(), JobFailure> {
        Box::pin(self.run_daily(kind, async {
            self.keep_step(kind).await.map(|step| {
                tracing::info!(
                    kind = kind.code(),
                    rows = step.rows,
                    more = step.more,
                    "data past its keep period was deleted"
                );
            })
        }))
        .await
        .map_err(|error| {
            tracing::warn!(cause = %error, kind = kind.code(), "data past its keep period could not be deleted");
            JobFailure::of(&error, FAILED, job)
        })
    }

    /// The daily round of `backups.daily` (OPS-07).
    pub(super) async fn backups_daily(&self, job: &Job) -> Result<(), JobFailure> {
        Box::pin(self.run_daily(JobKind::BackupsDaily, async {
            self.backup_round().await.map(|round| {
                tracing::info!(
                    created = round.created,
                    deleted = round.deleted,
                    "the backups of the schedule were looked at"
                );
            })
        }))
        .await
        .map_err(|error| {
            tracing::warn!(cause = %error, "the backup of the day could not be created");
            JobFailure::of(&error, FAILED, job)
        })
    }

    /// The daily round of `server-log.rotate`.
    pub(super) async fn rotate_server_log_daily(&self, job: &Job) -> Result<(), JobFailure> {
        Box::pin(self.run_daily(JobKind::RotateServerLog, async {
            self.rotate_old_server_log().await.map(|rotated| {
                tracing::info!(rotated, "the server log was looked at");
            })
        }))
        .await
        .map_err(|error| {
            tracing::warn!(cause = %error, "the server log could not be rotated");
            JobFailure::of(&error, FAILED, job)
        })
    }

    /// The daily round of `statistics.refresh` (in a function of its own, so `do_daily_job` stays
    /// short).
    pub(super) async fn refresh_statistics_daily(&self, job: &Job) -> Result<(), JobFailure> {
        Box::pin(self.run_daily(JobKind::RefreshStatistics, async {
            self.refresh_statistics().await.map(|refreshed| {
                tracing::info!(refreshed, "the statistics of the database were looked at");
            })
        }))
        .await
        .map_err(|error| {
            tracing::warn!(cause = %error, "the statistics could not be refreshed");
            JobFailure::of(&error, FAILED, job)
        })
    }
}
