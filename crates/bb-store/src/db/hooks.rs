//! The hooks of the tests into the database (test builds only): a busy timeout of their own,
//! SQLite's write lock held as the upkeep holds it, and a table taken away for a while.

use std::time::Duration;

use sqlx::Sqlite;

use super::upkeep::Running;
use super::{Database, Pool};
use crate::error::StoreError;

/// SQLite's write lock held as the upkeep holds it, until this is let go (test builds only; see
/// [`Database::hold_lock_as_upkeep_for_tests`]).
pub struct UpkeepForTests<'a> {
    _running: Running<'a>,
    _held: Option<sqlx::Transaction<'static, Sqlite>>,
}

impl Database {
    /// Opens the database named by `url` with a busy timeout shorter than the program's, so a test
    /// of a write that waits for the upkeep does not wait the whole timeout (test builds only).
    ///
    /// # Errors
    /// As [`Self::connect`].
    pub async fn connect_with_busy_timeout(
        url: &str,
        busy_timeout: Duration,
    ) -> Result<Self, StoreError> {
        Self::connect_waiting(url, busy_timeout).await
    }

    /// A SQLite database with a pool of one connection, so a test gets the same connection back
    /// every time it asks for one.
    #[cfg(test)]
    pub(crate) async fn single_connection(file: &std::path::Path) -> Result<Self, StoreError> {
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(file)
            .create_if_missing(true);
        Ok(Self {
            pool: Pool::Sqlite(
                sqlx::sqlite::SqlitePoolOptions::new()
                    .max_connections(1)
                    .connect_with(options)
                    .await?,
            ),
            statements: std::sync::Arc::default(),
            upkeep: std::sync::Arc::default(),
        })
    }

    /// Renames a table. For tests only: they break one table on purpose, to see how a use case
    /// behaves when the database fails half way, and put it right again afterwards. Nothing in
    /// the program calls it.
    ///
    /// # Errors
    /// Fails when the table does not exist or the name is taken.
    pub async fn rename_table_for_tests(&self, from: &str, to: &str) -> Result<(), StoreError> {
        let rename = sea_query::Table::rename()
            .table(sea_query::Alias::new(from), sea_query::Alias::new(to))
            .to_owned();
        let mut transaction = self.begin().await?;
        transaction.execute_schema(&rename).await?;
        transaction.commit().await
    }

    /// Holds SQLite's write lock as the upkeep does, until the returned value is let go (test
    /// builds only): a test of a write that meets the upkeep. On PostgreSQL, which has no lock of
    /// the whole database, it only marks the upkeep.
    ///
    /// # Errors
    /// Fails when the lock cannot be taken.
    pub async fn hold_lock_as_upkeep_for_tests(&self) -> Result<UpkeepForTests<'_>, StoreError> {
        let running = self.upkeep.mark();
        let held = match &self.pool {
            Pool::Sqlite(pool) => Some(pool.begin_with("BEGIN IMMEDIATE").await?),
            Pool::Postgres(_) => None,
        };
        Ok(UpkeepForTests {
            _running: running,
            _held: held,
        })
    }

    /// How many times a write met SQLite's busy answer while the upkeep held the lock and was
    /// tried again, since the database was opened (test builds only).
    #[must_use]
    pub fn writes_tried_again(&self) -> u64 {
        self.upkeep.tried_again()
    }
}
