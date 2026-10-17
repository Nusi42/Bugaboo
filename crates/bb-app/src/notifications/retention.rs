//! Keeping and deleting (PRIV-05, M1-F21, M1-F43, M1-F61, ADR-0033 §5): the jobs that remove what
//! outlived its keep period. Each is safe to run again, and each has its test of idempotency.
//!
//! - `notifications.purge`: notifications after 90 days, and the payload of a job that died after
//!   its own keep period (it can hold the address and the text of a person).
//! - `security-log.prune`: the oldest whole blocks of the security log past its keep setting.
//! - `sessions.purge`: the sessions nobody can use any more.
//! - `server-log.rotate`: a new file of the log of a server that runs as a service.

use bb_core::event::{Actor as EventActor, RecordKind};
use bb_core::job::dead_payload_cutoff;
use bb_core::notification::cutoff;
use bb_core::settings::{LOGS_SERVER_LOG_FILES, LOGS_SERVER_LOG_MEGABYTES, SECURITY_LOG_KEEP_DAYS};
use bb_core::time::SECONDS_PER_DAY;
use jiff::{SignedDuration, Timestamp};

use super::prefs::number;
use crate::auth::{
    AppError, AuthStore, Ctx, Stops, change_failed, changed, fixed_entity, internal, purge_sessions,
};

/// The bytes in a megabyte, for the size of the server log.
const MEGABYTE: u64 = 1_048_576;

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

    /// Removes the sessions nobody can use any more, and the links of sign-ups that were used or
    /// ran out. A change of Bugaboo itself, so it writes an event.
    pub(crate) async fn purge_dead_sessions(&self) -> Result<(), AppError> {
        self.delete_old_sign_up_links().await?;
        let settings = self.settings().await?;
        let event = self.event(
            EventActor::System,
            fixed_entity(RecordKind::Session, "purge")?,
            vec![changed("purged")],
        )?;
        let now = self.now();
        self.store()
            .record_change(&event, move |transaction| {
                Box::pin(async move { purge_sessions::<S>(transaction, &settings, now).await })
            })
            .await
            .map_err(|error| change_failed::<S>(&error, Stops::NONE))
    }

    /// Deletes the oldest whole blocks of the security log past its keep setting. Returns how many
    /// entries went.
    pub(crate) async fn prune_old_security_log(&self) -> Result<u64, AppError> {
        let layers = self.setting_layers_of(None).await?;
        let days = number(&layers, &SECURITY_LOG_KEEP_DAYS)
            .unwrap_or(365)
            .max(1);
        let now = self.now();
        let before: Timestamp = now
            .checked_sub(SignedDuration::from_secs(
                days.saturating_mul(SECONDS_PER_DAY),
            ))
            .map_err(|error| AppError::Internal(error.to_string()))?;
        self.store()
            .prune_security_log(before, now)
            .await
            .map_err(|error| internal::<S>(&error))
    }

    /// Starts a new file of the log of a server that runs without a console, when the file is
    /// big. Returns whether it did. Nothing happens on a server that logs to its console.
    pub(crate) async fn rotate_old_server_log(&self) -> Result<bool, AppError> {
        let Some(file) = &self.services.server_log else {
            return Ok(false);
        };
        let layers = self.setting_layers_of(None).await?;
        let megabytes = number(&layers, &LOGS_SERVER_LOG_MEGABYTES).unwrap_or(20);
        let files = number(&layers, &LOGS_SERVER_LOG_FILES).unwrap_or(5);
        file.rotate(
            u64::try_from(megabytes)
                .unwrap_or(1)
                .saturating_mul(MEGABYTE),
            u32::try_from(files).unwrap_or(1),
        )
        .map_err(|error| AppError::Internal(error.to_string()))
    }
}
