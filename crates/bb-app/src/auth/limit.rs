//! The limit on failed tries (SEC-14) and the methods of the context that use it.

use bb_core::event::{Actor, RecordKind};
use bb_core::id::UserId;
use bb_core::sign_in::SecurityAction;

use super::ctx::Ctx;
use super::error::{AppError, Stops, change_failed};
use super::events::{changed, entity};
use super::store::{AuthStore, SessionRecord};
use crate::ports::BoxFuture;

/// The limit on failed tries (SEC-14). A try is **reserved before it is checked** and counts as a
/// failure from that moment, so requests that arrive at the same time cannot all slip through
/// before the first failure is written down. A success gives the reservation back, so only real
/// failures stay counted: nobody can lock another person out by merely trying their address
/// (M1-F27). A key that keeps using up its allowance waits longer each time, up to a cap, and
/// never for good. An account that is over its allowance from everywhere takes the tries from
/// networks it never signed in from one at a time, and only so many a day; the networks it signed in
/// from are not held up. The HTTP layer implements it in memory.
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

    /// Reserves one answer of the day of `key` in one step: it is counted when the key has some
    /// left, so requests that arrive at the same time cannot all pass the last one (a day starts
    /// with the first answer that is counted).
    ///
    /// # Errors
    /// The seconds until the day ends, when `limit` answers are used up.
    fn daily_reserve(&self, key: &str, limit: u32) -> Result<(), u64>;

    /// Gives one reserved answer of the day of `key` back: the answer told nothing.
    fn daily_release(&self, key: &str);

    /// Reserves one try of the day of the account behind `key` from a network it never signed in
    /// from. [`Self::daily_release`] gives it back.
    ///
    /// # Errors
    /// The seconds until the day ends, when the account has taken all its tries of the day.
    fn day_reserve(&self, key: &str) -> Result<(), u64>;

    /// Waits for the turn of the account `key`, which is over its allowance from everywhere: its
    /// tries from networks it never signed in from are checked one at a time, a little apart, so
    /// guessing from many networks at once gains nothing.
    ///
    /// # Errors
    /// The seconds to wait, when the turn is too far away to wait for.
    fn wait_turn(&self, key: &str) -> BoxFuture<'_, Result<(), u64>>;

    /// Remembers that the account of `pair` (an account and a network) signed in from that network.
    fn signed_in_from(&self, pair: &str);

    /// Whether the account of `pair` signed in from its network lately: the person's own network,
    /// which the account alone does not hold up (M1-F27).
    fn signed_in_before(&self, pair: &str) -> bool;
}

/// A [`FailureLimit`] that lets every try through, for the places that have no failed tries and no
/// attacker: the tests of the use cases and the sample organization. Only there: the feature
/// `testing` (or a test of this crate) makes it, so a server build cannot hold one.
#[cfg(any(test, feature = "testing"))]
#[derive(Clone, Copy, Debug, Default)]
pub struct NoFailureLimit;

#[cfg(any(test, feature = "testing"))]
impl FailureLimit for NoFailureLimit {
    fn reserve(&self, _key: &str) -> Result<bool, u64> {
        Ok(false)
    }

    fn release(&self, _key: &str) {}

    fn clear(&self, _key: &str) {}

    fn daily_reserve(&self, _key: &str, _limit: u32) -> Result<(), u64> {
        Ok(())
    }

    fn daily_release(&self, _key: &str) {}

    fn tally(&self, _key: &str) -> u32 {
        0
    }

    fn forget(&self, _key: &str) {}

    fn day_reserve(&self, _key: &str) -> Result<(), u64> {
        Ok(())
    }

    fn wait_turn(&self, _key: &str) -> BoxFuture<'_, Result<(), u64>> {
        Box::pin(std::future::ready(Ok(())))
    }

    fn signed_in_from(&self, _pair: &str) {}

    fn signed_in_before(&self, _pair: &str) -> bool {
        false
    }
}

/// A reserved try (see [`FailureLimit`]).
#[derive(Clone)]
pub(crate) struct Slot {
    key: String,
    network: Option<String>,
    /// The account alone, when its try was counted (see [`Ctx::reserve_account`]).
    account: Option<String>,
    /// The try of the day of the account, when one was counted (a network it never signed in
    /// from).
    day: Option<String>,
    /// Whether the key is an account from a network: a success remembers the network as one the
    /// account signed in from.
    from_network: bool,
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
    purpose_key(PERSON_PURPOSE, &person)
}

/// The purpose of the tries that count against a person at sign-in.
const PERSON_PURPOSE: &str = "person";

/// What a request whose network is not known counts against.
const UNKNOWN_NETWORK: &str = "unknown";

/// The purpose of the tries of the day at an account from networks it never signed in from.
const DAY_PURPOSE: &str = "day";

/// The purpose of the one note a day that an account took all its tries of the day.
const CAPPED_PURPOSE: &str = "capped";

/// The key of the failed tries at the account of `email`: the address as typed, trimmed, in lower
/// case and cut at [`KEY_CHARS`] characters.
fn account_key(email: &str) -> String {
    format!(
        "account:{}",
        email
            .trim()
            .to_lowercase()
            .chars()
            .take(KEY_CHARS)
            .collect::<String>()
    )
}

/// Long text in the key of a failure counter is cut here: the key only has to tell accounts
/// apart, and a megabyte of "address" must not use memory.
const KEY_CHARS: usize = 320;

/// The one way a key of the limit is written: what the tries are for, and whom they count against.
fn purpose_key(purpose: &str, counted_against: &(impl std::fmt::Display + ?Sized)) -> String {
    format!("{purpose}:{counted_against}")
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Reserves a try of `key` (an account, a person, a setup) and of the client network. The
    /// try counts as a failure until [`Self::slot_succeeded`] says otherwise.
    pub(crate) fn reserve(&self, key: String) -> Result<Slot, AppError> {
        let limiter = &self.services.limiter;
        let wait_error = |retry_after| AppError::Limited { retry_after };
        // Only a request with a network counts against one (the server always knows it; a use
        // case run without a request, like a test of the rules, does not).
        let network = self
            .request
            .network
            .is_some()
            .then(|| format!("{NETWORK_KEY_PREFIX}{}", self.network_key()));
        let network_tipped = match &network {
            Some(network) => limiter.reserve(network).map_err(wait_error)?,
            None => false,
        };
        match limiter.reserve(&key) {
            Ok(tipped) => Ok(Slot {
                key,
                network,
                account: None,
                day: None,
                from_network: false,
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
        self.reserve_alone(format!(
            "{NETWORK_KEY_PREFIX}{}",
            purpose_key(purpose, &self.network_key())
        ))
    }

    /// The client network as the keys of the limits write it.
    pub(crate) fn network_key(&self) -> String {
        self.network_name()
            .unwrap_or_else(|| UNKNOWN_NETWORK.to_owned())
    }

    /// Reserves a try at signing in to the account of `email`. The account **from this network**
    /// is what is refused when it keeps failing, with the window that grows. The account alone
    /// counts the tries from everywhere; when it is over its allowance, the tries from networks it
    /// never signed in from wait their turn, one at a time, and those networks together have only
    /// so many tries a day, after which they are refused, written down and the person is told. A
    /// network the account signed in from is held up by neither: somebody who tries an address from
    /// many networks cannot lock the person out, who still gets in from their own network (M1-F27).
    pub(crate) async fn reserve_account(&self, email: &str) -> Result<Slot, AppError> {
        let limiter = &self.services.limiter;
        let account = account_key(email);
        let mut slot = self.reserve(purpose_key(&account, &self.network_key()))?;
        slot.from_network = true;
        if !limiter.signed_in_before(&slot.key) {
            let day = purpose_key(DAY_PURPOSE, &account);
            if let Err(retry_after) = limiter.day_reserve(&day) {
                self.slot_given_back(&slot);
                self.account_capped(email, &account).await;
                return Err(AppError::Limited { retry_after });
            }
            slot.day = Some(day);
        }
        match limiter.reserve(&account) {
            // The account alone refuses nothing, so its last try is not written down as a limit.
            Ok(_) => slot.account = Some(account),
            Err(_) if slot.day.is_some() => {
                if let Err(retry_after) = limiter.wait_turn(&account).await {
                    self.slot_given_back(&slot);
                    return Err(AppError::Limited { retry_after });
                }
            }
            // The person's own network.
            Err(_) => {}
        }
        Ok(slot)
    }

    /// The account of `email` took all its tries of the day from networks it never signed in from:
    /// the first refusal of the day is written to the security log and told to the person, the
    /// others are not, so a flood fills neither.
    async fn account_capped(&self, email: &str, account: &str) {
        let first_today = self
            .services
            .limiter
            .daily_reserve(&purpose_key(CAPPED_PURPOSE, account), 1)
            .is_ok();
        if !first_today {
            return;
        }
        let person = match self.store().account_by_email(email).await {
            Ok(found) => found.map(|account| account.person.id),
            Err(error) => {
                // Written down without the person: the refusal stands either way.
                tracing::warn!(cause = %S::describe(&error), "the account of a refused sign-in could not be read");
                None
            }
        };
        self.note(SecurityAction::SignInCapped, person, None).await;
        if let Some(person) = person
            && let Err(error) = self.tell_sign_in_capped(person).await
        {
            tracing::warn!(cause = %error, "the person could not be told that their sign-in was refused");
        }
    }

    /// Tells the person that the sign-in to their account was refused for the day from networks it
    /// never signed in from: an event of their record, which the notifications turn into a notice
    /// they cannot switch off.
    async fn tell_sign_in_capped(&self, person: UserId) -> Result<(), AppError> {
        let event = self.event(
            Actor::System,
            entity(RecordKind::Person, person),
            vec![changed(crate::member::person::SIGN_IN_CAPPED)],
        )?;
        self.store()
            .record_change(&event, |_| Box::pin(async { Ok(()) }))
            .await
            .map_err(|error| change_failed::<S>(&error, Stops::NONE))
    }

    /// Reserves one answer of the day of a person for a purpose (`handle`): the answers that tell
    /// something about other people are counted against the person for a day, and after the
    /// allowance every try gets the same answer, whatever it asked for. Check and count are one
    /// step, so parallel requests cannot all pass the last answer.
    ///
    /// # Errors
    /// `rate-limit.exceeded` with the wait, when the allowance of the day is used up.
    pub(crate) fn reserve_daily_answer(
        &self,
        purpose: &str,
        person: UserId,
        limit: u32,
    ) -> Result<(), AppError> {
        self.services
            .limiter
            .daily_reserve(&purpose_key(purpose, &person), limit)
            .map_err(|retry_after| AppError::Limited { retry_after })
    }

    /// Gives a reserved answer of the day back: what was answered told nothing about anybody.
    pub(crate) fn give_back_daily_answer(&self, purpose: &str, person: UserId) {
        self.services
            .limiter
            .daily_release(&purpose_key(purpose, &person));
    }

    /// Reserves a try of one person for a purpose (`handle`): the tries that are refused count
    /// until the person is let through, and a person who keeps being refused waits. Only the person
    /// counts, not the network, so a shared network is not held up by one of its people.
    pub(crate) fn reserve_for_person(
        &self,
        purpose: &str,
        person: UserId,
    ) -> Result<Slot, AppError> {
        self.reserve_alone(purpose_key(purpose, &person))
    }

    /// Reserves a try of `key` alone: no network is counted besides it.
    fn reserve_alone(&self, key: String) -> Result<Slot, AppError> {
        let tipped = self
            .services
            .limiter
            .reserve(&key)
            .map_err(|retry_after| AppError::Limited { retry_after })?;
        Ok(Slot {
            key,
            network: None,
            account: None,
            day: None,
            from_network: false,
            tipped,
        })
    }

    /// The reserved try gets its slot back: the server failed before the try could be checked, so
    /// it says nothing about the account (a request that was dropped stays counted).
    pub(crate) fn slot_given_back(&self, slot: &Slot) {
        self.services.limiter.release(&slot.key);
        for shared in [&slot.network, &slot.account].into_iter().flatten() {
            self.services.limiter.release(shared);
        }
        if let Some(day) = &slot.day {
            self.services.limiter.daily_release(day);
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

    /// The reserved try was no failure: its key is cleared; the network and the account alone,
    /// which others share, get this try back (the failures of others there stay counted), and so
    /// does the day of the account. An account remembers the network it signed in from.
    pub(crate) fn slot_succeeded(&self, slot: Slot) {
        let limiter = &self.services.limiter;
        let Slot {
            key,
            network,
            account,
            day,
            from_network,
            ..
        } = slot;
        limiter.clear(&key);
        for shared in [network, account].into_iter().flatten() {
            limiter.release(&shared);
        }
        if let Some(day) = day {
            limiter.daily_release(&day);
        }
        if from_network {
            limiter.signed_in_from(&key);
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
