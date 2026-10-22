//! The limit on failed tries (SEC-14), used by the sign-in use cases through
//! `bb_app::auth::FailureLimit`.
//!
//! - A **try is reserved before it is checked** and counts as a failure from that moment, so
//!   requests that arrive at the same time cannot all slip through before the first failure is
//!   written down. A success gives the reservation back (`release`, for keys that many people
//!   share, like a network) or clears the key (`clear`, for one account), so only real failures
//!   stay counted (M1-F27).
//! - A key has a **window** of a minute and `limit` tries in it. Every window that ends with at
//!   least one failure makes the next one twice as long, up to [`MAX_WAIT`]: a growing wait, never
//!   a lock for good. Pacing the tries just under the allowance does not help, because the
//!   windows grow all the same. The growth is forgotten after a success (`clear`) or after a quiet
//!   time of [`MAX_WAIT`]. Keys of a network (`bb_app::auth::NETWORK_KEY_PREFIX`) are shared by
//!   many people and keep a window of a minute: they limit a flood, they do not grow.
//! - A **tally** counts tries without a window (the wrong codes of one half-finished sign-in).
//! - The keys are kept as keyed hashes in memory (a restart forgets them, which only lets a client
//!   try again sooner). A table that is full of running counters refuses new keys (fail closed);
//!   counters that are still running are never thrown away.

use std::collections::HashMap;
use std::hash::{BuildHasher as _, RandomState};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use bb_app::auth::{FailureLimit, NETWORK_KEY_PREFIX};

/// Failed tries per key and window when nothing else is set (accounts, people and networks).
pub const DEFAULT_SIGN_IN_FAILURES_PER_MINUTE: u32 = 10;

/// The first window.
const BASE_WINDOW: Duration = Duration::from_mins(1);
/// The longest a key has to wait: windows double up to this.
pub const MAX_WAIT: Duration = Duration::from_mins(15);
/// Each table remembers at most this many keys.
const MAX_KEYS: usize = 100_000;
/// A tally is forgotten after this long without a new try.
const TALLY_FORGET: Duration = Duration::from_mins(10);

struct Tracker {
    started: Instant,
    window: Duration,
    count: u32,
    strikes: u32,
    /// Whether the windows of this key grow (not those of a network).
    grows: bool,
}

impl Tracker {
    /// When the key can be forgotten: after its window, and, while failures are still counted in
    /// it, after the quiet time in which a next window still grows.
    fn forget_at(&self) -> Instant {
        let end = self.started + self.window;
        if self.grows && self.count >= 1 {
            end + MAX_WAIT
        } else {
            end
        }
    }
}

struct Tables {
    trackers: HashMap<u64, Tracker>,
    tallies: HashMap<u64, (Instant, u32)>,
    /// The answers counted for a day: when the day began, and how many so far.
    dailies: HashMap<u64, (Instant, u32)>,
}

/// How long a day of answers lasts.
const DAY: Duration = Duration::from_hours(24);

/// The window after `strikes` full windows: doubled each time, up to [`MAX_WAIT`].
fn window_after(strikes: u32) -> Duration {
    let doubled = BASE_WINDOW.saturating_mul(2_u32.saturating_pow(strikes.min(16)));
    doubled.min(MAX_WAIT)
}

fn seconds_left(until: Instant, now: Instant) -> u64 {
    let wait = until.saturating_duration_since(now);
    (wait.as_secs() + u64::from(wait.subsec_nanos() > 0)).max(1)
}

/// The limit on failed tries per account, person, network and half-finished sign-in.
pub struct FailureLimits {
    limit: u32,
    tables: Mutex<Tables>,
    /// Random for each start, so the hash of an account name cannot be computed elsewhere.
    hasher: RandomState,
}

impl FailureLimits {
    /// A limit that allows `failures_per_minute` tries per key and window. `0` refuses every key.
    #[must_use]
    pub fn new(failures_per_minute: u32) -> Arc<Self> {
        Arc::new(Self {
            limit: failures_per_minute,
            tables: Mutex::new(Tables {
                trackers: HashMap::new(),
                tallies: HashMap::new(),
                dailies: HashMap::new(),
            }),
            hasher: RandomState::new(),
        })
    }

    fn key(&self, key: &str) -> u64 {
        self.hasher.hash_one(key.trim().to_lowercase())
    }

    fn lock(&self) -> MutexGuard<'_, Tables> {
        self.tables.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn reserve_at(&self, key: &str, now: Instant) -> Result<bool, u64> {
        let grows = !key.starts_with(NETWORK_KEY_PREFIX);
        let key = self.key(key);
        let mut tables = self.lock();
        if tables.trackers.len() >= MAX_KEYS && !tables.trackers.contains_key(&key) {
            tables
                .trackers
                .retain(|_, tracker| now < tracker.forget_at());
            if tables.trackers.len() >= MAX_KEYS {
                return Err(BASE_WINDOW.as_secs());
            }
        }
        let tracker = tables.trackers.entry(key).or_insert(Tracker {
            started: now,
            window: BASE_WINDOW,
            count: 0,
            strikes: 0,
            grows,
        });
        if now >= tracker.started + tracker.window {
            // A window that ended with a failure makes the next one longer, however few failures
            // it had, as long as the key keeps failing; a quiet time forgives.
            let strikes = if grows && tracker.count >= 1 && now < tracker.forget_at() {
                tracker.strikes.saturating_add(1)
            } else {
                0
            };
            *tracker = Tracker {
                started: now,
                window: window_after(strikes),
                count: 0,
                strikes,
                grows,
            };
        }
        if tracker.count >= self.limit {
            return Err(seconds_left(tracker.started + tracker.window, now));
        }
        tracker.count += 1;
        Ok(tracker.count == self.limit)
    }

    fn daily_wait_at(&self, key: &str, limit: u32, now: Instant) -> Option<u64> {
        let key = self.key(key);
        let mut tables = self.lock();
        let (began, count) = *tables.dailies.get(&key)?;
        if now >= began + DAY {
            tables.dailies.remove(&key);
            return None;
        }
        (count >= limit).then(|| seconds_left(began + DAY, now))
    }

    fn daily_spend_at(&self, key: &str, now: Instant) {
        let key = self.key(key);
        let mut tables = self.lock();
        if tables.dailies.len() >= MAX_KEYS && !tables.dailies.contains_key(&key) {
            tables.dailies.retain(|_, (began, _)| now < *began + DAY);
            if tables.dailies.len() >= MAX_KEYS {
                // Fail closed: the table is full of running days, so this person is counted as
                // having used theirs up.
                tables.dailies.insert(key, (now, u32::MAX));
                return;
            }
        }
        let entry = tables.dailies.entry(key).or_insert((now, 0));
        if now >= entry.0 + DAY {
            *entry = (now, 0);
        }
        entry.1 = entry.1.saturating_add(1);
    }

    fn tally_at(&self, key: &str, now: Instant) -> u32 {
        let key = self.key(key);
        let mut tables = self.lock();
        if tables.tallies.len() >= MAX_KEYS && !tables.tallies.contains_key(&key) {
            tables
                .tallies
                .retain(|_, (last, _)| now.saturating_duration_since(*last) < TALLY_FORGET);
            if tables.tallies.len() >= MAX_KEYS {
                // Fail closed: the caller gives the half-finished sign-in up.
                return u32::MAX;
            }
        }
        let entry = tables.tallies.entry(key).or_insert((now, 0));
        if now.saturating_duration_since(entry.0) >= TALLY_FORGET {
            *entry = (now, 0);
        }
        *entry = (now, entry.1.saturating_add(1));
        entry.1
    }
}

impl FailureLimit for FailureLimits {
    fn reserve(&self, key: &str) -> Result<bool, u64> {
        self.reserve_at(key, Instant::now())
    }

    fn release(&self, key: &str) {
        let key = self.key(key);
        if let Some(tracker) = self.lock().trackers.get_mut(&key) {
            tracker.count = tracker.count.saturating_sub(1);
        }
    }

    fn clear(&self, key: &str) {
        let key = self.key(key);
        self.lock().trackers.remove(&key);
    }

    fn tally(&self, key: &str) -> u32 {
        self.tally_at(key, Instant::now())
    }

    fn daily_wait(&self, key: &str, limit: u32) -> Option<u64> {
        self.daily_wait_at(key, limit, Instant::now())
    }

    fn daily_spend(&self, key: &str) {
        self.daily_spend_at(key, Instant::now());
    }

    fn forget(&self, key: &str) {
        let key = self.key(key);
        self.lock().tallies.remove(&key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(seconds: u64) -> Duration {
        Duration::from_secs(seconds)
    }

    #[test]
    fn sec_14_a_try_is_counted_when_it_is_reserved_and_the_last_one_says_so() {
        let failures = FailureLimits::new(3);
        let now = Instant::now();
        assert_eq!(failures.reserve_at("Ada@Example.test", now), Ok(false));
        assert_eq!(failures.reserve_at(" ada@example.test ", now), Ok(false));
        assert_eq!(failures.reserve_at("ADA@example.test", now), Ok(true));
        // The allowance is used up: the next try waits for the rest of the window.
        let wait = failures
            .reserve_at("ada@example.test", now + secs(20))
            .unwrap_err();
        assert_eq!(wait, 40);
        // Another key is not affected.
        assert_eq!(failures.reserve_at("bob@example.test", now), Ok(false));
    }

    #[test]
    fn sec_m2_the_answers_of_a_day_run_out_and_come_back_after_the_day() {
        let failures = FailureLimits::new(10);
        let now = Instant::now();
        for _ in 0..3 {
            assert_eq!(failures.daily_wait_at("handle:ada", 3, now), None);
            failures.daily_spend_at("handle:ada", now);
        }
        assert_eq!(
            failures.daily_wait_at("handle:ada", 3, now + secs(3_600)),
            Some(23 * 3_600)
        );
        // Another person has their own day, and a new day starts with the next answer.
        assert_eq!(failures.daily_wait_at("handle:bob", 3, now), None);
        assert_eq!(
            failures.daily_wait_at("handle:ada", 3, now + DAY + secs(1)),
            None
        );
    }

    #[test]
    fn sec_14_a_success_clears_a_key_and_gives_one_try_back_to_a_shared_key() {
        let failures = FailureLimits::new(2);
        let now = Instant::now();
        failures.reserve_at("ada@example.test", now).unwrap();
        failures.clear("ada@example.test");
        assert_eq!(failures.reserve_at("ada@example.test", now), Ok(false));
        assert_eq!(failures.reserve_at("ada@example.test", now), Ok(true));
        assert!(failures.reserve_at("ada@example.test", now).is_err());
        failures.clear("ada@example.test");
        assert!(failures.reserve_at("ada@example.test", now).is_ok());

        // A network is shared: a success gives back its own try only.
        failures.reserve_at("net:203.0.113.0", now).unwrap();
        failures.reserve_at("net:203.0.113.0", now).unwrap();
        failures.release("net:203.0.113.0");
        assert!(failures.reserve_at("net:203.0.113.0", now).is_ok());
        assert!(failures.reserve_at("net:203.0.113.0", now).is_err());
    }

    #[test]
    fn sec_14_each_full_window_of_failures_doubles_the_next_one_up_to_fifteen_minutes() {
        let failures = FailureLimits::new(2);
        let mut now = Instant::now();
        let mut waits = Vec::new();
        for _ in 0..7 {
            failures.reserve_at("ada@example.test", now).unwrap();
            failures.reserve_at("ada@example.test", now).unwrap();
            let wait = failures.reserve_at("ada@example.test", now).unwrap_err();
            waits.push(wait);
            // The wait is over; the key fails again at once.
            now += secs(wait);
        }
        assert_eq!(waits, [60, 120, 240, 480, 900, 900, 900]);
    }

    #[test]
    fn sec_14_pacing_the_tries_just_under_the_allowance_does_not_dodge_the_growing_wait() {
        let failures = FailureLimits::new(3);
        let start = Instant::now();
        let mut now = start;
        let mut window = 60;
        // Two failures a window, never the third: the windows grow all the same.
        for expected in [60, 120, 240, 480] {
            failures.reserve_at("ada@example.test", now).unwrap();
            failures.reserve_at("ada@example.test", now).unwrap();
            now += secs(window);
            window = expected * 2;
        }
        // The fifth window is the one after four with failures: 15 minutes at most, here 960 -> 900.
        failures.reserve_at("ada@example.test", now).unwrap();
        failures.reserve_at("ada@example.test", now).unwrap();
        failures.reserve_at("ada@example.test", now).unwrap();
        assert_eq!(
            failures.reserve_at("ada@example.test", now).unwrap_err(),
            900
        );
        // A key that did not pace gets the first window again only after a quiet time ...
        let quiet = now + secs(900) + MAX_WAIT + secs(1);
        failures.reserve_at("ada@example.test", quiet).unwrap();
        failures.reserve_at("ada@example.test", quiet).unwrap();
        failures.reserve_at("ada@example.test", quiet).unwrap();
        assert_eq!(
            failures.reserve_at("ada@example.test", quiet).unwrap_err(),
            60
        );
        // ... or after a success.
        let mut later = quiet + secs(60);
        failures.reserve_at("ada@example.test", later).unwrap();
        failures.clear("ada@example.test");
        later += secs(1);
        for _ in 0..3 {
            failures.reserve_at("ada@example.test", later).unwrap();
        }
        assert_eq!(
            failures.reserve_at("ada@example.test", later).unwrap_err(),
            60
        );
    }

    #[test]
    fn sec_14_the_window_of_a_network_stays_a_minute_however_often_it_fails() {
        let failures = FailureLimits::new(2);
        let mut now = Instant::now();
        let mut waits = Vec::new();
        for _ in 0..4 {
            failures.reserve_at("net:203.0.113.0", now).unwrap();
            failures.reserve_at("net:203.0.113.0", now).unwrap();
            let wait = failures.reserve_at("net:203.0.113.0", now).unwrap_err();
            waits.push(wait);
            now += secs(wait);
        }
        // Many people share a network, so it limits a flood and never grows into a long wait.
        assert_eq!(waits, [60, 60, 60, 60]);
    }

    #[test]
    fn sec_14_a_limit_of_zero_refuses_every_key() {
        let failures = FailureLimits::new(0);
        assert!(
            failures
                .reserve_at("ada@example.test", Instant::now())
                .is_err()
        );
    }

    #[test]
    fn sec_14_a_full_table_refuses_new_keys_and_keeps_running_counters() {
        let failures = FailureLimits::new(5);
        let now = Instant::now();
        for number in 0..MAX_KEYS {
            failures.reserve_at(&format!("key{number}"), now).unwrap();
        }
        assert!(failures.reserve_at("new key", now).is_err());
        assert!(failures.reserve_at("key7", now).is_ok());
        // Counters whose window and quiet time are over make room.
        assert!(
            failures
                .reserve_at("new key", now + MAX_WAIT + secs(61))
                .is_ok()
        );
    }

    #[test]
    fn sec_14_a_tally_counts_without_a_window_until_it_is_forgotten() {
        let failures = FailureLimits::new(3);
        let now = Instant::now();
        assert_eq!(failures.tally_at("pending:1", now), 1);
        assert_eq!(failures.tally_at("pending:1", now + secs(300)), 2);
        assert_eq!(failures.tally_at("pending:2", now), 1);
        failures.forget("pending:1");
        assert_eq!(failures.tally_at("pending:1", now), 1);
        // Left alone for longer than the forgetting time, a tally starts over.
        assert_eq!(
            failures.tally_at("pending:1", now + TALLY_FORGET + secs(1)),
            1
        );
    }
}
