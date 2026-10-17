//! Keeping and deleting notifications (PRIV-05, M1-F21, ADR-0033 §5): the job `notifications.purge`
//! removes notifications after 90 days, and the payload of a job that died after its own keep
//! period (it can hold the address and the text of a person). Safe to run again. The other jobs
//! that remove what outlived its keep period are in `keeping`.

use bb_core::job::dead_payload_cutoff;
use bb_core::notification::cutoff;

use crate::auth::{AppError, AuthStore, Ctx, internal};

/// What a purge of notifications removed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Purged {
    /// Notifications past their 90 days.
    pub(crate) notifications: u64,
    /// Payloads of dead jobs past their keep period.
    pub(crate) payloads: u64,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Deletes the notifications that outlived 90 days and empties the payloads of the jobs that
    /// died long ago.
    pub(crate) async fn purge_old_notifications(&self) -> Result<Purged, AppError> {
        let failed = |error| internal::<S>(&error);
        let now = self.now();
        let notifications = match cutoff(now) {
            Some(before) => self
                .store()
                .purge_notifications(before)
                .await
                .map_err(failed)?,
            None => 0,
        };
        let payloads = match dead_payload_cutoff(now) {
            Some(before) => self
                .store()
                .clear_dead_job_payloads(before)
                .await
                .map_err(failed)?,
            None => 0,
        };
        Ok(Purged {
            notifications,
            payloads,
        })
    }
}
