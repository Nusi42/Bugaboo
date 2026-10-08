//! The limit on failed tries (SEC-14) and the methods of the context that use it.

use bb_core::event::{Actor, RecordKind};
use bb_core::id::UserId;
use bb_core::sign_in::SecurityAction;

use super::ctx::Ctx;
use super::error::{AppError, Stops, change_failed};
use super::events::{changed, entity};
use super::store::{AuthStore, SessionRecord};

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

/// A [`FailureLimit`] that lets every try through, for the places that have no failed tries and no
/// attacker: the tests of the use cases and the sample organization.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoFailureLimit;

impl FailureLimit for NoFailureLimit {
    fn reserve(&self, _key: &str) -> Result<bool, u64> {
        Ok(false)
    }

    fn release(&self, _key: &str) {}

    fn clear(&self, _key: &str) {}

    fn tally(&self, _key: &str) -> u32 {
        0
    }

    fn forget(&self, _key: &str) {}
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

/// The key under which the wrong codes of a half-finished sign-in are counted. Only a finished
/// second step clears it.
pub(crate) fn person_key(person: UserId) -> String {
    format!("person:{person}")
}

impl<S: AuthStore> Ctx<'_, S> {
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
}
