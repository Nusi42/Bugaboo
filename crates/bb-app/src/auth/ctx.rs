//! What every sign-in use case has at hand: the store, the clock, the password hasher, passkeys,
//! the CSRF key and the limit on failed tries, plus what is known about the request.
//!
//! Helpers that need the context are methods of [`Ctx`], not free functions, so the only free
//! `async fn`s that take a `Ctx` are the use cases themselves (ARCH-EVENTS reads them).

use std::fmt;
use std::sync::Arc;

use bb_auth::{
    CsrfKey, Device, Passkeys, PasswordHasher, Purpose, SecretToken, StoredCredential, same_digest,
};
use bb_core::ErrorCode;
use bb_core::event::{Actor, Channel, DomainEvent, EntityKind, EntityRef, FieldChange};
use bb_core::id::{EntityId, EventId, IdSource, SessionId, UserId};
use bb_core::network::NetworkPrefix;
use bb_core::person::Person;
use bb_core::sign_in::{SecurityAction, SessionStage, SignInSession, StoredPasskey};
use jiff::Timestamp;
use zeroize::Zeroizing;

use super::store::{AuthSettings, AuthStore, SecurityEvent, SessionRecord};
use super::types::{PersonView, Started};
use crate::context::{Clock, SystemIds};

/// A session that only had its password checked lives this long, in seconds.
pub(crate) const PENDING_SECONDS: i64 = 300;
/// A session is marked as used at most this often, in seconds, so reading does not mean writing.
const TOUCH_EVERY_SECONDS: i64 = 60;
/// How long an invitation works, in seconds (7 days).
pub(crate) const INVITATION_SECONDS: i64 = 7 * 24 * 3600;

/// Why a use case did not do what was asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppError {
    /// The request was turned away for a reason a person may be told (an error code).
    Refused(ErrorCode),
    /// Too many failed tries: wait this many seconds.
    Limited {
        /// Seconds until the next try may work.
        retry_after: u64,
    },
    /// Something failed inside the server. The text is for the log only.
    Internal(String),
}

impl From<ErrorCode> for AppError {
    fn from(code: ErrorCode) -> Self {
        Self::Refused(code)
    }
}

impl From<bb_auth::AuthError> for AppError {
    fn from(error: bb_auth::AuthError) -> Self {
        Self::Internal(error.to_string())
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Refused(code) => write!(formatter, "refused: {code}"),
            Self::Limited { retry_after } => {
                write!(formatter, "limited for {retry_after} seconds")
            }
            Self::Internal(cause) => write!(formatter, "failed inside the server: {cause}"),
        }
    }
}

impl std::error::Error for AppError {}

/// The limit on failed tries (SEC-14). A try is **reserved before it is checked** and counts as a
/// failure from that moment, so requests that arrive at the same time cannot all slip through
/// before the first failure is written down. A success gives the reservation back, so only real
/// failures stay counted: nobody can lock another person out by merely trying their address
/// (M1-F27). A key that keeps using up its allowance waits longer each time, up to a cap, and
/// never for good. The HTTP layer implements it in memory.
pub trait FailureLimit: Send + Sync {
    /// Reserves one try of `key`. `Ok(true)` when this try used up the allowance (the next one
    /// waits).
    ///
    /// # Errors
    /// The seconds to wait when the key has used up its allowance.
    fn reserve(&self, key: &str) -> Result<bool, u64>;

    /// Gives one reserved try back (a success on a key that many people share, like a network).
    fn release(&self, key: &str);

    /// Clears the key (the person signed in).
    fn clear(&self, key: &str);

    /// Counts a try of `key` without a window and returns the count so far (the wrong codes of
    /// one half-finished sign-in). `u32::MAX` when the count cannot be kept.
    fn tally(&self, key: &str) -> u32;

    /// Forgets the count of `key`.
    fn forget(&self, key: &str);
}

/// What is known about the request that is being served.
#[derive(Clone, Debug, Default)]
pub struct RequestInfo {
    /// The client network, shortened.
    pub network: Option<NetworkPrefix>,
    /// The browser and the system.
    pub device: Device,
}

/// Everything the sign-in use cases share. Built once at start-up (`bb-server`).
pub struct AuthServices<S> {
    pub(crate) store: S,
    pub(crate) clock: Arc<dyn Clock + Send + Sync>,
    pub(crate) hasher: PasswordHasher,
    pub(crate) passkeys: Option<Passkeys>,
    pub(crate) csrf: CsrfKey,
    pub(crate) limiter: Arc<dyn FailureLimit>,
    pub(crate) product_name: String,
}

impl<S> AuthServices<S> {
    /// The services around `store`. `product_name` is what code apps call this server. Without
    /// `passkeys` (the public address has no host name, which browsers need for passkeys) the
    /// passkey routes refuse.
    pub fn new(
        store: S,
        clock: Arc<dyn Clock + Send + Sync>,
        hasher: PasswordHasher,
        passkeys: Option<Passkeys>,
        csrf: CsrfKey,
        limiter: Arc<dyn FailureLimit>,
        product_name: String,
    ) -> Self {
        Self {
            store,
            clock,
            hasher,
            passkeys,
            csrf,
            limiter,
            product_name,
        }
    }

    /// The context of one request.
    #[must_use]
    pub const fn ctx(&self, request: RequestInfo) -> Ctx<'_, S> {
        Ctx {
            services: self,
            request,
        }
    }

    /// Whether `presented` (the `x-csrf-token` header) is the token of the session behind
    /// `session_token` (the cookie). Anything that is not a session secret is `false`.
    #[must_use]
    pub fn csrf_matches(&self, session_token: &str, presented: &str) -> bool {
        SecretToken::parse(session_token)
            .is_some_and(|session| self.csrf.verify(&session, presented))
    }
}

/// The services plus the request, which is what a use case takes.
pub struct Ctx<'a, S> {
    pub(crate) services: &'a AuthServices<S>,
    pub(crate) request: RequestInfo,
}

/// Makes IDs from the clock of the context and the secure random source.
struct ClockIds<'a>(&'a (dyn Clock + Send + Sync));

impl IdSource for ClockIds<'_> {
    fn now(&mut self) -> Timestamp {
        self.0.now()
    }

    fn random(&mut self) -> [u8; 16] {
        SystemIds.random()
    }
}

/// A reserved try (see [`FailureLimit`]).
pub(crate) struct Slot {
    key: String,
    network: Option<String>,
    tipped: bool,
}

/// How many wrong codes a half-finished sign-in takes before it is ended.
pub(crate) const SECOND_STEP_TRIES: u32 = 10;

/// A session that is made but not stored yet: the caller stores it inside its own change.
pub(crate) struct NewSession {
    pub(crate) token: SecretToken,
    pub(crate) session: SignInSession,
}

impl<S: AuthStore> Ctx<'_, S> {
    pub(crate) const fn store(&self) -> &S {
        &self.services.store
    }

    pub(crate) fn now(&self) -> Timestamp {
        self.services.clock.now()
    }

    /// Fresh IDs.
    pub(crate) fn new_session_id(&self) -> SessionId {
        SessionId::generate(&mut ClockIds(self.services.clock.as_ref()))
    }

    /// Fresh IDs of any kind.
    pub(crate) fn ids(&self) -> impl IdSource + '_ {
        ClockIds(self.services.clock.as_ref())
    }

    /// The domain event of a change by `actor` to the record `entity`. Credentials never go in
    /// `changes`.
    pub(crate) fn event(
        &self,
        actor: Actor,
        entity: EntityRef,
        changes: Vec<FieldChange>,
    ) -> Result<DomainEvent, AppError> {
        let mut ids = ClockIds(self.services.clock.as_ref());
        Ok(DomainEvent {
            id: EventId::generate(&mut ids)
                .map_err(|error| AppError::Internal(error.to_string()))?,
            time: self.now(),
            actor,
            channel: Channel::Ui,
            entity,
            changes,
            reason: None,
        })
    }

    /// Writes an entry into the security log. A log that cannot be written is reported in the
    /// server log but does not undo or refuse what already happened (decision for review,
    /// SEC-15).
    pub(crate) async fn note(
        &self,
        action: SecurityAction,
        person: Option<UserId>,
        subject: Option<EntityRef>,
    ) {
        let subject_kind = subject
            .as_ref()
            .map(|subject| subject.kind.as_str().to_owned());
        let subject_id = subject.as_ref().map(|subject| subject.id);
        let event = SecurityEvent {
            time: self.now(),
            person,
            action,
            subject,
            network: self.request.network.clone(),
        };
        if let Err(error) = self.store().append_security_log(&event).await {
            // IDs only: who and what it was about, never a secret or a name.
            tracing::error!(
                action = action.code(),
                person = ?person,
                subject_kind = subject_kind.as_deref(),
                subject_id = ?subject_id,
                cause = %S::describe(&error),
                "the security log could not be written"
            );
        }
    }

    /// The settings sign-in depends on.
    pub(crate) async fn settings(&self) -> Result<AuthSettings, AppError> {
        self.store()
            .auth_settings()
            .await
            .map_err(|error| internal::<S>(&error))
    }

    /// The client network as text, for the pools of waiting passkey challenges.
    pub(crate) fn network_name(&self) -> Option<String> {
        self.request.network.as_ref().map(ToString::to_string)
    }

    /// Reserves a try of `key` (an account, a person, a setup) and of the client network. The
    /// try counts as a failure until [`Self::slot_succeeded`] says otherwise.
    pub(crate) fn reserve(&self, key: String) -> Result<Slot, AppError> {
        let limiter = &self.services.limiter;
        let wait_error = |retry_after| AppError::Limited { retry_after };
        let network = self
            .request
            .network
            .as_ref()
            .map(|network| format!("net:{network}"));
        let network_tipped = match &network {
            Some(network) => limiter.reserve(network).map_err(wait_error)?,
            None => false,
        };
        match limiter.reserve(&key) {
            Ok(tipped) => Ok(Slot {
                key,
                network,
                tipped: tipped || network_tipped,
            }),
            Err(wait) => {
                if let Some(network) = &network {
                    limiter.release(network);
                }
                Err(wait_error(wait))
            }
        }
    }

    /// The reserved try was a failure. When it used up an allowance, that is written down once.
    pub(crate) async fn slot_failed(&self, slot: Slot, person: Option<UserId>) {
        if slot.tipped {
            self.note(SecurityAction::SignInLimited, person, None).await;
        }
    }

    /// The reserved try was no failure: the account is cleared, the network gets its try back.
    pub(crate) fn slot_succeeded(&self, slot: Slot) {
        let Slot { key, network, .. } = slot;
        self.services.limiter.clear(&key);
        if let Some(network) = &network {
            self.services.limiter.release(network);
        }
    }

    /// A wrong try at the second step of the half-finished sign-in `pending`. After
    /// [`SECOND_STEP_TRIES`] wrong tries the sign-in is ended and that is written down: whoever
    /// has the password has to start again, and the password steps have their own limit.
    pub(crate) async fn second_step_failed(
        &self,
        slot: Slot,
        pending: &SessionRecord,
        action: SecurityAction,
    ) -> Result<(), AppError> {
        let person = pending.person.id;
        self.slot_failed(slot, Some(person)).await;
        self.note(action, Some(person), None).await;
        let key = format!("pending:{}", pending.session.id);
        if self.services.limiter.tally(&key) < SECOND_STEP_TRIES {
            return Ok(());
        }
        self.services.limiter.forget(&key);
        let id = pending.session.id;
        let event = self.event(
            Actor::Person(person),
            entity("session", id)?,
            vec![changed("session")],
        )?;
        self.store()
            .record_change(&event, move |transaction| {
                Box::pin(async move { S::delete_session(transaction, id).await.map(|_| ()) })
            })
            .await
            .map_err(|error| change_failed::<S>(&error, None))?;
        self.note(
            SecurityAction::SecondStepGivenUp,
            Some(person),
            Some(entity("session", id)?),
        )
        .await;
        Ok(())
    }

    /// The second step was done: the reserved try is no failure and the count of wrong tries goes.
    pub(crate) fn second_step_done(&self, slot: Slot, pending: &SessionRecord) {
        self.services
            .limiter
            .forget(&format!("pending:{}", pending.session.id));
        self.slot_succeeded(slot);
    }

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

    /// Makes a session for `person`; the caller stores it in its own change.
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
        let not_signed_in = || AppError::Refused(ErrorCode::NotSignedIn);
        let token = SecretToken::parse(token_text).ok_or_else(not_signed_in)?;
        let hash = token.hash(Purpose::Session);
        let record = self
            .store()
            .session_by_hash(&hash)
            .await
            .map_err(|error| internal::<S>(&error))?
            .ok_or_else(not_signed_in)?;
        // The lookup is by hash; this comparison is for the record that came back.
        if !same_digest(&record.session.token_hash, &hash) || record.session.stage != wanted {
            return Err(not_signed_in());
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
            return Err(not_signed_in());
        }
        if idle >= TOUCH_EVERY_SECONDS {
            self.store()
                .touch_session(record.session.id, now)
                .await
                .map_err(|error| internal::<S>(&error))?;
        }
        Ok(record)
    }

    /// The signed-in person behind a cookie.
    pub(crate) async fn active(&self, token_text: &str) -> Result<SessionRecord, AppError> {
        self.session(token_text, SessionStage::Active).await
    }

    /// The signed-in administrator behind a cookie.
    pub(crate) async fn administrator(&self, token_text: &str) -> Result<SessionRecord, AppError> {
        let record = self.active(token_text).await?;
        if record.person.administrator {
            Ok(record)
        } else {
            Err(AppError::Refused(ErrorCode::Forbidden))
        }
    }

    /// Checks the password a signed-in person types again before a sensitive change. Wrong
    /// guesses count against the person, like the second step.
    pub(crate) async fn confirm_password(
        &self,
        person: &Person,
        password: String,
    ) -> Result<(), AppError> {
        let slot = self.reserve(person_key(person.id))?;
        let account = self
            .store()
            .account_by_email(&person.email)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let stored = account.and_then(|account| account.password_hash);
        let verdict = self
            .services
            .hasher
            .verify(Zeroizing::new(password), stored)
            .await?;
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

/// The answer for a failure of the store: the person gets the standard answer and the cause goes
/// to the log.
pub(crate) fn internal<S: AuthStore>(error: &S::Error) -> AppError {
    AppError::Internal(S::describe(error))
}

/// The answer for a change that failed. A change that stopped itself (a check inside the
/// transaction failed) or hit a duplicate key (two people at once) is told apart by the caller
/// through `refused`; everything else is an internal failure.
pub(crate) fn change_failed<S: AuthStore>(
    error: &S::Error,
    refused: Option<ErrorCode>,
) -> AppError {
    if let Some(code) = refused
        && (S::is_aborted(error) || S::is_duplicate(error))
    {
        return AppError::Refused(code);
    }
    internal::<S>(error)
}

/// The record `id` of a kind, for events and the security log.
pub(crate) fn entity(kind: &str, id: impl Into<EntityId>) -> Result<EntityRef, AppError> {
    Ok(EntityRef {
        kind: EntityKind::new(kind).map_err(|_| AppError::Internal("an entity kind".into()))?,
        id: id.into(),
    })
}

/// The record that is one fixed thing (a setting, the setup code of the first start): its ID is
/// made from its name, so every event about it names the same record.
pub(crate) fn fixed_entity(kind: &str, name: &str) -> Result<EntityRef, AppError> {
    let id = EntityId::from_bytes(bb_auth::stable_id(kind, name))
        .map_err(|error| AppError::Internal(error.to_string()))?;
    entity(kind, id)
}

/// The key under which failures of a signed-in or half signed-in person are counted.
pub(crate) fn person_key(person: UserId) -> String {
    format!("person:{person}")
}

/// A field of a change whose value is not shown (credentials): only the fact is recorded.
pub(crate) fn changed(field: &str) -> FieldChange {
    FieldChange {
        field: field.to_owned(),
        before: None,
        after: None,
    }
}

/// A field of a change with its new value.
pub(crate) fn set_to(field: &str, value: serde_json::Value) -> FieldChange {
    FieldChange {
        field: field.to_owned(),
        before: None,
        after: Some(value),
    }
}

/// The actor of an event made by a person.
pub(crate) const fn by(person: UserId) -> Actor {
    Actor::Person(person)
}
