//! Ports: what the use cases need from the outside, as traits. Adapters (`bb-store`, `bb-files`,
//! …) implement them; `bb-server` wires them up (architecture §4).

use std::future::Future;
use std::pin::Pin;

use bb_core::event::DomainEvent;

/// A future that can move between threads: what a change hands back to the store, so the
/// use cases stay `Send` whatever store they run on (HTTP handlers need that).
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Writes a change together with its domain event (ADR-0007).
///
/// This is the only way for a command use case to change stored data: the change, the event and
/// the outbox entry are committed together or not at all. A command use case calls
/// `record_change` and nothing else writes (ARCH-EVENTS).
///
/// The change is a closure that returns a boxed future (`Box::pin(async move { … })`): that is
/// what lets the compiler know the whole write is `Send`, which a plain async closure cannot say
/// in a trait.
pub trait ChangeStore: Send + Sync {
    /// The open transaction the change runs in; the adapter decides what it is.
    type Transaction: Send;
    /// What a failed change or a failed commit reports.
    type Error: Send;

    /// Runs `change`, appends `event` and its outbox entry, and commits all of it. If `change`
    /// fails, nothing is written.
    ///
    /// # Errors
    /// Returns the error of `change`, or the error of the database.
    fn record_change<'a, T: Send + 'a>(
        &'a self,
        event: &'a DomainEvent,
        change: impl for<'t> FnOnce(&'t mut Self::Transaction) -> BoxFuture<'t, Result<T, Self::Error>>
        + Send
        + 'a,
    ) -> impl Future<Output = Result<T, Self::Error>> + Send + 'a;
}

/// The file in which a server without a console saves the setup code of the first start
/// (`setup-code.txt` in the data folder). The use case that makes the first administrator removes
/// it in the same run, because the code in it is dead from then on (AUTH-09, M1-F58); the program
/// that wires the server implements it, because only it knows the data folder.
pub trait SetupCodeFile: Send + Sync {
    /// Removes the file. A file that is not there is no error.
    ///
    /// # Errors
    /// The error of the file system when the file is there and cannot be removed.
    fn remove(&self) -> std::io::Result<()>;
}
