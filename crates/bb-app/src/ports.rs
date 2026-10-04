//! Ports: what the use cases need from the outside, as traits. Adapters (`bb-store`, `bb-files`,
//! …) implement them; `bb-server` wires them up (architecture §4).

use std::future::Future;

use bb_core::event::DomainEvent;

/// Writes a change together with its domain event (ADR-0007).
///
/// This is the only way for a command use case to change stored data: the change, the event and
/// the outbox entry are committed together or not at all. A command use case calls
/// `record_change` and nothing else writes (ARCH-EVENTS).
pub trait ChangeStore {
    /// The open transaction the change runs in; the adapter decides what it is.
    type Transaction;
    /// What a failed change or a failed commit reports.
    type Error;

    /// Runs `change`, appends `event` and its outbox entry, and commits all of it. If `change`
    /// fails, nothing is written.
    ///
    /// # Errors
    /// Returns the error of `change`, or the error of the database.
    fn record_change<T>(
        &self,
        event: &DomainEvent,
        change: impl AsyncFnOnce(&mut Self::Transaction) -> Result<T, Self::Error>,
    ) -> impl Future<Output = Result<T, Self::Error>>;
}
