//! The loop every reader of the outbox shares (ADR-0007): the history, the notifications and the
//! index of the text search read their events a batch at a time until one batch comes back short.

use std::future::Future;

use bb_core::outbox::OUTBOX_BATCH;

/// Calls `one_batch` with the size of a batch until it reads fewer events than that, and returns
/// how many events it read in all.
///
/// # Errors
/// The error of `one_batch`.
pub(crate) async fn read_all<E, F: Future<Output = Result<u64, E>>>(
    mut one_batch: impl FnMut(u64) -> F,
) -> Result<u64, E> {
    let mut read = 0_u64;
    loop {
        let events = one_batch(OUTBOX_BATCH).await?;
        read = read.saturating_add(events);
        if events < OUTBOX_BATCH {
            return Ok(read);
        }
    }
}
