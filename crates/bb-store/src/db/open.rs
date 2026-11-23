//! Opening the database: from a URL, the file of the data folder, or the copy a backup holds.

use std::str::FromStr as _;
use std::sync::Arc;
use std::time::Duration;

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::sqlite::SqliteConnectOptions;

#[cfg(unix)]
use super::sqlite_open::create_private_file;
use super::sqlite_open::{SQLITE_BUSY_TIMEOUT, open_sqlite, open_sqlite_copy};
use super::{Database, Pool, WriterGate};
use crate::error::StoreError;

/// The most connections PostgreSQL gets when the number of processors is unknown.
const POSTGRES_FALLBACK_CONNECTIONS: u32 = 8;

impl Database {
    /// Opens the database named by `url`: `sqlite:<file>` (made if missing) or
    /// `postgres://…`. SQLite runs in WAL mode with foreign keys on and a busy timeout.
    ///
    /// # Errors
    /// Fails for another kind of URL or when the database cannot be reached.
    pub async fn connect(url: &str) -> Result<Self, StoreError> {
        Self::connect_waiting(url, SQLITE_BUSY_TIMEOUT).await
    }

    pub(super) async fn connect_waiting(
        url: &str,
        busy_timeout: Duration,
    ) -> Result<Self, StoreError> {
        let pool = if url.starts_with("sqlite:") {
            Pool::Sqlite(open_sqlite(SqliteConnectOptions::from_str(url)?, busy_timeout).await?)
        } else if url.starts_with("postgres:") || url.starts_with("postgresql:") {
            let options = PgConnectOptions::from_str(url)?;
            let connections = std::thread::available_parallelism()
                .map_or(POSTGRES_FALLBACK_CONNECTIONS, |count| {
                    u32::try_from(count.get()).unwrap_or(POSTGRES_FALLBACK_CONNECTIONS)
                });
            Pool::Postgres(
                PgPoolOptions::new()
                    .max_connections(connections.max(2))
                    .connect_with(options)
                    .await?,
            )
        } else {
            return Err(StoreError::UnsupportedDatabase);
        };
        Ok(Self {
            pool,
            statements: Arc::default(),
            upkeep: Arc::default(),
            writers: WriterGate::default(),
        })
    }

    /// Opens the SQLite database in `file` (made if missing), the built-in database of the data
    /// folder. The path is used as it is: it is not read as a URL.
    ///
    /// # Errors
    /// Fails when the database cannot be opened.
    pub async fn connect_file(file: &std::path::Path) -> Result<Self, StoreError> {
        #[cfg(unix)]
        create_private_file(file)?;
        Ok(Self {
            pool: Pool::Sqlite(
                open_sqlite(
                    SqliteConnectOptions::new().filename(file),
                    SQLITE_BUSY_TIMEOUT,
                )
                .await?,
            ),
            statements: Arc::default(),
            upkeep: Arc::default(),
            writers: WriterGate::default(),
        })
    }

    /// Opens the copy of a SQLite database that a backup holds, only to read it: nothing is
    /// written into the file, not even the switch to WAL mode (ADR-0046).
    pub(crate) async fn open_copy(file: &std::path::Path) -> Result<Self, StoreError> {
        Ok(Self {
            pool: Pool::Sqlite(open_sqlite_copy(file).await?),
            statements: Arc::default(),
            upkeep: Arc::default(),
            writers: WriterGate::default(),
        })
    }
}
