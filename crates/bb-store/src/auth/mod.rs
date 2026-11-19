//! The adapter of the sign-in port (`bb_app::auth::AuthStore`): people, passwords, sessions,
//! second steps, passkeys, invitations and the setup code.
//!
//! Writes are associated functions that run inside the transaction of a change
//! ([`Database::record_change`]), so a change and its event are committed together. The few
//! reads and writes that are not domain changes use the pool directly: marking a session as used,
//! the encrypted code app secret and the security log.

mod access_key_records;
mod accounts;
mod invitations;
pub(crate) mod replay_waiting;
mod second_step;
mod sessions;
mod sign_up;
pub(crate) mod standing;

pub(crate) use accounts::{person_columns, read_person};
use bb_core::version::Version;
use std::sync::Arc;

use bb_app::auth::{AuthSettings, AuthStore, SecurityEvent};
use bb_app::ports::{BoxFuture, ChangeStore};
use bb_core::event::DomainEvent;
use bb_core::security_log::Digest;
use bb_core::settings::{
    AUTH_ACCESS_KEY_DEFAULT_DAYS, AUTH_ACCESS_KEY_LONGEST_DAYS, AUTH_SIGN_UP_DOMAINS,
    AUTH_SIGN_UP_ENABLED, SECURITY_SESSION_ABSOLUTE_HOURS, SECURITY_SESSION_IDLE_MINUTES,
    SIGN_UP_POLICY, SettingValue,
};
use bb_core::technical::TechnicalName;
use jiff::Timestamp;

use crate::anchor::LogSeal;
use crate::db::{Database, Transaction};
use crate::error::StoreError;
use crate::keys::KeyRing;
use crate::security_log::NewSecurityLogEntry;
use crate::settings::{Scopes, count_policy_version_up, policy_version, put_organization_setting};

/// The sign-in storage: the database, the keys that protect secrets in it, and the seal of the
/// security log.
#[derive(Clone, Debug)]
pub struct AuthStorage {
    database: Database,
    keys: Arc<KeyRing>,
    seal: Arc<LogSeal>,
    /// The sessions that are being marked as used (`sessions.rs`).
    touches: Arc<sessions::Touches>,
}

impl AuthStorage {
    /// The storage on `database`.
    #[must_use]
    pub fn new(database: Database, keys: Arc<KeyRing>, seal: LogSeal) -> Self {
        Self {
            database,
            keys,
            seal: Arc::new(seal),
            touches: Arc::default(),
        }
    }

    /// The database underneath.
    #[must_use]
    pub const fn database(&self) -> &Database {
        &self.database
    }

    /// The keys that protect the secrets in the database.
    pub(crate) fn key_ring(&self) -> &KeyRing {
        &self.keys
    }
}

/// The hash stored in a row, or the report that it is not one.
pub(crate) fn digest(bytes: &[u8]) -> Result<Digest, StoreError> {
    Digest::from_slice(bytes).ok_or(StoreError::Corrupt("a hash"))
}

impl ChangeStore for AuthStorage {
    type Transaction = Transaction;
    type Error = StoreError;

    fn record_changes<'a, T: Send + 'a>(
        &'a self,
        events: &'a [DomainEvent],
        change: impl for<'t> FnOnce(&'t mut Transaction) -> BoxFuture<'t, Result<T, StoreError>>
        + Send
        + 'a,
    ) -> impl Future<Output = Result<T, StoreError>> + Send + 'a {
        // The entries of the security log a change added are appended at its end, under the lock
        // of the end of the log, and committed with it; the anchor file follows after the commit.
        let seal = &*self.seal;
        async move {
            let (result, head) = self
                .database
                .record_signed_changes(events, Some(seal.key()), async move |transaction| {
                    change(transaction).await
                })
                .await?;
            // The file one entry behind the database is harmless (`crate::anchor`); the change
            // stands.
            if let Some((seq, hash)) = head
                && let Err(error) = seal.record(seq, &hash)
            {
                tracing::warn!(cause = %error, "the anchor file of the security log could not be written");
            }
            Ok(result)
        }
    }

    fn read_snapshot<'a, T: Send + 'a>(
        &'a self,
        read: impl for<'t> FnOnce(&'t mut Transaction) -> BoxFuture<'t, Result<T, StoreError>>
        + Send
        + 'a,
    ) -> impl Future<Output = Result<T, StoreError>> + Send + 'a {
        ChangeStore::read_snapshot(&self.database, read)
    }
}

impl AuthStore for AuthStorage {
    fn is_duplicate(error: &StoreError) -> bool {
        error.is_duplicate()
    }

    fn aborted() -> StoreError {
        StoreError::Aborted
    }

    fn is_aborted(error: &StoreError) -> bool {
        matches!(error, StoreError::Aborted)
    }

    fn is_contended(error: &StoreError) -> bool {
        matches!(error, StoreError::Contended)
    }

    fn is_busy(error: &StoreError) -> bool {
        matches!(error, StoreError::Busy)
    }

    fn refused(refusal: bb_policy::guards::Refusal) -> StoreError {
        StoreError::Refused(refusal)
    }

    fn as_refusal(error: &StoreError) -> Option<bb_policy::guards::Refusal> {
        match error {
            StoreError::Refused(refusal) => Some(*refusal),
            _ => None,
        }
    }

    fn describe(error: &StoreError) -> String {
        error.to_string()
    }

    async fn auth_settings(&self) -> Result<AuthSettings, StoreError> {
        // One snapshot of the organization's values: every request that needs a session asks.
        let layers = self.database.setting_layers(&Scopes::default()).await?;
        // Whole numbers and the list are cut to their bounds: a value that was stored before the
        // bounds, or by hand, never reaches a use case.
        Ok(AuthSettings {
            session_idle_minutes: layers.number(&SECURITY_SESSION_IDLE_MINUTES),
            session_absolute_hours: layers.number(&SECURITY_SESSION_ABSOLUTE_HOURS),
            sign_up_enabled: layers.yes(&AUTH_SIGN_UP_ENABLED),
            sign_up_domains: layers.list(&AUTH_SIGN_UP_DOMAINS),
            access_key_default_days: layers.number(&AUTH_ACCESS_KEY_DEFAULT_DAYS),
            access_key_longest_days: layers.number(&AUTH_ACCESS_KEY_LONGEST_DAYS),
        })
    }

    async fn append_security_log(&self, event: &SecurityEvent) -> Result<(), StoreError> {
        self.database
            .append_security_log(&self.seal, &new_entry(event)?)
            .await
            .map(|_| ())
    }

    async fn prune_security_log(
        &self,
        before: Timestamp,
        now: Timestamp,
        limit: u64,
    ) -> Result<u64, StoreError> {
        self.database
            .prune_security_log(&self.seal, before, now, limit)
            .await
    }

    async fn refresh_statistics(&self) -> Result<bool, StoreError> {
        self.database.refresh_statistics().await
    }

    async fn sign_up_policy_version(&self) -> Result<i64, StoreError> {
        policy_version(&self.database, SIGN_UP_POLICY).await
    }

    async fn count_sign_up_policy_up(
        transaction: &mut Transaction,
        version: Version,
    ) -> Result<bool, StoreError> {
        count_policy_version_up(transaction, SIGN_UP_POLICY, version.get()).await
    }

    async fn put_setting(
        transaction: &mut Transaction,
        key: &str,
        value: &SettingValue,
        at: Timestamp,
    ) -> Result<(), StoreError> {
        put_organization_setting(transaction, key, value, at).await
    }

    async fn append_security_log_in_change(
        transaction: &mut Transaction,
        event: &SecurityEvent,
    ) -> Result<(), StoreError> {
        // Signed and appended by `record_changes` at the end of the change.
        transaction.security_log.push(new_entry(event)?);
        Ok(())
    }
}

/// The entry of the security log that `event` is.
fn new_entry(event: &SecurityEvent) -> Result<NewSecurityLogEntry, StoreError> {
    Ok(NewSecurityLogEntry {
        time: event.time,
        person: event.person,
        action: TechnicalName::new(event.action.code())
            .map_err(|_| StoreError::Corrupt("a security action"))?,
        subject: event.subject.clone(),
        network: event.network.clone(),
    })
}

#[cfg(test)]
#[path = "../tests/auth.rs"]
mod tests;
