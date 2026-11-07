//! The limits on the calls of AI apps (ADR-0042, decision 6): at most 120 calls a minute for one
//! grant (an access key counts as its own grant) and 240 a minute for one person over all their
//! grants and keys. A call over the limit is answered with the time to wait, and nothing of it is
//! done. The counting is the fixed window of the rest of the server (`crate::rate_window`).

use std::time::{Duration, Instant};

use bb_core::id::UserId;
use bb_core::oauth::{
    CALLS_A_MINUTE_FOR_A_GRANT, CALLS_A_MINUTE_FOR_A_PERSON, REGISTRATIONS_AN_HOUR,
};

use super::ctx::Ctx;
use super::error::AppError;
use super::store::AuthStore;
use crate::access::{Actor, CredentialSource};
use crate::rate_window::{Overflow, Window};

/// The calls of the last minute, by grant and by person, and the registrations of the last hour,
/// by network.
pub(crate) struct CallRate {
    grants: Window<String>,
    people: Window<UserId>,
    registrations: Window<String>,
}

impl Default for CallRate {
    fn default() -> Self {
        // A full table refuses a grant or person it does not know yet: a flood of invented ones
        // cannot use up the memory.
        Self {
            grants: Window::new(CALLS_A_MINUTE_FOR_A_GRANT, Overflow::Refuse),
            people: Window::new(CALLS_A_MINUTE_FOR_A_PERSON, Overflow::Refuse),
            registrations: Window::lasting(
                Duration::from_hours(1),
                REGISTRATIONS_AN_HOUR,
                Overflow::Refuse,
            ),
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
    pub(crate) fn take(&self, counted: &Counted<'_>, now: Instant) -> Result<(), u64> {
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
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Counts one registration of an app against the client network.
    ///
    /// # Errors
    /// [`AppError::Limited`] with the seconds to wait.
    pub(crate) fn count_registration(&self) -> Result<(), AppError> {
        let network = self
            .request
            .network
            .as_ref()
            .map_or_else(|| "unknown".to_owned(), ToString::to_string);
        self.services
            .call_rate
            .registrations
            .hit(network, Instant::now())
            .map_err(|retry_after| AppError::Limited { retry_after })
    }

    /// Counts a call of an AI app against its grant (or key) and its person.
    ///
    /// # Errors
    /// [`AppError::Limited`] with the seconds to wait.
    pub(crate) fn count_call(&self, actor: &Actor) -> Result<(), AppError> {
        let grant = match actor.key.as_ref().map(|key| &key.source) {
            Some(CredentialSource::Key(id)) => id.to_string(),
            Some(CredentialSource::App { grant, .. }) => grant.to_string(),
            None => actor.person.id.to_string(),
        };
        self.services
            .call_rate
            .take(
                &Counted {
                    grant: &grant,
                    person: actor.person.id,
                },
                Instant::now(),
            )
            .map_err(|retry_after| AppError::Limited { retry_after })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn ai_23_a_grant_over_its_limit_waits_and_the_call_counts_for_nothing() {
        let rate = CallRate::default();
        let start = Instant::now();
        let at = |seconds: u64| start + Duration::from_secs(seconds);
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
}
