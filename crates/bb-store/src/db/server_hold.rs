//! The hold of a running server on its PostgreSQL database (ADR-0046, decision 4), which servers
//! on other computers may share: each holds `LockClass::ServerRunning` shared on a connection of
//! its own while it runs, and a restore takes it alone or refuses. No transaction takes it, so it
//! is outside the order of the locks. On SQLite the lock of the data folder (`server.lock`) does
//! this.
//!
//! The lock lives as long as its connection. The connection is asked every [`CHECK_EVERY`]
//! whether it still answers, and the database server is asked to send keep-alive packets on it,
//! so a link that broke without a word is noticed on both ends. A connection that broke is
//! replaced and the lock taken again, unless a restore holds it by then: then the server must
//! stop, and [`ServerHold::lost`] says so. A pooler in transaction mode between the server and
//! the database hands the session to other clients, and a session lock taken through it holds
//! nothing; the database address must lead to the database server itself (or a pooler in session
//! mode).

use std::time::Duration;

use sea_query::{Func, SelectStatement};
use sqlx::Connection as _;
use sqlx::postgres::{PgConnectOptions, PgConnection};
use tokio::sync::watch;
use tokio::task::JoinHandle;

use super::{Ask, Connection, Database, Pool};
use crate::error::StoreError;
use crate::locks::LockClass;

/// The key of the lock: one database, one lock.
const SERVER_KEY: i32 = 0;

/// The column the try of the lock answers in.
const SERVER_HELD: crate::schema::Computed = crate::schema::Computed::Total;

/// How often the connection that holds the lock of a server is asked whether it still answers.
const CHECK_EVERY: Duration = Duration::from_secs(30);

/// How long the question may take before the connection counts as broken: a link that is cut
/// without a word would otherwise keep it waiting for many minutes.
const ANSWER_WITHIN: Duration = Duration::from_secs(10);

/// What the database server is told about keep-alive packets on the connection of the hold, in
/// seconds and packets (`tcp_keepalives_*`): it sends one after a minute without traffic and lets
/// the connection, and with it the lock, go when three in a row stay unanswered.
const KEEPALIVES: [(&str, &str); 3] = [
    ("tcp_keepalives_idle", "60"),
    ("tcp_keepalives_interval", "10"),
    ("tcp_keepalives_count", "3"),
];

/// The hold of a server, or of a restore, on the database; it ends when it is dropped (the
/// connection closes, and PostgreSQL lets the lock go).
#[must_use = "the hold ends when it is dropped"]
pub struct ServerHold {
    held: Held,
}

/// What keeps the lock.
enum Held {
    /// SQLite, or nothing to hold.
    Nothing,
    /// A connection of its own: the hold of a restore, which lasts as long as the restore.
    Connection { _connection: PgConnection },
    /// The hold of a server: a task that keeps the connection and checks it, and the word it gives
    /// when the lock is lost for good.
    Watched {
        task: JoinHandle<()>,
        lost: watch::Receiver<bool>,
    },
}

impl Drop for ServerHold {
    fn drop(&mut self) {
        if let Held::Watched { task, .. } = &self.held {
            // The task owns the connection; ending it closes the connection and frees the lock.
            task.abort();
        }
    }
}

impl ServerHold {
    const fn nothing() -> Self {
        Self {
            held: Held::Nothing,
        }
    }

    /// Waits until the hold is lost for good: its connection broke and the lock could not be
    /// taken again, because a restore holds it. A hold that cannot be lost (SQLite, a restore)
    /// never answers.
    pub async fn lost(&mut self) {
        match &mut self.held {
            Held::Watched { lost, .. } => {
                // A task that ended without a word ended because the hold was lost too.
                lost.wait_for(|lost| *lost).await.ok();
            }
            Held::Nothing | Held::Connection { .. } => std::future::pending().await,
        }
    }
}

impl Database {
    /// The hold every running server keeps on its database, shared with the other servers. It
    /// waits while a restore holds the database, and says so in the log.
    ///
    /// # Errors
    /// Fails when the database cannot be reached.
    pub async fn hold_as_server(&self) -> Result<ServerHold, StoreError> {
        self.hold_as_server_checking(CHECK_EVERY).await
    }

    /// The same, with the connection checked every `every` (the tests).
    pub(crate) async fn hold_as_server_checking(
        &self,
        every: Duration,
    ) -> Result<ServerHold, StoreError> {
        let Pool::Postgres(pool) = &self.pool else {
            return Ok(ServerHold::nothing());
        };
        let options = pool.connect_options().as_ref().clone().options(KEEPALIVES);
        let connection = if let Some(connection) = take(&options, Ask::SharedIfFree).await? {
            connection
        } else {
            tracing::warn!(
                "a restore holds the database: the server waits for it to end before it starts"
            );
            let held = take(&options, Ask::Shared).await?;
            tracing::info!("the restore has ended: the server starts");
            held.ok_or(StoreError::Corrupt("the hold of the server"))?
        };
        let (said, lost) = watch::channel(false);
        let task = tokio::spawn(watch_hold(connection, options, every, said));
        Ok(ServerHold {
            held: Held::Watched { task, lost },
        })
    }

    /// The hold of a restore: the database alone, or `None` when a server holds it.
    ///
    /// # Errors
    /// Fails when the database cannot be reached.
    pub async fn hold_alone(&self) -> Result<Option<ServerHold>, StoreError> {
        let Pool::Postgres(pool) = &self.pool else {
            return Ok(Some(ServerHold::nothing()));
        };
        Ok(take(&pool.connect_options(), Ask::AloneIfFree)
            .await?
            .map(|connection| ServerHold {
                held: Held::Connection {
                    _connection: connection,
                },
            }))
    }
}

/// A connection of its own that holds the lock as `ask` says, or `None` when a try did not get
/// it.
async fn take(options: &PgConnectOptions, ask: Ask) -> Result<Option<PgConnection>, StoreError> {
    let mut connection = PgConnection::connect_with(options).await?;
    let mut select = SelectStatement::new();
    select.expr_as(
        Func::cust(sea_query::Alias::new(ask.function()))
            .arg(LockClass::ServerRunning as i32)
            .arg(SERVER_KEY),
        SERVER_HELD,
    );
    let rows = Connection::Session(&mut connection)
        .fetch_all((&select).into())
        .await?;
    // `pg_advisory_lock_shared` answers nothing (void) once it holds; a try answers whether.
    let held = match ask {
        Ask::Shared => true,
        Ask::SharedIfFree | Ask::AloneIfFree => rows
            .first()
            .map(|row| row.get::<bool>(SERVER_HELD))
            .transpose()?
            .unwrap_or(false),
    };
    Ok(held.then_some(connection))
}

/// Checks the connection of a server's hold every `every`, and replaces it when it broke. When
/// the lock cannot be taken again, it says so on `said` and ends.
async fn watch_hold(
    mut connection: PgConnection,
    options: PgConnectOptions,
    every: Duration,
    said: watch::Sender<bool>,
) {
    loop {
        tokio::time::sleep(every).await;
        let answered = tokio::time::timeout(ANSWER_WITHIN, connection.ping()).await;
        if matches!(answered, Ok(Ok(()))) {
            continue;
        }
        tracing::warn!(
            "the connection that holds the database for this server broke: the hold is taken again"
        );
        match take(&options, Ask::SharedIfFree).await {
            Ok(Some(again)) => connection = again,
            Ok(None) => {
                tracing::error!(
                    "the server lost its hold on the database, and a restore holds it now: the server stops"
                );
                said.send_replace(true);
                return;
            }
            Err(error) => {
                tracing::error!(
                    cause = %error,
                    "the server lost its hold on the database and cannot take it again: the server stops"
                );
                said.send_replace(true);
                return;
            }
        }
    }
}
