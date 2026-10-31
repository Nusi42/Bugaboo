//! The mark that the upkeep of the database (`Database::refresh_statistics`, ADR-0017) sets while it
//! may hold SQLite's write lock, and the one place a write that met SQLite's busy answer is tried
//! again: only while the mark is set, so the busy timeout of every writer stays as it is and a
//! write that waits for another person's change is not held up longer.

use std::future::Future;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

use crate::error::StoreError;

/// How often a write that met SQLite's busy answer while the upkeep held the lock is tried, the
/// first try included. Each try waits the busy timeout of the connection first.
const BUSY_WRITE_TRIES: u32 = 3;

/// How long a write waits before it is tried again.
const BUSY_WRITE_WAIT: Duration = Duration::from_millis(500);

/// Whether the upkeep holds the database, and how often a write was tried again because of it.
/// All clones of a `Database` share one.
#[derive(Debug, Default)]
pub(super) struct Upkeep {
    /// How many rounds of upkeep run now.
    running: AtomicUsize,
    /// How many rounds of upkeep have ended: a write that began before one ended met it.
    ended: AtomicU64,
    /// How many times a write was tried again (read by the tests).
    tried_again: AtomicU64,
}

/// A round of upkeep that runs while this lives.
pub(super) struct Running<'a>(&'a Upkeep);

impl Drop for Running<'_> {
    fn drop(&mut self) {
        self.0.ended.fetch_add(1, Ordering::SeqCst);
        self.0.running.fetch_sub(1, Ordering::SeqCst);
    }
}

/// What a write saw of the upkeep when it began.
#[derive(Clone, Copy)]
struct Seen(u64);

impl Upkeep {
    /// Marks a round of upkeep until the value is dropped.
    pub(super) fn mark(&self) -> Running<'_> {
        self.running.fetch_add(1, Ordering::SeqCst);
        Running(self)
    }

    /// What a write sees of the upkeep as it begins.
    fn seen(&self) -> Seen {
        Seen(self.ended.load(Ordering::SeqCst))
    }

    /// Whether a round of upkeep ran at some moment since `seen`: then a busy answer may be its
    /// lock, and the write is tried again (and counted).
    fn ran_since(&self, seen: Seen) -> bool {
        let ran =
            self.running.load(Ordering::SeqCst) > 0 || self.ended.load(Ordering::SeqCst) != seen.0;
        if ran {
            self.tried_again.fetch_add(1, Ordering::SeqCst);
        }
        ran
    }

    /// How many times a write was tried again since the database was opened.
    #[cfg(any(test, feature = "testing"))]
    pub(super) fn tried_again(&self) -> u64 {
        self.tried_again.load(Ordering::SeqCst)
    }

    /// Runs `write`, which must be one whole transaction or one statement on its own (so a try
    /// that met the busy answer changed nothing), and tries it again while the upkeep holds the
    /// lock, [`BUSY_WRITE_TRIES`] times at most. A write that stays busy is [`StoreError::Busy`].
    pub(super) async fn write<T, Fut>(&self, write: impl FnMut() -> Fut) -> Result<T, StoreError>
    where
        Fut: Future<Output = Result<T, StoreError>>,
    {
        let seen = self.seen();
        match while_locked(
            BUSY_WRITE_TRIES,
            BUSY_WRITE_WAIT,
            || self.ran_since(seen),
            write,
        )
        .await
        {
            Err(error) if error.is_locked() => Err(StoreError::Busy),
            other => other,
        }
    }
}

/// Runs `attempt` and tries it again while SQLite answers that the database is locked and
/// `again()` allows it, `tries` times at most with `wait` between: the one loop for a locked
/// database (opening a new file, and a write while the upkeep holds the lock).
pub(super) async fn while_locked<T, Fut>(
    tries: u32,
    wait: Duration,
    again: impl Fn() -> bool,
    mut attempt: impl FnMut() -> Fut,
) -> Result<T, StoreError>
where
    Fut: Future<Output = Result<T, StoreError>>,
{
    let mut tried = 1;
    loop {
        match attempt().await {
            Err(error) if error.is_locked() && tried < tries && again() => {
                tried += 1;
                tokio::time::sleep(wait).await;
            }
            other => return other,
        }
    }
}
