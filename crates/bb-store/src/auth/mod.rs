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
mod second_step;
mod sessions;

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
use crate::settings::{
    Scopes, count_policy_version_up, policy_version, put_organization_setting, resolved,
    resolved_in_bounds, resolved_list,
};

/// The sign-in storage: the database, the keys that protect secrets in it, and the seal of the
/// security log.
#[derive(Clone, Debug)]
pub struct AuthStorage {
    database: Database,
    keys: Arc<KeyRing>,
    seal: Arc<LogSeal>,
}

impl AuthStorage {
    /// The storage on `database`.
    #[must_use]
    pub fn new(database: Database, keys: Arc<KeyRing>, seal: LogSeal) -> Self {
        Self {
            database,
            keys,
            seal: Arc::new(seal),
        }
    }

    /// The database underneath.
    #[must_use]
    pub const fn database(&self) -> &Database {
        &self.database
    }
}

/// The hash stored in a row, or the report that it is not one.
pub(super) fn digest(bytes: &[u8]) -> Result<Digest, StoreError> {
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
        ChangeStore::record_changes(&self.database, events, change)
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
            session_idle_minutes: resolved_in_bounds(&SECURITY_SESSION_IDLE_MINUTES, &layers)?,
            session_absolute_hours: resolved_in_bounds(&SECURITY_SESSION_ABSOLUTE_HOURS, &layers)?,
            sign_up_enabled: resolved(&AUTH_SIGN_UP_ENABLED, &layers)?,
            sign_up_domains: resolved_list(&AUTH_SIGN_UP_DOMAINS, &layers)?,
            access_key_default_days: resolved_in_bounds(&AUTH_ACCESS_KEY_DEFAULT_DAYS, &layers)?,
            access_key_longest_days: resolved_in_bounds(&AUTH_ACCESS_KEY_LONGEST_DAYS, &layers)?,
        })
    }

    async fn append_security_log(&self, event: &SecurityEvent) -> Result<(), StoreError> {
        let entry = NewSecurityLogEntry {
            time: event.time,
            person: event.person,
            action: TechnicalName::new(event.action.code())
                .map_err(|_| StoreError::Corrupt("a security action"))?,
            subject: event.subject.clone(),
            network: event.network.clone(),
        };
        self.database
            .append_security_log(&self.seal, &entry)
            .await
            .map(|_| ())
    }

    async fn sign_up_policy_version(&self) -> Result<i64, StoreError> {
        policy_version(&self.database, SIGN_UP_POLICY).await
    }

    async fn count_sign_up_policy_up(
        transaction: &mut Transaction,
        version: i64,
    ) -> Result<bool, StoreError> {
        count_policy_version_up(transaction, SIGN_UP_POLICY, version).await
    }

    async fn put_setting(
        transaction: &mut Transaction,
        key: &str,
        value: &SettingValue,
        at: Timestamp,
    ) -> Result<(), StoreError> {
        put_organization_setting(transaction, key, value, at).await
    }
}

#[cfg(test)]
#[path = "../tests/auth.rs"]
mod tests;
