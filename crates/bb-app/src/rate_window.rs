//! A fixed window that counts events per key (SEC-14): the one window type of the server. The rate
//! limits of the API (`bb-api`) and the calls of AI apps (`bb-app`) both use it, so a limit is
//! counted one way everywhere. It holds no clock: the caller passes the moment, which keeps it pure
//! and its tests exact.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

/// How long a window lasts when nothing else is said.
pub const PERIOD: Duration = Duration::from_mins(1);

/// Each window remembers at most this many clients. A flood of invented addresses must not be
/// able to use up the memory. When the table is full, expired entries go first; counters of
/// clients that are still within their minute are never thrown away. If it is still full, see
/// [`Overflow`].
pub const MAX_CLIENTS: usize = 100_000;

/// A full table is swept for expired counters at most this often.
pub const SWEEP_EVERY: Duration = Duration::from_secs(1);

struct Entry {
    started: Instant,
    count: u32,
}

/// What a window does with a new key when its table is full of unexpired counters.
pub enum Overflow<K> {
    /// All such keys share one counter. Used for the general limit: a flood of invented clients
    /// ends up sharing one limit, while the clients already counted keep theirs.
    Share(K),
    /// The new key is refused (counts as over the limit). Used for sign-in: fail closed.
    Refuse,
}

/// Counts events per key in fixed windows.
pub struct Window<K> {
    limit: u32,
    /// How long a window lasts.
    period: Duration,
    overflow: Overflow<K>,
    table: Mutex<Table<K>>,
}

/// The counters of one window.
struct Table<K> {
    entries: HashMap<K, Entry>,
    /// When expired entries were last swept out. A full table is swept at most once per
    /// [`SWEEP_EVERY`], so a flood cannot make every request scan 100,000 entries.
    last_sweep: Option<Instant>,
    /// How many sweeps ran (tests count them).
    #[cfg(test)]
    sweeps: u32,
}

impl<K: Hash + Eq + Clone> Window<K> {
    /// A window of one minute that allows `limit` events per key.
    #[must_use]
    pub fn new(limit: u32, overflow: Overflow<K>) -> Self {
        Self::lasting(PERIOD, limit, overflow)
    }

    /// A window of the given length.
    #[must_use]
    pub fn lasting(period: Duration, limit: u32, overflow: Overflow<K>) -> Self {
        Self {
            limit,
            period,
            overflow,
            table: Mutex::new(Table {
                entries: HashMap::new(),
                last_sweep: None,
                #[cfg(test)]
                sweeps: 0,
            }),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Table<K>> {
        self.table.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Says whether the key has room for one more event, and counts nothing: for a call that has to
    /// fit two windows at once and counts against neither when one is full.
    ///
    /// # Errors
    /// The seconds to wait (at least one) when the key is over the limit.
    pub fn room(&self, key: &K, now: Instant) -> Result<(), u64> {
        let table = self.lock();
        match table.entries.get(key) {
            Some(entry)
                if now.duration_since(entry.started) < self.period && entry.count >= self.limit =>
            {
                Err(seconds_left(entry.started, now, self.period))
            }
            _ => Ok(()),
        }
    }

    /// Forgets what the key has counted, so its next event starts a new window: an agent that its
    /// person lets work again starts from nothing.
    pub fn forget(&self, key: &K) {
        self.lock().entries.remove(key);
    }

    /// Counts one event.
    ///
    /// # Errors
    /// The seconds to wait (at least one) when the key is over the limit.
    pub fn hit(&self, key: K, now: Instant) -> Result<(), u64> {
        self.hit_within(key, now, self.limit)
    }

    /// Says whether the key has room for one more event under a limit that belongs to the key, and
    /// counts nothing.
    ///
    /// # Errors
    /// The seconds to wait (at least one) when the key is over `limit`.
    pub fn room_within(&self, key: &K, now: Instant, limit: u32) -> Result<(), u64> {
        let table = self.lock();
        match table.entries.get(key) {
            Some(entry)
                if now.duration_since(entry.started) < self.period && entry.count >= limit =>
            {
                Err(seconds_left(entry.started, now, self.period))
            }
            _ => Ok(()),
        }
    }

    /// Counts one event against a limit that belongs to the key (the limit an agent has was set for
    /// it, not for the window).
    ///
    /// # Errors
    /// The seconds to wait (at least one) when the key is over `limit`.
    pub fn hit_within(&self, key: K, now: Instant, limit: u32) -> Result<(), u64> {
        let mut table = self.lock();
        let mut key = key;
        if table.entries.len() >= MAX_CLIENTS && !table.entries.contains_key(&key) {
            create_room(&mut table, now, self.period);
            if table.entries.len() >= MAX_CLIENTS {
                match &self.overflow {
                    Overflow::Share(shared) => key = shared.clone(),
                    Overflow::Refuse => return Err(self.period.as_secs()),
                }
            }
        }
        let entry = table.entries.entry(key).or_insert(Entry {
            started: now,
            count: 0,
        });
        if now.duration_since(entry.started) >= self.period {
            *entry = Entry {
                started: now,
                count: 0,
            };
        }
        if entry.count >= limit {
            return Err(seconds_left(entry.started, now, self.period));
        }
        entry.count += 1;
        Ok(())
    }
}

/// The seconds a key that started its window at `started` still has to wait: at least one.
fn seconds_left(started: Instant, now: Instant, period: Duration) -> u64 {
    let wait = period.saturating_sub(now.duration_since(started));
    (wait.as_secs() + u64::from(wait.subsec_nanos() > 0)).max(1)
}

/// Throws out the counters whose window is over, when the table is full. A full table is swept
/// at most once per [`SWEEP_EVERY`]; counters that are still running are never thrown away.
fn create_room<K>(table: &mut Table<K>, now: Instant, period: Duration) {
    let due = table
        .last_sweep
        .is_none_or(|last| now.saturating_duration_since(last) >= SWEEP_EVERY);
    if due {
        table
            .entries
            .retain(|_, entry| now.duration_since(entry.started) < period);
        table.last_sweep = Some(now);
        #[cfg(test)]
        {
            table.sweeps += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fill(window: &Window<u32>, now: Instant) {
        for key in 0..u32::try_from(MAX_CLIENTS).unwrap() {
            window.hit(key, now).unwrap();
        }
    }

    #[test]
    fn ai_37_a_limit_of_the_key_is_counted_at_the_edge_and_forgotten_on_request() {
        let window: Window<u32> = Window::new(u32::MAX, Overflow::Refuse);
        let now = Instant::now();
        for _ in 0..3 {
            window.hit_within(1, now, 3).unwrap();
        }
        assert!(window.hit_within(1, now, 3).is_err());
        // Another key has its own count, and a higher limit lets the same key go on.
        window.hit_within(2, now, 3).unwrap();
        window.hit_within(1, now, 4).unwrap();
        window.forget(&1);
        window.hit_within(1, now, 3).unwrap();
    }

    #[test]
    fn sec_14_a_full_table_never_forgets_counters_that_are_still_running() {
        let window: Window<u32> = Window::new(1, Overflow::Share(u32::MAX));
        let now = Instant::now();
        fill(&window, now);
        // A client already counted keeps its counter, also while the table is full.
        assert!(window.hit(7, now).is_err());
        // New clients share one counter: the first passes, the next are over the limit.
        window.hit(u32::MAX - 1, now).unwrap();
        assert!(window.hit(u32::MAX - 2, now).is_err());
        assert!(window.table.lock().unwrap().entries.len() <= MAX_CLIENTS + 1);
        assert!(window.hit(7, now).is_err());
        // Expired counters make room again.
        let later = now + PERIOD + Duration::from_secs(1);
        window.hit(u32::MAX - 3, later).unwrap();
        window.hit(8, later).unwrap();
    }

    #[test]
    fn sec_14_a_full_table_is_swept_at_most_once_a_second_not_on_every_request() {
        let window: Window<u32> = Window::new(1, Overflow::Share(u32::MAX));
        let now = Instant::now();
        fill(&window, now);
        for new in 0..1000 {
            let _ = window.hit(
                u32::try_from(MAX_CLIENTS).unwrap() + new,
                now + Duration::from_millis(u64::from(new)),
            );
        }
        // 1000 requests over about one second: one sweep, maybe two at the second's edge.
        let sweeps = window.table.lock().unwrap().sweeps;
        assert!((1..=2).contains(&sweeps), "{sweeps}");
        // Much later the sweep runs again and frees the expired counters.
        let later = now + PERIOD + Duration::from_secs(5);
        window.hit(u32::MAX - 5, later).unwrap();
        assert!(window.table.lock().unwrap().entries.len() < 10);
    }

    #[test]
    fn sec_14_sign_in_refuses_new_keys_when_the_table_is_full() {
        let window: Window<u32> = Window::new(5, Overflow::Refuse);
        let now = Instant::now();
        fill(&window, now);
        let wait = window.hit(u32::MAX, now).unwrap_err();
        assert_eq!(wait, PERIOD.as_secs());
        // Known keys still work; after expiry a new key is accepted.
        window.hit(3, now).unwrap();
        window
            .hit(u32::MAX, now + PERIOD + Duration::from_secs(1))
            .unwrap();
    }
}
