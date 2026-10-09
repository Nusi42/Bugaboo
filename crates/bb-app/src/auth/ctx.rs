//! What every sign-in use case has at hand: the store, the clock, the password hasher, passkeys,
//! the CSRF key and the limit on failed tries, plus what is known about the request.
//!
//! Helpers that need the context are methods of [`Ctx`], not free functions, so the only free
//! `async fn`s that take a `Ctx` are the use cases themselves (ARCH-EVENTS reads them).

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use bb_auth::{CsrfKey, Device, Passkeys, PasswordHasher, SecretToken};
use bb_core::event::{Actor, Channel, DomainEvent, EntityRef, FieldChange};
use bb_core::id::{EventId, IdSource, SessionId, UserId};
use bb_core::network::NetworkPrefix;
use bb_core::sign_in::SecurityAction;
use bb_policy::{Evaluation, Place};
use jiff::Timestamp;

use super::error::{AppError, internal};
use super::limit::FailureLimit;
use super::store::{AuthSettings, AuthStore, SecurityEvent};
use crate::context::{Clock, SystemIds};
use crate::ports::SetupCodeFile;
use crate::status::LogWrites;

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

/// Makes the events of one request: its actor, its channel and the clock of the context.
#[derive(Clone)]
pub(crate) struct EventMaker {
    clock: Arc<dyn Clock + Send + Sync>,
    actor: Actor,
    channel: Channel,
}

impl EventMaker {
    /// The event of a change to the record `entity`.
    pub(crate) fn make(
        &self,
        entity: EntityRef,
        changes: Vec<FieldChange>,
    ) -> Result<DomainEvent, bb_core::id::IdError> {
        let mut ids = ClockIds(self.clock.as_ref());
        Ok(DomainEvent {
            id: EventId::generate(&mut ids)?,
            time: self.clock.now(),
            actor: self.actor,
            channel: self.channel,
            entity,
            changes,
            reason: None,
        })
    }
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
        self.event_maker(actor)
            .make(entity, changes)
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    /// What makes events of this request by `actor` inside a change, which cannot borrow the
    /// context (a change that does more than its own event says, one event for each thing).
    pub(crate) fn event_maker(&self, actor: Actor) -> EventMaker {
        EventMaker {
            clock: Arc::clone(&self.services.clock),
            actor,
            channel: if self.via_key.load(Ordering::Relaxed) {
                Channel::Api
            } else {
                Channel::Ui
            },
        }
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
}
