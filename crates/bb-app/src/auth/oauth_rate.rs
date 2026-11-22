//! The limits on the calls of AI apps (ADR-0042, decision 6): at most 120 calls a minute for one
//! grant (an access key counts as its own grant) and 240 a minute for one person over all their
//! grants and keys. A call over the limit is answered with the time to wait, and nothing of it is
//! done. The counting is the fixed window of the rest of the server (`crate::rate_window`), on the
//! clock of the app: the calls of a minute are those of a minute of that clock, so a busy computer
//! that answers slowly does not spread the calls over more windows than the app saw minutes.

use std::sync::{Mutex, PoisonError};

use jiff::Timestamp;

use bb_core::id::UserId;
use bb_core::oauth::{
    CALLS_A_MINUTE_FOR_A_GRANT, CALLS_A_MINUTE_FOR_A_PERSON, REGISTRATIONS_AN_HOUR,
    REGISTRATIONS_AN_HOUR_FOR_THE_SERVER,
};

use super::ctx::Ctx;
use super::error::AppError;
use super::store::AuthStore;
use crate::access::Actor;
use crate::rate_window::{HOUR, Overflow, Window};

/// The calls of the last minute, by grant and by person, and the registrations of the last hour,
/// by network and for the whole server.
pub(crate) struct CallRate {
    grants: Window<String, Timestamp>,
    people: Window<UserId, Timestamp>,
    registrations: Window<String, Timestamp>,
    all_registrations: Window<(), Timestamp>,
    /// Held while a registration is counted, so the count of the hour it reads is its own.
    registering: Mutex<()>,
}

/// What counting a registration found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Registration {
    /// It was counted: the how-manyth of the server's hour it is.
    Counted(u32),
    /// The server's hour is full: the app may only take the place of one that nobody allowed, or
    /// waits this many seconds.
    Full(u64),
}

impl Default for CallRate {
    fn default() -> Self {
        // A full table refuses a grant or person it does not know yet: a flood of invented ones
        // cannot use up the memory.
        Self {
            grants: Window::new(CALLS_A_MINUTE_FOR_A_GRANT, Overflow::Refuse),
            people: Window::new(CALLS_A_MINUTE_FOR_A_PERSON, Overflow::Refuse),
            registrations: Window::lasting(HOUR, REGISTRATIONS_AN_HOUR, Overflow::Refuse),
            all_registrations: Window::lasting(
                HOUR,
                REGISTRATIONS_AN_HOUR_FOR_THE_SERVER,
                Overflow::Refuse,
            ),
            registering: Mutex::new(()),
        }
    }
}

/// What a call is counted against.
pub(crate) struct Counted<'a> {
    pub(crate) grant: &'a str,
    pub(crate) person: UserId,
}

impl CallRate {
    /// Counts one call against the grant and the person, or says how many seconds to wait. Both
    /// must have room: a call that is turned away counts against neither.
    pub(crate) fn take(&self, counted: &Counted<'_>, now: Timestamp) -> Result<(), u64> {
        let grant = counted.grant.to_owned();
        let wait = [
            self.grants.room(&grant, now),
            self.people.room(&counted.person, now),
        ]
        .into_iter()
        .filter_map(Result::err)
        .max();
        if let Some(wait) = wait {
            return Err(wait);
        }
        self.grants.hit(grant, now)?;
        self.people.hit(counted.person, now)
    }

    /// Counts one registration against `network` and the whole server, in one step: the count of
    /// the server's hour it gives back is exact, also when apps register at the same moment.
    ///
    /// # Errors
    /// The seconds to wait when the network has registered its share of the hour; it is not
    /// counted then.
    fn register(&self, network: String, now: Timestamp) -> Result<Registration, u64> {
        let _alone = self
            .registering
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        self.registrations.hit(network, now)?;
        Ok(match self.all_registrations.hit((), now) {
            Ok(()) => Registration::Counted(self.all_registrations.count(&(), now)),
            Err(wait) => Registration::Full(wait),
        })
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Counts one registration of an app against the client network and the whole server, and
    /// says how many the server took in this hour so far, or that its hour is full.
    ///
    /// # Errors
    /// [`AppError::Limited`] with the seconds to wait, when the network has registered its share.
    pub(crate) fn count_registration(&self) -> Result<Registration, AppError> {
        self.services
            .call_rate
            .register(self.network_key(), self.now())
            .map_err(|retry_after| AppError::Limited { retry_after })
    }

    /// Gives the registration of this network back: the server's hour was full and no app could
    /// make room, so nothing was registered.
    pub(crate) fn give_back_registration(&self) {
        self.services
            .call_rate
            .registrations
            .give_back(&self.network_key());
    }

    /// Counts a call of an AI app against its grant (or key) and its person.
    ///
    /// # Errors
    /// [`AppError::Limited`] with the seconds to wait.
    pub(crate) fn count_call(&self, actor: &Actor) -> Result<(), AppError> {
        let grant = actor.credential_key();
        self.services
            .call_rate
            .take(
                &Counted {
                    grant: &grant,
                    person: actor.person.id,
                },
                self.now(),
            )
            .map_err(|retry_after| AppError::Limited { retry_after })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_23_a_grant_over_its_limit_waits_and_the_call_counts_for_nothing() {
        let rate = CallRate::default();
        let start = Timestamp::from_second(1_800_000_000).unwrap_or_default();
        let at = |seconds: i64| start + jiff::SignedDuration::from_secs(seconds);
        let person = UserId::generate(&mut bb_core::id::SequentialIds::default());
        let first = Counted {
            grant: "g1",
            person,
        };
        for _ in 0..CALLS_A_MINUTE_FOR_A_GRANT {
            assert!(rate.take(&first, at(0)).is_ok());
        }
        assert_eq!(rate.take(&first, at(10)), Err(50));
        // Another grant of the same person has room until the limit of the person is used up.
        let second = Counted {
            grant: "g2",
            person,
        };
        for _ in 0..CALLS_A_MINUTE_FOR_A_GRANT {
            assert!(rate.take(&second, at(10)).is_ok());
        }
        let third = Counted {
            grant: "g3",
            person,
        };
        // The person's window started at 0: 240 calls, so the next waits for 50 seconds more.
        assert_eq!(rate.take(&third, at(11)), Err(49));
        // A minute later all of it is free again.
        assert!(rate.take(&first, at(61)).is_ok());
        assert!(rate.take(&third, at(61)).is_ok());
    }

    #[test]
    fn sec_14_registrations_at_the_same_moment_each_get_a_count_of_their_own() {
        let rate = CallRate::default();
        let now = Timestamp::from_second(1_800_000_000).unwrap_or_default();
        let networks = REGISTRATIONS_AN_HOUR_FOR_THE_SERVER / REGISTRATIONS_AN_HOUR;
        let mut counts: Vec<u32> = std::thread::scope(|scope| {
            let running: Vec<_> = (0..REGISTRATIONS_AN_HOUR_FOR_THE_SERVER)
                .map(|number| {
                    let rate = &rate;
                    scope.spawn(move || rate.register(format!("net{}", number % networks), now))
                })
                .collect();
            running
                .into_iter()
                .map(|thread| match thread.join() {
                    Ok(Ok(Registration::Counted(count))) => count,
                    other => panic!("{other:?}"),
                })
                .collect()
        });
        counts.sort_unstable();
        // Each is counted once and reads its own count: the first and the last of the hour are
        // found exactly once.
        assert_eq!(
            counts,
            (1..=REGISTRATIONS_AN_HOUR_FOR_THE_SERVER).collect::<Vec<_>>()
        );
        assert!(matches!(
            rate.register("another".to_owned(), now),
            Ok(Registration::Full(_))
        ));
    }
}
