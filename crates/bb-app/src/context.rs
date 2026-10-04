//! What every use case needs from the outside world: the clock and secure randomness.
//!
//! `bb-core` does no I/O, so the real sources live here and `bb-server` hands them to the use
//! cases (architecture §4, "Ctx: actor, request id, clock, ports"). Tests use `SequentialIds`
//! from `bb-core` (feature `fake-ids`) instead.

use bb_core::id::IdSource;
use jiff::Timestamp;

/// The current moment. A port, so a test can stop the clock.
pub trait Clock {
    /// The current moment.
    fn now(&self) -> Timestamp;
}

/// The real clock of the computer.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        Timestamp::now()
    }
}

/// The real [`IdSource`]: the clock of the computer and the secure random source of the
/// operating system (M1-F13).
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemIds;

impl IdSource for SystemIds {
    fn now(&mut self) -> Timestamp {
        Timestamp::now()
    }

    fn random(&mut self) -> [u8; 16] {
        let mut bytes = [0_u8; 16];
        if getrandom::fill(&mut bytes).is_err() {
            // Without secure randomness every ID and every key would be guessable, and there is
            // nothing a caller could do about it. Stopping is the only safe answer.
            std::process::abort();
        }
        bytes
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use bb_core::id::{IssueId, UserId};

    use super::*;

    #[test]
    fn system_ids_are_new_every_time_and_time_ordered() {
        let mut ids = SystemIds;
        let first = IssueId::generate(&mut ids).unwrap();
        let second = IssueId::generate(&mut ids).unwrap();
        assert_ne!(first, second);
        assert!(first.created_at_millis() >= 1_767_225_600_000, "after 2026");
        let people: HashSet<UserId> = (0..100).map(|_| UserId::generate(&mut ids)).collect();
        assert_eq!(people.len(), 100);
    }

    #[test]
    fn the_system_clock_moves_on() {
        let clock = SystemClock;
        let first = clock.now();
        assert!(clock.now() >= first);
    }
}
