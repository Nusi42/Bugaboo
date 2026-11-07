//! The limits on the calls of AI apps (ADR-0042, decision 6): at most 120 calls a minute for one
//! grant (an access key counts as its own grant) and 240 a minute for one person over all their
//! grants and keys. A call over the limit is answered with the time to wait, and nothing of it is
//! done.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use bb_core::id::UserId;
use bb_core::oauth::{CALLS_A_MINUTE_FOR_A_GRANT, CALLS_A_MINUTE_FOR_A_PERSON};
use jiff::Timestamp;

use super::ctx::{Ctx, locked};
use super::error::AppError;
use super::store::AuthStore;
use crate::access::{Actor, CredentialSource};

/// The length of the window the calls are counted in.
const WINDOW_SECONDS: i64 = 60;
/// More keys than this are cleaned of empty windows when a call is counted.
const KEYS_BEFORE_CLEANING: usize = 4_096;

/// The calls of the last minute, by what they are counted against.
#[derive(Debug, Default)]
pub(crate) struct CallRate {
    windows: Mutex<HashMap<String, VecDeque<i64>>>,
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
        let now = now.as_second();
        let mut windows = locked(&self.windows);
        if windows.len() > KEYS_BEFORE_CLEANING {
            windows.retain(|_, calls| calls.back().is_some_and(|last| now - last < WINDOW_SECONDS));
        }
        let person = format!("person:{}", counted.person);
        let grant = format!("grant:{}", counted.grant);
        let mut wait = 0;
        for (key, limit) in [
            (&grant, CALLS_A_MINUTE_FOR_A_GRANT),
            (&person, CALLS_A_MINUTE_FOR_A_PERSON),
        ] {
            let calls = windows.entry(key.clone()).or_default();
            while calls
                .front()
                .is_some_and(|first| now - first >= WINDOW_SECONDS)
            {
                calls.pop_front();
            }
            if calls.len() >= limit as usize
                && let Some(first) = calls.front()
            {
                wait = wait.max(
                    u64::try_from(first + WINDOW_SECONDS - now)
                        .unwrap_or(1)
                        .max(1),
                );
            }
        }
        if wait > 0 {
            return Err(wait);
        }
        for key in [grant, person] {
            windows.entry(key).or_default().push_back(now);
        }
        Ok(())
    }
}

impl<S: AuthStore> Ctx<'_, S> {
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
                self.now(),
            )
            .map_err(|retry_after| AppError::Limited { retry_after })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(seconds: i64) -> Timestamp {
        Timestamp::from_second(1_800_000_000 + seconds).unwrap()
    }

    #[test]
    fn ai_23_a_grant_over_its_limit_waits_and_the_call_counts_for_nothing() {
        let rate = CallRate::default();
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
        assert_eq!(rate.take(&third, at(11)), Err(49));
        // A minute later all of it is free again.
        assert!(rate.take(&first, at(61)).is_ok());
        assert!(rate.take(&third, at(61)).is_ok());
    }
}
