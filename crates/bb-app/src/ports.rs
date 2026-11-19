//! Ports: what the use cases need from the outside, as traits. Adapters (`bb-store`, `bb-files`,
//! …) implement them; `bb-server` wires them up (architecture §4).

use std::future::Future;
use std::pin::Pin;

use bb_core::event::DomainEvent;
use bb_core::keeping::{KeepKind, KeptUntil};
use jiff::civil::Date;

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
    ) -> impl Future<Output = Result<T, Self::Error>> + Send + 'a {
        self.record_changes(std::slice::from_ref(event), change)
    }

    /// The same for a change that has more than one event of its own, or none besides the ones it
    /// adds inside (`Transaction` follow-ups): a step of a job that renews the IDs of many issues
    /// writes one event for each issue and nothing else.
    ///
    /// # Errors
    /// Returns the error of `change`, or the error of the database.
    fn record_changes<'a, T: Send + 'a>(
        &'a self,
        events: &'a [DomainEvent],
        change: impl for<'t> FnOnce(&'t mut Self::Transaction) -> BoxFuture<'t, Result<T, Self::Error>>
        + Send
        + 'a,
    ) -> impl Future<Output = Result<T, Self::Error>> + Send + 'a;

    /// Runs `read` in one snapshot: everything it reads belongs together, and nothing is written.
    /// This is how a read that is also needed inside a change (`queries!` in the ports) is
    /// written once: the change runs it on its own transaction, a plain read runs it here.
    ///
    /// # Errors
    /// Returns the error of `read`, or the error of the database.
    fn read_snapshot<'a, T: Send + 'a>(
        &'a self,
        read: impl for<'t> FnOnce(&'t mut Self::Transaction) -> BoxFuture<'t, Result<T, Self::Error>>
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

/// The walk of a person's data (ADR-0044, decision 1): the one way to find the rows that name a
/// person, read by *Download my data*, *Remove a person* and the removal plan. The store answers it
/// from the links of the entities; nothing else searches rows by person.
pub trait PersonData: ChangeStore {
    /// The kinds of record that name `person`, each with how many of its rows do; a kind with no
    /// such row is left out.
    ///
    /// # Errors
    /// The error of the store.
    fn kinds_of(
        &self,
        person: bb_core::id::UserId,
    ) -> impl Future<Output = Result<Vec<crate::privacy::walk::KindOfRows>, Self::Error>> + Send;

    /// How many rows of `kind` name `person` by a link that is one of `links` (what the person is
    /// to the row): the rows of a kind that a removal deletes, apart from those that name the
    /// person some other way.
    ///
    /// # Errors
    /// The error of the store; a kind that is not a table is no error but none.
    fn rows_linked(
        &self,
        person: bb_core::id::UserId,
        kind: &str,
        links: &[bb_core::personal::Link],
    ) -> impl Future<Output = Result<u64, Self::Error>> + Send;

    /// A page of the rows of `kind` that name `person`, starting at `from` (0 for the first page),
    /// at most [`crate::privacy::walk::ROWS_PAGE`] rows.
    ///
    /// # Errors
    /// The error of the store; a kind that is not a table is no error but an empty page.
    fn person_rows(
        &self,
        person: bb_core::id::UserId,
        kind: &str,
        from: u64,
    ) -> impl Future<Output = Result<crate::privacy::walk::RowsPage, Self::Error>> + Send;
}

/// How long each kind of data is kept (ADR-0045): the last day a record of a kind is kept, from the
/// record's date and the organization's keep periods. The window of *Remove a person* and the trash
/// read it; `crate::keeping` answers it from the settings, so no period is counted twice.
pub trait KeepPeriods {
    /// The last day a record of `kind` dated `date` is kept, or no fixed end: backups, which the
    /// rotation keeps whatever their age, and a day past the end of the calendar.
    fn kept_until(&self, kind: KeepKind, date: Date) -> KeptUntil;
}
