//! The queue of the writers of one SQLite database inside one program.
//!
//! SQLite lets one writer at a time in and makes the others wait by sleeping and asking again,
//! which is not fair: with many writers one can be passed by again and again until its busy
//! timeout ends, and every writer that waits holds a connection of the small pool, so the readers
//! wait for a connection too. The writers of one program therefore queue here first, in the order
//! they came, and take a connection only when it is their turn; SQLite's own waiting is left for
//! writers of other programs (PERF-01, architecture §7).

use std::sync::Arc;

use tokio::sync::{Mutex, OwnedMutexGuard};

use super::Dialect;

/// The one place in the queue that writes: all clones of a `Database` share it. The mutex of tokio
/// lets in first come, first served.
#[derive(Clone, Debug, Default)]
pub(super) struct WriterGate(Arc<Mutex<()>>);

/// The turn of one writer: it ends when this is dropped, after the writer's transaction has.
#[derive(Debug)]
pub(crate) struct Turn {
    _guard: OwnedMutexGuard<()>,
}

impl WriterGate {
    /// Waits for the turn of the next writer on SQLite; PostgreSQL has many writers and no queue.
    pub(super) async fn wait_on(&self, dialect: Dialect) -> Option<Turn> {
        match dialect {
            Dialect::Sqlite => Some(Turn {
                _guard: Arc::clone(&self.0).lock_owned().await,
            }),
            Dialect::Postgres => None,
        }
    }
}
