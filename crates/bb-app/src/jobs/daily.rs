//! The daily rounds that have a function of their own, so `do_daily_job` stays short.

use super::{AuthStore, Ctx, FAILED, Job, JobFailure, JobKind};

impl<S: AuthStore> Ctx<'_, S> {
    /// The daily round of `oauth.clean-old-grants` (in a function of its own, so `do_daily_job`
    /// stays short).
    pub(super) async fn clean_old_grants_daily(&self, job: &Job) -> Result<(), JobFailure> {
        Box::pin(self.run_daily(JobKind::CleanOldGrants, async {
            crate::auth::clean_old_grants(self).await.map(|cleaned| {
                tracing::info!(
                    secrets = cleaned.secrets,
                    grants = cleaned.grants,
                    clients = cleaned.clients,
                    "old grants of AI apps were cleared"
                );
            })
        }))
        .await
        .map_err(|error| {
            tracing::warn!(cause = %error, "old grants of AI apps could not be cleared");
            JobFailure::of(&error, FAILED, job)
        })
    }

    /// The daily round of `sessions.purge` (in a function of its own, so `do_daily_job` stays
    /// short).
    pub(super) async fn purge_sessions_daily(&self, job: &Job) -> Result<(), JobFailure> {
        Box::pin(self.run_daily(JobKind::PurgeSessions, async {
            self.purge_dead_sessions().await.map(|()| {
                tracing::info!("sessions nobody can use were deleted");
            })
        }))
        .await
        .map_err(|error| {
            tracing::warn!(cause = %error, "old sessions could not be deleted");
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
