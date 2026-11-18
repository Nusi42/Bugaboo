//! Keeping and deleting notifications (PRIV-05, ADR-0033 §5, ADR-0045): the job
//! `notifications.purge` removes notifications after their keep period, one step at a time (the
//! shape of `keeping`). Safe to run again.

use bb_core::keeping::KeepKind;

use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::keeping::KEEP_STEP_ROWS;

impl<S: AuthStore> Ctx<'_, S> {
    /// Deletes the notifications that outlived their keep period: one step. Returns how many went.
    pub(crate) async fn purge_old_notifications(&self) -> Result<u64, AppError> {
        let before = self.keep_cutoff(KeepKind::Notifications).await?;
        self.store()
            .purge_notifications(before, KEEP_STEP_ROWS)
            .await
            .map_err(|error| internal::<S>(&error))
    }
}
