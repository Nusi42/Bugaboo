//! What every sign-in use case has at hand: the store, the clock, the password hasher, passkeys,
//! the CSRF key and the limit on failed tries, plus what is known about the request.
//!
//! Helpers that need the context are methods of [`Ctx`], not free functions, so the only free
//! `async fn`s that take a `Ctx` are the use cases themselves (ARCH-EVENTS reads them).

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use bb_auth::{
    CsrfKey, Device, Passkeys, PasswordHasher, Purpose, SecretToken, StoredCredential,
    looks_like_access_key, same_digest,
};
use bb_core::ErrorCode;
use bb_core::event::{Actor, Channel, DomainEvent, EntityRef, FieldChange, RecordKind};
use bb_core::id::{EntityId, EventId, IdSource, SessionId, UserId};
use bb_core::network::NetworkPrefix;
use bb_core::person::Person;
use bb_core::sign_in::{SecurityAction, SessionStage, SignInSession, StoredPasskey};
use bb_policy::{Evaluation, Place};
use jiff::Timestamp;
use zeroize::Zeroizing;

use super::store::{AuthSettings, AuthStore, SecurityEvent, SessionRecord};
use super::types::{PersonView, Started};
use crate::context::{Clock, SystemIds};
use crate::ports::SetupCodeFile;
use crate::status::LogWrites;

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
    /// The session of the cookie has ended (it ran out, was ended, or never was one): the same
    /// answer as [`ErrorCode::NotSignedIn`], and the browser is told to forget the cookie and
    /// what it kept (M2-F06). A session that is only not at the stage the request needs (the
    /// password step used where a finished sign-in is needed) is not this: it is still waiting.
    SessionEnded,
    /// Too many failed tries: wait this many seconds.
    Limited {
        /// Seconds until the next try may work.
        retry_after: u64,
    },
    /// Something failed inside the server. The text is for the log only.
    Internal(String),
}

impl AppError {
    /// The error code of a refusal, whatever kind of refusal it is. A session that has ended
    /// is refused as not signed in.
    #[must_use]
    pub const fn code(&self) -> Option<ErrorCode> {
        match self {
            Self::Refused(code) => Some(*code),
            Self::SessionEnded => Some(ErrorCode::NotSignedIn),
            Self::Limited { .. } | Self::Internal(_) => None,
        }
    }
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
            Self::SessionEnded => write!(formatter, "refused: the session has ended"),
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
    pub(crate) log_writes: Arc<LogWrites>,
    pub(crate) setup_code_file: Option<Arc<dyn SetupCodeFile>>,
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
            log_writes: Arc::new(LogWrites::default()),
            setup_code_file: None,
        }
    }

    /// The file that holds the setup code of a server without a console, which the use case that
    /// makes the first administrator removes (AUTH-09, M1-F58).
    #[must_use]
    pub fn with_setup_code_file(mut self, file: Arc<dyn SetupCodeFile>) -> Self {
        self.setup_code_file = Some(file);
        self
    }

    /// How many entries of the security log were written, and how many could not be, since the
    /// server started. The status page reads it.
    #[must_use]
    pub fn log_writes(&self) -> Arc<LogWrites> {
        Arc::clone(&self.log_writes)
    }

    /// The context of one request. Its evaluations of who may do what live as long as the
    /// context does: a request, never longer (ADR-0025, decision 3).
    #[must_use]
    pub fn ctx(&self, request: RequestInfo) -> Ctx<'_, S> {
        Ctx {
            services: self,
            request,
            evaluations: Mutex::default(),
            via_key: AtomicBool::new(false),
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

/// The evaluations a request has made, by person and place.
pub(crate) type Evaluations = Mutex<HashMap<(UserId, Place), Arc<Evaluation>>>;

/// The services plus the request, which is what a use case takes.
///
/// The context lives for one request. It holds the evaluations of the policy engine made during
/// the request (the per-request cache of ADR-0025, decision 3); nothing is kept across requests,
/// so a role taken away counts from the next request.
pub struct Ctx<'a, S> {
    pub(crate) services: &'a AuthServices<S>,
    pub(crate) request: RequestInfo,
    pub(crate) evaluations: Evaluations,
    /// The request came with an access key, so what it changes is recorded as made through the
    /// API (the channel of its events).
    pub(crate) via_key: AtomicBool,
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

/// Makes IDs from a clock it owns and the secure random source.
struct OwnedClockIds(Arc<dyn Clock + Send + Sync>);

impl IdSource for OwnedClockIds {
    fn now(&mut self) -> Timestamp {
        self.0.now()
    }

    fn random(&mut self) -> [u8; 16] {
        SystemIds.random()
    }
}

/// A reserved try (see [`FailureLimit`]).
#[derive(Clone)]
pub(crate) struct Slot {
    key: String,
    network: Option<String>,
    tipped: bool,
}

/// The keys of a client network start with this: they are shared by many people, so the limit
/// keeps a window of a minute for them and does not let the windows grow.
pub const NETWORK_KEY_PREFIX: &str = "net:";

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

    /// Fresh IDs of any kind from a source that owns its clock, for a change that makes IDs
    /// inside the transaction (its closure cannot borrow the context).
    pub(crate) fn owned_ids(&self) -> impl IdSource + Send + 'static {
        OwnedClockIds(Arc::clone(&self.services.clock))
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
            channel: if self.via_key.load(Ordering::Relaxed) {
                Channel::Api
            } else {
                Channel::Ui
            },
            entity,
            changes,
            reason: None,
        })
    }

    /// Writes an entry into the security log. A log that cannot be written is reported in the
    /// server log and counted (the status page shows the count, M1-F40) but does not undo or
    /// refuse what already happened (the decision of M1, SEC-15).
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
        let written = self.store().append_security_log(&event).await;
        self.services.log_writes.record(written.is_ok());
        if let Err(error) = written {
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
            .map(|network| format!("{NETWORK_KEY_PREFIX}{network}"));
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

    /// Reserves a try of the client network alone, for a purpose that has no account to count
    /// against (guessing access keys). It is asked for after a failure, not before the check, so
    /// it counts as one and is never given back; the keys of a network keep a window of a minute
    /// and never grow.
    pub(crate) fn reserve_network(&self, purpose: &str) -> Result<Slot, AppError> {
        let network = self
            .request
            .network
            .as_ref()
            .map_or_else(|| "unknown".to_owned(), ToString::to_string);
        let key = format!("{NETWORK_KEY_PREFIX}{purpose}:{network}");
        let tipped = self
            .services
            .limiter
            .reserve(&key)
            .map_err(|retry_after| AppError::Limited { retry_after })?;
        Ok(Slot {
            key,
            network: None,
            tipped,
        })
    }

    /// The reserved try gets its slot back: the server failed before the try could be checked, so
    /// it says nothing about the account (a request that was dropped stays counted).
    pub(crate) fn slot_given_back(&self, slot: &Slot) {
        self.services.limiter.release(&slot.key);
        if let Some(network) = &slot.network {
            self.services.limiter.release(network);
        }
    }

    /// `result`, after the slot was given back when it is an internal failure.
    pub(crate) fn unless_internal<T>(
        &self,
        slot: &Slot,
        result: Result<T, AppError>,
    ) -> Result<T, AppError> {
        if matches!(result, Err(AppError::Internal(_))) {
            self.slot_given_back(slot);
        }
        result
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
        let id = pending.session.id;
        let event = self.event(
            Actor::Person(person),
            entity(RecordKind::Session, id),
            vec![changed("session")],
        )?;
        self.store()
            .record_change(&event, move |transaction| {
                Box::pin(async move { S::delete_session(transaction, id).await.map(|_| ()) })
            })
            .await
            .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
        // Forgotten only now: when the sign-in could not be ended, the next try counts on.
        self.services.limiter.forget(&key);
        self.note(
            SecurityAction::SecondStepGivenUp,
            Some(person),
            Some(entity(RecordKind::Session, id)),
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

/// The answer for a failure of the store: the person gets the standard answer and the cause goes
/// to the log.
pub(crate) fn internal<S: AuthStore>(error: &S::Error) -> AppError {
    AppError::Internal(S::describe(error))
}

/// What a change that did not go through says when it stopped itself (a check inside the
/// transaction failed) and when it hit a duplicate key (two people at once). A rule of the policy
/// engine that stopped it needs no word here: it answers for itself.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Stops {
    aborted: Option<ErrorCode>,
    duplicate: Option<ErrorCode>,
}

impl Stops {
    /// The change cannot stop itself or hit a duplicate: either is an internal failure.
    pub(crate) const NONE: Self = Self {
        aborted: None,
        duplicate: None,
    };

    /// What a change that stopped itself says, and what one that hit a duplicate key says.
    pub(crate) const fn new(aborted: ErrorCode, duplicate: ErrorCode) -> Self {
        Self {
            aborted: Some(aborted),
            duplicate: Some(duplicate),
        }
    }

    /// Both say the same.
    pub(crate) const fn both(code: ErrorCode) -> Self {
        Self::new(code, code)
    }
}

/// The answer for a change that failed: the rule of the policy engine that refused it, what
/// `stops` says for a change that stopped itself or hit a duplicate key, and for anything else an
/// internal failure (the cause goes to the log). The one place that reads the kinds of failure of
/// a change.
pub(crate) fn change_failed<S: AuthStore>(error: &S::Error, stops: Stops) -> AppError {
    if let Some(refusal) = S::as_refusal(error) {
        return refusal.into();
    }
    let code = if S::is_aborted(error) {
        stops.aborted
    } else if S::is_duplicate(error) {
        stops.duplicate
    } else {
        None
    };
    code.map_or_else(|| internal::<S>(error), AppError::Refused)
}

/// The record `id` of a kind, for events and the security log.
pub(crate) fn entity(kind: RecordKind, id: impl Into<EntityId>) -> EntityRef {
    EntityRef {
        kind: kind.into(),
        id: id.into(),
    }
}

/// The record that is one fixed thing (a setting, the setup code of the first start): its ID is
/// made from its name, so every event about it names the same record.
pub(crate) fn fixed_entity(kind: RecordKind, name: &str) -> Result<EntityRef, AppError> {
    let id = EntityId::from_bytes(bb_auth::stable_id(kind.code(), name))
        .map_err(|error| AppError::Internal(error.to_string()))?;
    Ok(entity(kind, id))
}

/// The key under which the wrong codes of a half-finished sign-in are counted. Only a finished
/// second step clears it.
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
