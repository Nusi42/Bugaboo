//! The one-off jobs that have a function of their own, so `do_one_off_job` stays short.

use super::{AppError, AuthStore, Ctx, FAILED, Job, JobFailure};
use bb_core::job::FailureKind;

impl<S: AuthStore> Ctx<'_, S> {
    /// The job `search-mail.send`: one e-mail of an e-mailed search (QRY-07).
    pub(super) async fn do_search_mail(&self, job: &Job) -> Result<(), JobFailure> {
        Box::pin(self.run_search_mail(job)).await.map_err(|error| {
            // The class only: the answer of the mail server can hold an address.
            tracing::warn!(
                refused = matches!(error, AppError::Refused(_)),
                "an e-mailed search did not go"
            );
            JobFailure::of(&error, FAILED, job)
        })
    }

    /// The job `backups.now`: the backup a person asked for (ADR-0046, amendment of 2026-11-23).
    pub(super) async fn do_backup_now(&self, job: &Job) -> Result<(), JobFailure> {
        Box::pin(self.run_backup_now(job)).await.map_err(|error| {
            tracing::warn!(cause = %error, "a backup that was asked for could not be created");
            // A person waits for it: they are told it failed and may ask again, instead of
            // waiting for the tries of a whole copy of the database. Only a backup of the
            // schedule that runs on this server meanwhile is waited for.
            let failure = JobFailure::of(&error, FAILED, job);
            match error {
                AppError::Refused(bb_core::ErrorCode::BackupRunning) => failure,
                _ => JobFailure {
                    kind: FailureKind::Permanent,
                    ..failure
                },
            }
        })
    }
}
