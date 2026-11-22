//! A SQLite connection of the pool while a read with a time limit runs on it.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use sqlx::Sqlite;

use crate::error::StoreError;

/// A pooled SQLite connection while a timed read runs on it. If the read does not end normally (the
/// future is dropped, a client went away) the read is told to stop at its next step and the
/// connection is closed instead of going back to the pool, so no limit of a read that is gone can
/// stop a later statement of anybody else.
pub(super) struct Timed {
    connection: Option<sqlx::pool::PoolConnection<Sqlite>>,
    pub(super) cancelled: Arc<AtomicBool>,
    pub(super) finished: bool,
}

impl Timed {
    pub(super) fn new(connection: sqlx::pool::PoolConnection<Sqlite>) -> Self {
        Self {
            connection: Some(connection),
            cancelled: Arc::default(),
            finished: false,
        }
    }

    /// The connection, which is there until the guard is dropped.
    pub(super) fn connection(
        &mut self,
    ) -> Result<&mut sqlx::pool::PoolConnection<Sqlite>, StoreError> {
        self.connection
            .as_mut()
            .ok_or(StoreError::Corrupt("a connection"))
    }
}

impl Drop for Timed {
    fn drop(&mut self) {
        if !self.finished {
            self.cancelled.store(true, Ordering::Relaxed);
            if let Some(connection) = self.connection.as_mut() {
                connection.close_on_drop();
            }
        }
    }
}
