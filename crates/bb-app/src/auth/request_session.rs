//! The session behind a request: finding it, keeping it alive, and making a new one.

use bb_auth::{
    Passkeys, Purpose, SecretToken, StoredCredential, looks_like_access_key, same_digest,
};
use bb_core::ErrorCode;
use bb_core::id::{SessionId, UserId};
use bb_core::person::Person;
use bb_core::sign_in::{SessionStage, SignInSession, StoredPasskey};
use zeroize::Zeroizing;

use super::ctx::Ctx;
use super::error::{AppError, internal};
use super::store::{AuthSettings, AuthStore, SessionRecord};
use super::types::{PersonView, Started};

/// A session that only had its password checked lives this long, in seconds.
pub(crate) const PENDING_SECONDS: i64 = 300;
/// A session is marked as used at most this often, in seconds, so reading does not mean writing.
const TOUCH_EVERY_SECONDS: i64 = 60;
/// How long an invitation works, in seconds (7 days).
pub(crate) const INVITATION_SECONDS: i64 = 7 * 24 * 3600;

/// A session that is made but not stored yet: the caller stores it inside its own change.
pub(crate) struct NewSession {
    pub(crate) token: SecretToken,
    pub(crate) session: SignInSession,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The session of the cookie a browser still has when it signs in again, if it is one:
    /// signing in ends it.
    pub(crate) async fn previous_session(
        &self,
        token_text: Option<&str>,
    ) -> Result<Option<SessionId>, AppError> {
        let Some(token) = token_text.and_then(SecretToken::parse) else {
            return Ok(None);
        };
        Ok(self
            .store()
            .session_by_hash(&token.hash(Purpose::Session))
            .await
            .map_err(|error| internal::<S>(&error))?
            .map(|record| record.session.id))
    }

    /// Creates a session for `person`; the caller stores it in its own change.
    pub(crate) fn new_session(
        &self,
        person: UserId,
        stage: SessionStage,
    ) -> Result<NewSession, AppError> {
        let token = SecretToken::generate()?;
        let now = self.now();
        let session = SignInSession {
            id: self.new_session_id(),
            person,
            token_hash: token.hash(Purpose::Session),
            stage,
            created_at: now,
            last_seen_at: now,
            network: self.request.network.clone(),
            device: self.request.device.to_stored(),
        };
        Ok(NewSession { token, session })
    }

    /// What the browser gets for a new session: the two cookie values and who it is for.
    pub(crate) fn started(
        &self,
        new: &NewSession,
        person: &Person,
        settings: &AuthSettings,
        second_step_needed: Option<super::types::SecondStepMethods>,
    ) -> Result<Started, AppError> {
        let seconds = if new.session.stage == SessionStage::Active {
            settings.session_absolute_hours.saturating_mul(3600)
        } else {
            PENDING_SECONDS
        };
        Ok(Started {
            session_token: new.token.encode(),
            csrf_token: self.services.csrf.token(&new.token)?,
            lifetime: std::time::Duration::from_secs(u64::try_from(seconds).unwrap_or(0)),
            person: PersonView::from(person),
            second_step_needed,
        })
    }

    /// The passkey ceremonies, when this server has a host name for them.
    pub(crate) fn passkeys(&self) -> Result<&Passkeys, AppError> {
        self.services
            .passkeys
            .as_ref()
            .ok_or(AppError::Refused(ErrorCode::PasskeyRejected))
    }

    /// The session behind a cookie, if it is of the `wanted` stage and has not run out. A
    /// session that is used is marked as used (at most once a minute).
    pub(crate) async fn session(
        &self,
        token_text: &str,
        wanted: SessionStage,
    ) -> Result<SessionRecord, AppError> {
        // An access key never opens a session: where a session is needed (the owner's own
        // sign-in data) a key is turned away.
        if looks_like_access_key(token_text) {
            return Err(self.key_on_sign_in_data(token_text).await);
        }
        // A cookie that opens no session (garbage, unknown, run out) belongs to one that has
        // ended: the browser is told to forget it. A session at another stage is still waiting.
        let ended = || AppError::SessionEnded;
        let token = SecretToken::parse(token_text).ok_or_else(ended)?;
        let hash = token.hash(Purpose::Session);
        let record = self
            .store()
            .session_by_hash(&hash)
            .await
            .map_err(|error| internal::<S>(&error))?
            .ok_or_else(ended)?;
        // The lookup is by hash; this comparison is for the record that came back.
        if !same_digest(&record.session.token_hash, &hash) {
            return Err(ended());
        }
        if record.session.stage != wanted {
            return Err(AppError::Refused(ErrorCode::NotSignedIn));
        }
        let now = self.now();
        let age = now
            .as_second()
            .saturating_sub(record.session.created_at.as_second());
        let idle = now
            .as_second()
            .saturating_sub(record.session.last_seen_at.as_second());
        let (absolute, idle_limit) = if wanted == SessionStage::Active {
            let settings = self.settings().await?;
            (
                settings.session_absolute_hours.saturating_mul(3600),
                settings.session_idle_minutes.saturating_mul(60),
            )
        } else {
            (PENDING_SECONDS, PENDING_SECONDS)
        };
        if age > absolute || idle > idle_limit {
            return Err(ended());
        }
        if idle >= TOUCH_EVERY_SECONDS {
            self.store()
                .touch_session(record.session.id, now)
                .await
                .map_err(|error| internal::<S>(&error))?;
        }
        Ok(record)
    }

    /// The signed-in person behind a cookie, who may use their session for anything: a person
    /// who still has to set up a second step may not (it counts from this request on, also for a
    /// session that was open before: ADR-0025, decision 7).
    ///
    /// # Errors
    /// `auth.second_step_required` while the person has to set up a second step.
    pub(crate) async fn active(&self, token_text: &str) -> Result<SessionRecord, AppError> {
        let record = self.session(token_text, SessionStage::Active).await?;
        if self.must_set_up_second_step(record.person.id).await? {
            return Err(AppError::Refused(ErrorCode::SecondStepRequired));
        }
        Ok(record)
    }

    /// The signed-in person behind a cookie, whether or not they still have to set up a second
    /// step: only what sets one up (and says that it has to be) uses this.
    pub(crate) async fn active_to_set_up(
        &self,
        token_text: &str,
    ) -> Result<SessionRecord, AppError> {
        self.session(token_text, SessionStage::Active).await
    }

    /// Checks the password a signed-in person types again before a sensitive change. Wrong
    /// guesses count against the person, like the second step.
    pub(crate) async fn confirm_password(
        &self,
        person: &Person,
        password: String,
    ) -> Result<(), AppError> {
        // Its own key: only a finished second step clears the key of the second step.
        let slot = self.reserve(format!("password:{}", person.id))?;
        let checked = async {
            let account = self
                .store()
                .account_by_email(&person.email)
                .await
                .map_err(|error| internal::<S>(&error))?;
            let stored = account.and_then(|account| account.password_hash);
            Ok(self
                .services
                .hasher
                .verify(Zeroizing::new(password), stored)
                .await?)
        }
        .await;
        let verdict = self.unless_internal(&slot, checked)?;
        if verdict.matches {
            self.slot_succeeded(slot);
            Ok(())
        } else {
            self.slot_failed(slot, Some(person.id)).await;
            Err(AppError::Refused(ErrorCode::WrongCredentials))
        }
    }

    /// The passkeys of a person in the form the passkey library takes.
    pub(crate) async fn credentials(
        &self,
        person: UserId,
    ) -> Result<(Vec<StoredPasskey>, Vec<StoredCredential>), AppError> {
        let stored = self
            .store()
            .passkeys_of(person)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let credentials = stored
            .iter()
            .map(|passkey| StoredCredential {
                credential_id: passkey.credential_id.clone(),
                data: passkey.data.clone(),
            })
            .collect();
        Ok((stored, credentials))
    }
}
