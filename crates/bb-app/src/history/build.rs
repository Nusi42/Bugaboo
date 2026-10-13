//! The outbox consumer that builds the history (ISS-06, ADR-0007): it reads the events that it has
//! not finished, makes the rows of the history from them (`map.rs`) and moves its place on. It is
//! idempotent: a row is made once for each event and issue, so reading an event again (after a
//! restart, or when two readers catch up at the same moment) changes nothing, and it resumes from
//! the place it recorded.
//!
//! It takes the store and the time and nothing else: it is no use case of a person (it has no
//! actor and writes no domain event of its own), so it is not a command of ARCH-EVENTS.

use bb_core::history::BATCH;
use jiff::Timestamp;

use super::map::rows_of;
use super::store::HistoryStore;
use crate::auth::{AppError, AuthStore, Ctx, internal};

/// Reads at most `batch` events the history has not finished and makes their rows. Returns how
/// many events it read: 0 when it has caught up.
///
/// # Errors
/// The error of the store.
pub async fn catch_up_batch<S: HistoryStore>(
    store: &S,
    now: Timestamp,
    batch: u64,
) -> Result<u64, S::Error> {
    let events = store.history_events(batch).await?;
    let Some(last) = events.last().map(|(position, _)| *position) else {
        return Ok(0);
    };
    let rows: Vec<_> = events
        .iter()
        .flat_map(|(position, event)| rows_of(*position, event))
        .collect();
    store.store_history(&rows, last, now).await?;
    Ok(u64::try_from(events.len()).unwrap_or(u64::MAX))
}

/// Reads every event the history has not finished, a batch at a time.
///
/// # Errors
/// The error of the store.
pub async fn catch_up<S: HistoryStore>(store: &S, now: Timestamp) -> Result<u64, S::Error> {
    let mut read = 0_u64;
    loop {
        let events = catch_up_batch(store, now, BATCH).await?;
        read = read.saturating_add(events);
        if events < BATCH {
            return Ok(read);
        }
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Catches the history up with every event not yet read: the server does it when it starts
    /// and once a day after that, and a reader of a history does it first (M3-F71). Returns how
    /// many events it read.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub async fn catch_up_history(&self) -> Result<u64, AppError> {
        catch_up(self.store(), self.now())
            .await
            .map_err(|error| internal::<S>(&error))
    }
}
