//! Keeping and deleting (PRIV-05, M1-F21, M1-F43, M1-F61, ADR-0017): the jobs that remove what
//! outlived its keep period. Each is safe to run again, and each has its test of idempotency. The
//! notifications and the payloads of dead jobs are in `notifications/keep.rs`.
//!
//! - `security-log.prune`: the oldest whole blocks of the security log past its keep setting.
//! - `sessions.purge`: the sessions nobody can use any more, and the links of finished sign-ups.
//! - `server-log.rotate`: a new file of the log of a server that runs as a service.

use bb_core::event::{Actor as EventActor, RecordKind};
use bb_core::settings::{LOGS_SERVER_LOG_FILES, LOGS_SERVER_LOG_MEGABYTES, SECURITY_LOG_KEEP_DAYS};
use jiff::Timestamp;

use crate::auth::{
    AppError, AuthStore, Ctx, Stops, change_failed, changed, fixed_entity, internal, purge_sessions,
};

/// The bytes in a megabyte, for the size of the server log.
const MEGABYTE: u64 = 1_048_576;

impl<S: AuthStore> Ctx<'_, S> {
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
        let days = layers.number(&SECURITY_LOG_KEEP_DAYS);
        let now = self.now();
        let before: Timestamp = now
            .checked_sub(bb_core::time::days(days))
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
        let megabytes = layers.number(&LOGS_SERVER_LOG_MEGABYTES);
        let files = layers.number(&LOGS_SERVER_LOG_FILES);
        file.rotate(
            u64::try_from(megabytes)
                .unwrap_or(1)
                .saturating_mul(MEGABYTE),
            u32::try_from(files).unwrap_or(1),
        )
        .map_err(|error| AppError::Internal(error.to_string()))
    }
}
