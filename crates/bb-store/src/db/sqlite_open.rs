//! Opening the SQLite file: WAL mode, foreign keys on, deleted rows overwritten, a busy timeout,
//! and a file that only its owner can read.

use std::time::Duration;

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

use super::upkeep::while_locked;
use crate::error::StoreError;

/// How long SQLite waits for another writer before it gives up.
pub(super) const SQLITE_BUSY_TIMEOUT: Duration = Duration::from_secs(10);
/// The most connections SQLite gets: many readers; writers queue up on the database lock.
const SQLITE_MAX_CONNECTIONS: u32 = 8;

/// How often opening a SQLite file is tried when another process holds it, and how long to wait
/// between tries. Switching a brand-new file to WAL mode needs a lock that the busy timeout does
/// not cover, so two programs that start at the same moment can fail with "database is locked";
/// the second one only has to wait a moment (M1-F24).
const SQLITE_OPEN_TRIES: u32 = 20;
const SQLITE_OPEN_WAIT: Duration = Duration::from_millis(100);

/// Opens a SQLite pool in WAL mode with foreign keys on and a busy timeout. A file that another
/// process is just setting up is tried again for a little while.
pub(super) async fn open_sqlite(
    options: SqliteConnectOptions,
    busy_timeout: Duration,
) -> Result<SqlitePool, StoreError> {
    while_locked(
        SQLITE_OPEN_TRIES,
        SQLITE_OPEN_WAIT,
        || true,
        || async { Ok(open_sqlite_once(options.clone(), busy_timeout).await?) },
    )
    .await
}

async fn open_sqlite_once(
    options: SqliteConnectOptions,
    busy_timeout: Duration,
) -> Result<SqlitePool, sqlx::Error> {
    let options = options
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        // A connection that closes does not run `PRAGMA optimize`: its statistics come from a short
        // look at each index and misled SQLite on 1 M issues (`Database::refresh_statistics`).
        .optimize_on_close(false, None)
        // FULL: a committed change survives a power cut (security first). It costs one
        // more disk flush per commit than NORMAL does in WAL mode (M1-F23).
        .synchronous(SqliteSynchronous::Full)
        .foreign_keys(true)
        // A deleted row leaves zeros, not its text, in the pages it freed: the removal of a person
        // and every deleting job leave nothing readable in the file (M5-F52). On every connection,
        // because SQLite keeps the switch per connection and any of them may delete.
        .pragma("secure_delete", "on")
        .busy_timeout(busy_timeout);
    SqlitePoolOptions::new()
        .max_connections(SQLITE_MAX_CONNECTIONS)
        .connect_with(options)
        .await
}

/// Makes the database file before SQLite does, readable by its owner only: SQLite would make it
/// with the mode the umask leaves (0644), and the `-wal` and `-shm` files it keeps next to it copy
/// that mode (SEC-16, M1-F65). A file that exists keeps the rights it has. On Windows the file
/// inherits the access list of the data folder.
#[cfg(unix)]
pub(super) fn create_private_file(file: &std::path::Path) -> Result<(), StoreError> {
    use std::os::unix::fs::OpenOptionsExt as _;
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(file)
    {
        Err(error) if error.kind() != std::io::ErrorKind::AlreadyExists => {
            Err(StoreError::Database(sqlx::Error::Io(error)))
        }
        _ => Ok(()),
    }
}

/// Opens the copy of a database inside a backup to read it only: no WAL mode, no lock, nothing
/// written into the file.
pub(super) async fn open_sqlite_copy(file: &std::path::Path) -> Result<SqlitePool, StoreError> {
    let options = SqliteConnectOptions::new()
        .filename(file)
        .read_only(true)
        .immutable(true);
    Ok(SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?)
}
