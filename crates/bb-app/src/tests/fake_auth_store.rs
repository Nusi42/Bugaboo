//! The in-memory store as the use cases of signing in see it as a whole (`AuthStore`): the errors of
//! a change, the settings of signing in, the security log and the version of the sign-up policy.

use bb_core::settings::{
    AUTH_ACCESS_KEY_DEFAULT_DAYS, AUTH_ACCESS_KEY_LONGEST_DAYS, AUTH_SIGN_UP_DOMAINS,
    AUTH_SIGN_UP_ENABLED, Integer, SECURITY_SESSION_ABSOLUTE_HOURS, SECURITY_SESSION_IDLE_MINUTES,
    Scope, Setting, SettingValue, Text,
};
use bb_core::version::Version;
use bb_policy::guards::Refusal;
use jiff::Timestamp;

use crate::auth::{AuthSettings, AuthStore, SecurityEvent};

use super::fake::*;

impl AuthStore for FakeStore {
    fn is_duplicate(error: &FakeError) -> bool {
        *error == FakeError::Duplicate
    }

    fn aborted() -> FakeError {
        FakeError::Aborted
    }

    fn is_aborted(error: &FakeError) -> bool {
        *error == FakeError::Aborted
    }

    fn is_contended(error: &FakeError) -> bool {
        *error == FakeError::Contended
    }

    fn is_busy(error: &FakeError) -> bool {
        *error == FakeError::Busy
    }

    fn refused(refusal: Refusal) -> FakeError {
        FakeError::Refused(refusal)
    }

    fn as_refusal(error: &FakeError) -> Option<Refusal> {
        match error {
            FakeError::Refused(refusal) => Some(*refusal),
            _ => None,
        }
    }

    fn describe(error: &FakeError) -> String {
        error.to_string()
    }

    async fn auth_settings(&self) -> Result<AuthSettings, FakeError> {
        Ok(self.read(|state| {
            let layers = super::fake_settings::layers_of(state, None, None);
            let number = |setting: &Setting<Integer>| {
                setting.resolve(Some(&layers.resolve(setting.definition()).value))
            };
            let text = |setting: &Setting<Text>| {
                setting.resolve(Some(&layers.resolve(setting.definition()).value))
            };
            AuthSettings {
                session_idle_minutes: number(&SECURITY_SESSION_IDLE_MINUTES).unwrap_or(30),
                session_absolute_hours: number(&SECURITY_SESSION_ABSOLUTE_HOURS).unwrap_or(24),
                sign_up_enabled: AUTH_SIGN_UP_ENABLED
                    .resolve(Some(
                        &layers.resolve(AUTH_SIGN_UP_ENABLED.definition()).value,
                    ))
                    .unwrap_or(false),
                sign_up_domains: text(&AUTH_SIGN_UP_DOMAINS).unwrap_or_default(),
                access_key_default_days: number(&AUTH_ACCESS_KEY_DEFAULT_DAYS).unwrap_or(90),
                access_key_longest_days: number(&AUTH_ACCESS_KEY_LONGEST_DAYS).unwrap_or(365),
            }
        }))
    }

    async fn append_security_log(&self, event: &SecurityEvent) -> Result<(), FakeError> {
        let mut shared = self.lock();
        if shared.log_fails {
            return Err(FakeError::Failed);
        }
        shared.log.push(event.clone());
        Ok(())
    }

    async fn prune_security_log(
        &self,
        before: Timestamp,
        _now: Timestamp,
    ) -> Result<u64, FakeError> {
        let mut shared = self.lock();
        let newest = shared.log.last().map(|event| event.time);
        let kept = shared.log.len();
        shared
            .log
            .retain(|event| event.time >= before || Some(event.time) == newest);
        Ok(u64::try_from(kept - shared.log.len()).unwrap_or(u64::MAX))
    }

    /// The fake keeps no statistics.
    async fn refresh_statistics(&self) -> Result<bool, FakeError> {
        Ok(false)
    }

    async fn sign_up_policy_version(&self) -> Result<i64, FakeError> {
        Ok(self.read(|state| state.sign_up_version))
    }

    async fn count_sign_up_policy_up(tx: &mut Tx, version: Version) -> Result<bool, FakeError> {
        if tx.state.sign_up_version != version.get() {
            return Ok(false);
        }
        tx.state.sign_up_version += 1;
        Ok(true)
    }

    async fn put_setting(
        tx: &mut Tx,
        key: &str,
        value: &SettingValue,
        _at: Timestamp,
    ) -> Result<(), FakeError> {
        super::fake_settings::put(&mut tx.state, Scope::Organization, key, value);
        Ok(())
    }
}
