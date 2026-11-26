//! The clock of the sample: it shows the moment the generator sets and nothing else, so the
//! organization never depends on the day it is made on. The use cases ask it for the time like any
//! clock (`bb_app::context::Clock`), and for the random part of the IDs they make, which it draws
//! from the seed: two samples made from one seed have the same IDs, also the random codes of the
//! issue IDs of a project, so the pictures of the views are the same every time. Secrets (passwords,
//! sessions, keys) never come from here.

use std::sync::Mutex;
use std::sync::atomic::{AtomicI64, Ordering};

use bb_app::context::Clock;
use jiff::Timestamp;

use crate::large_ids::random16;
use crate::random::SeededRandom;

/// What the random part of the IDs is drawn from, apart from the stream of the people and the
/// issues, so a new ID somewhere does not change who or what the sample makes.
const ID_STREAM: u64 = 0x0001_D50F_5A3D;

/// A clock the generator sets by hand, with the random part of the IDs drawn from the seed.
#[derive(Debug)]
pub struct SampleClock {
    seconds: AtomicI64,
    draws: Mutex<SeededRandom>,
}

impl SampleClock {
    /// A clock that shows `moment` and draws the random part of the IDs from `seed`.
    #[must_use]
    pub fn at(moment: Timestamp, seed: u64) -> Self {
        Self {
            seconds: AtomicI64::new(moment.as_second()),
            draws: Mutex::new(SeededRandom::new(seed ^ ID_STREAM)),
        }
    }

    /// Moves the clock on by `seconds`.
    pub fn advance(&self, seconds: i64) {
        self.seconds.fetch_add(seconds, Ordering::SeqCst);
    }

    /// Moves the clock to `moment`.
    pub fn set(&self, moment: Timestamp) {
        self.seconds.store(moment.as_second(), Ordering::SeqCst);
    }
}

impl Clock for SampleClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_second(self.seconds.load(Ordering::SeqCst)).unwrap_or(Timestamp::UNIX_EPOCH)
    }

    fn random(&self) -> [u8; 16] {
        // A poisoned lock only means a use case panicked while it held it; the stream is still a
        // stream of numbers.
        let mut draws = self
            .draws
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        random16(&mut draws)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_clock_shows_what_was_set_and_does_not_move_by_itself() {
        let start = Timestamp::from_second(bb_dev_words::accounts::CLOCK_START).unwrap();
        let clock = SampleClock::at(start, 42);
        assert_eq!(clock.now(), start);
        assert_eq!(clock.now(), start);
        let later = Timestamp::from_second(1_800_000_020).unwrap();
        clock.set(later);
        assert_eq!(clock.now(), later);
        clock.advance(1);
        assert_eq!(clock.now().as_second(), later.as_second() + 1);
    }

    #[test]
    fn the_random_part_of_the_ids_comes_from_the_seed() {
        let start = Timestamp::from_second(bb_dev_words::accounts::CLOCK_START).unwrap();
        let draws = |seed| {
            let clock = SampleClock::at(start, seed);
            [clock.random(), clock.random()]
        };
        assert_eq!(draws(42), draws(42));
        assert_ne!(draws(42), draws(7));
        let [first, second] = draws(42);
        assert_ne!(first, second);
    }
}
