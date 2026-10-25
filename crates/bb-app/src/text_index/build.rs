//! The outbox consumer that keeps the index (ISS-13, ADR-0007): it reads the events it has not
//! finished, names the issues whose texts they touch (`map.rs`) and moves its place on. It is
//! idempotent, so reading an event again (after a restart, or when two readers catch up at the
//! same moment) changes nothing, and it resumes from the place it recorded. It never holds up a
//! change: the change is committed before the consumer sees it.
//!
//! It takes the store and the time and nothing else: it is no use case of a person.

use bb_core::text_index::BATCH;
use jiff::Timestamp;

use super::map::{Touched, touched};
use super::store::TextIndexStore;
use crate::auth::{AppError, AuthStore, Ctx, internal};

/// Reads at most `batch` events the index has not finished and updates the issues they touch.
/// Returns how many events it read: 0 when it has caught up.
///
/// # Errors
/// The error of the store.
pub async fn catch_up_batch<S: TextIndexStore>(
    store: &S,
    now: Timestamp,
    batch: u64,
) -> Result<u64, S::Error> {
    let events = store.text_index_events(batch).await?;
    let Some(last) = events.last().map(|(position, _)| *position) else {
        return Ok(0);
    };
    let mut named: Vec<Touched> = Vec::new();
    for (_, event) in &events {
        if let Some(one) = touched(event)
            && !named.contains(&one)
        {
            named.push(one);
        }
    }
    store.store_text_index(&named, last, now).await?;
    Ok(u64::try_from(events.len()).unwrap_or(u64::MAX))
}

/// Reads every event the index has not finished, a batch at a time.
///
/// # Errors
/// The error of the store.
pub async fn catch_up<S: TextIndexStore>(store: &S, now: Timestamp) -> Result<u64, S::Error> {
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
    /// Catches the index up with every event not yet read: the daily job `text-index.catch-up`
    /// does it (also at every start), every round of the jobs does it, and a search does it
    /// first. Returns how many events it read.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub async fn catch_up_text_index(&self) -> Result<u64, AppError> {
        catch_up(self.store(), self.now())
            .await
            .map_err(|error| internal::<S>(&error))
    }

    /// Builds the whole index again from the stored texts. Returns how many issues it read.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub async fn rebuild_text_index(&self) -> Result<u64, AppError> {
        self.store()
            .rebuild_text_index()
            .await
            .map_err(|error| internal::<S>(&error))
    }
}
