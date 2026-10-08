//! The clock of the sample: it shows the moment the generator sets and nothing else, so the
//! organization never depends on the day it is made on. The use cases ask it for the time like any
//! clock (`bb_app::context::Clock`).

use std::sync::atomic::{AtomicI64, Ordering};

use bb_app::context::Clock;
use jiff::Timestamp;

/// A clock the generator sets by hand.
#[derive(Debug)]
pub struct SampleClock(AtomicI64);

impl SampleClock {
    /// A clock that shows `moment`.
    #[must_use]
    pub fn at(moment: Timestamp) -> Self {
        Self(AtomicI64::new(moment.as_second()))
    }

    /// Moves the clock on by `seconds`.
    pub fn advance(&self, seconds: i64) {
        self.0.fetch_add(seconds, Ordering::SeqCst);
    }

    /// Moves the clock to `moment`.
    pub fn set(&self, moment: Timestamp) {
        self.0.store(moment.as_second(), Ordering::SeqCst);
    }
}

impl Clock for SampleClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_second(self.0.load(Ordering::SeqCst)).unwrap_or(Timestamp::UNIX_EPOCH)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_clock_shows_what_was_set_and_does_not_move_by_itself() {
        let start = Timestamp::from_second(1_800_000_000).unwrap();
        let clock = SampleClock::at(start);
        assert_eq!(clock.now(), start);
        assert_eq!(clock.now(), start);
        let later = Timestamp::from_second(1_800_000_020).unwrap();
        clock.set(later);
        assert_eq!(clock.now(), later);
        clock.advance(1);
        assert_eq!(clock.now().as_second(), later.as_second() + 1);
    }
}
