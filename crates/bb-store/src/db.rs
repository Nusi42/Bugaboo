//! Pools and transactions for both databases, and the one place where the output of the query
//! builder meets sqlx (ADR-0003).
//!
//! Every query is written once with sea-query and built for the dialect of the pool it runs on.
//! The text the builder returns holds placeholders only; all values travel as bound arguments.
//! This is the only file that may wrap query text in `AssertSqlSafe`, and the only one that
//! names the sqlx query functions (ARCH-SQL-EDGE).

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use bb_core::event::DomainEvent;
use futures_util::{Stream, TryStreamExt as _};
use sea_query::{
    Func, PostgresQueryBuilder, SchemaStatementBuilder, SelectStatement, SqliteQueryBuilder,
};
use sqlx::postgres::PgConnection;
use sqlx::{AssertSqlSafe, PgPool, Postgres, Sqlite, SqlitePool};

use crate::error::StoreError;
use crate::locks::{Held, LockClass, Mode};
use crate::security_log::NewSecurityLogEntry;

mod fixed;
mod gate;
mod open;
mod peer;
mod reader;
mod server_hold;
mod sqlite_open;
mod statement;
mod statements;
mod statistics;
mod timed;
mod upkeep;

pub(crate) use fixed::FixedSql;
pub(crate) use statement::Statement;
use statements::Statements;
#[cfg(any(test, feature = "testing"))]
mod hooks;
use gate::{Turn, WriterGate};
#[cfg(any(test, feature = "testing"))]
pub use hooks::UpkeepForTests;
pub use peer::{certificate_unchecked, with_checked_certificate};
pub(crate) use reader::{Reader, count_of};
pub use server_hold::ServerHold;
use timed::Timed;
use upkeep::Upkeep;

/// How many values one statement binds at most. SQLite allows 32,766 and PostgreSQL 65,535, so the
/// smaller counts, less a little room for the values a statement binds besides its list. This is
/// the one limit: what is asked about or written in bulk is cut into chunks that fit it, here in
/// the store, and no caller chunks for it.
pub(crate) const BIND_LIMIT: usize = 32_000;

/// How many things one statement names when each is one bound value (people, items).
pub(crate) const STATEMENT_ITEMS: usize = BIND_LIMIT;

/// The IDs in order and once each, cut into chunks that one statement can bind
/// ([`STATEMENT_ITEMS`]); asking about the chunks one after the other, each ordered by issue,
/// gives the same order as one statement would.
pub(crate) fn sorted_chunks<T: StoredId>(ids: &[T]) -> Vec<Vec<Vec<u8>>> {
    let mut stored: Vec<Vec<u8>> = ids.iter().map(|id| id.stored()).collect();
    stored.sort();
    stored.dedup();
    stored.chunks(STATEMENT_ITEMS).map(<[_]>::to_vec).collect()
}

/// How many rows one insert writes at most when each row binds `columns` values.
pub(crate) const fn rows_per_insert(columns: usize) -> usize {
    BIND_LIMIT / columns
}

/// The error code PostgreSQL gives a statement that `statement_timeout` stopped.
const QUERY_CANCELED: &str = "57014";

/// The code SQLite gives a statement that the progress handler interrupted.
const SQLITE_INTERRUPT: &str = "9";

/// How many steps of its virtual machine SQLite takes between two looks at the clock.
const PROGRESS_STEPS: i32 = 1_000;

/// The two databases Bugaboo supports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dialect {
    /// SQLite, the default.
    Sqlite,
    /// PostgreSQL 16 or newer.
    Postgres,
}

#[derive(Clone, Debug)]
enum Pool {
    Sqlite(SqlitePool),
    Postgres(PgPool),
}

/// A connection pool to the database. Cheap to clone; all clones share the pool.
#[derive(Clone, Debug)]
pub struct Database {
    pool: Pool,
    /// How many statements this database has run (all clones share it): the tests of PERF-05 read
    /// it before and after a request to see that a read runs as many statements for a busy
    /// issue as for a bare one.
    statements: Arc<Statements>,
    /// Whether the upkeep holds the database (all clones share it): a write that meets SQLite's
    /// busy answer meanwhile is tried again (`db/upkeep.rs`).
    upkeep: Arc<Upkeep>,
    /// The queue of the writers on SQLite (`db/gate.rs`).
    writers: WriterGate,
}

impl Database {
    /// Copies the SQLite database into the new file `file` with `VACUUM INTO`, which reads one
    /// consistent state while writers go on, and then folds the write-ahead log, which grew
    /// meanwhile, back into the database (ADR-0046, decision 2). `false` on PostgreSQL, whose
    /// copy is made table by table.
    pub(crate) async fn vacuum_into(&self, file: &std::path::Path) -> Result<bool, StoreError> {
        let Pool::Sqlite(pool) = &self.pool else {
            return Ok(false);
        };
        let target = file
            .to_str()
            .ok_or(StoreError::Corrupt("the path of a copy"))?;
        sqlx::query("VACUUM INTO ?")
            .bind(target)
            .execute(pool)
            .await?;
        sqlx::query("PRAGMA wal_checkpoint(PASSIVE)")
            .execute(pool)
            .await?;
        Ok(true)
    }

    /// Folds the write-ahead log of SQLite back into the database and empties it
    /// (`wal_checkpoint(TRUNCATE)`), so no old page of a deleted row stays in it; nothing on
    /// PostgreSQL. A reader that holds an old page keeps the log from being emptied this time: that
    /// is said in the log of the server, and the next checkpoint writes over it.
    pub(crate) async fn empty_write_ahead_log(&self) -> Result<(), StoreError> {
        let Pool::Sqlite(pool) = &self.pool else {
            return Ok(());
        };
        let (busy, _, _): (i64, i64, i64) = sqlx::query_as("PRAGMA wal_checkpoint(TRUNCATE)")
            .fetch_one(pool)
            .await?;
        if busy != 0 {
            tracing::warn!("the write-ahead log could not be emptied while a reader held it");
        }
        Ok(())
    }

    /// The file of a SQLite database as it was opened, whether from an address or a path;
    /// `None` on PostgreSQL. The address is read once, by the database library, so nothing of
    /// its query (`?mode=…`) is ever taken for a part of the file's name.
    #[must_use]
    pub fn file(&self) -> Option<std::path::PathBuf> {
        match &self.pool {
            Pool::Sqlite(pool) => Some(pool.connect_options().get_filename().to_path_buf()),
            Pool::Postgres(_) => None,
        }
    }

    /// Which database this is.
    #[must_use]
    pub const fn dialect(&self) -> Dialect {
        match self.pool {
            Pool::Sqlite(_) => Dialect::Sqlite,
            Pool::Postgres(_) => Dialect::Postgres,
        }
    }

    /// Closes all connections.
    pub async fn close(&self) {
        match &self.pool {
            Pool::Sqlite(pool) => pool.close().await,
            Pool::Postgres(pool) => pool.close().await,
        }
    }

    /// Makes the statistics SQLite chooses its indexes by again from every row, for each table
    /// SQLite reports out of date, one table at a time (`db/statistics.rs`). Returns whether it
    /// analysed a table; `false` on PostgreSQL, which keeps its own.
    ///
    /// # Errors
    /// Fails when the database cannot be read or the statistics cannot be written.
    pub async fn refresh_statistics(&self) -> Result<bool, StoreError> {
        let Pool::Sqlite(pool) = &self.pool else {
            return Ok(false);
        };
        // A write that meets the lock of an analysis is tried again while this lives.
        let _running = self.upkeep.mark();
        let mut connection = pool.acquire().await?;
        sqlx::query(statistics::RELOAD)
            .execute(&mut *connection)
            .await?;
        let reported = sqlx::query_scalar(statistics::OUT_OF_DATE)
            .fetch_all(&mut *connection)
            .await?;
        let mut analysed = false;
        for (statement, table) in statistics::tables_to_analyse(reported) {
            // SQLite's own text for one table, alone: the write lock is held for that table only.
            sqlx::query(AssertSqlSafe(statement))
                .execute(&mut *connection)
                .await?;
            sqlx::query(statistics::EMPTY_IS_UP_TO_DATE)
                .bind(&table)
                .bind(&table)
                .execute(&mut *connection)
                .await?;
            analysed = true;
        }
        Ok(analysed)
    }

    /// Asks the database a trivial question; for the *ready* health check.
    ///
    /// # Errors
    /// Fails when the database does not answer.
    pub async fn ping(&self) -> Result<(), StoreError> {
        let mut select = SelectStatement::new();
        select.expr(sea_query::Expr::val(1_i64));
        self.fetch_all(&select).await.map(|_| ())
    }

    /// Starts a transaction. On SQLite it takes the write lock at once (`BEGIN IMMEDIATE`), so
    /// writers wait for each other instead of failing half way: the single writer of
    /// architecture §7 (Performance).
    pub(crate) async fn begin(&self) -> Result<Transaction, StoreError> {
        // On SQLite the writers of this program queue first (`db/gate.rs`).
        let turn = self.writers.wait_on(self.dialect()).await;
        self.statements.record_transaction();
        Ok(Transaction {
            follow_ups: Vec::new(),
            security_log: Vec::new(),
            locks: Held::default(),
            statements: Arc::clone(&self.statements),
            inner: match &self.pool {
                // Nothing is written before the lock is taken, so a try that met the busy answer
                // can be made again whole.
                Pool::Sqlite(pool) => TxInner::Sqlite(
                    self.upkeep
                        .write(|| async { Ok(pool.begin_with("BEGIN IMMEDIATE").await?) })
                        .await?,
                ),
                Pool::Postgres(pool) => TxInner::Postgres(pool.begin().await?),
            },
            _turn: turn,
        })
    }

    /// Starts a read-only transaction that sees one consistent snapshot. It takes no write lock
    /// on SQLite (readers and the writer do not wait for each other in WAL mode) and uses
    /// repeatable read on PostgreSQL.
    pub(crate) async fn begin_read(&self) -> Result<Transaction, StoreError> {
        self.statements.record_transaction();
        Ok(Transaction {
            follow_ups: Vec::new(),
            security_log: Vec::new(),
            locks: Held::default(),
            statements: Arc::clone(&self.statements),
            _turn: None,
            inner: match &self.pool {
                Pool::Sqlite(pool) => TxInner::Sqlite(pool.begin().await?),
                Pool::Postgres(pool) => TxInner::Postgres(
                    pool.begin_with("BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY")
                        .await?,
                ),
            },
        })
    }

    /// [`Self::begin_read`] for reads that many small looks into indexes answer (the arms of the
    /// walk of a person's data): PostgreSQL starts no parallel workers in it, whose start takes
    /// longer than such a read (about 25 ms against 2 ms for the events of a person on the dataset
    /// of 50,000 issues). SQLite has none.
    pub(crate) async fn begin_read_serial(&self) -> Result<Transaction, StoreError> {
        let mut transaction = self.begin_read().await?;
        if let TxInner::Postgres(inner) = &mut transaction.inner {
            sqlx::query("SELECT set_config('max_parallel_workers_per_gather', '0', true)")
                .execute(&mut **inner)
                .await?;
        }
        Ok(transaction)
    }

    /// Runs a statement that changes nothing else but one row or a few, outside a transaction.
    pub(crate) async fn execute<'a>(
        &self,
        statement: impl Into<Statement<'a>>,
    ) -> Result<u64, StoreError> {
        let statement = statement.into();
        self.statements.record(&statement, self.dialect());
        // One statement on its own is its own transaction: a try that met the busy answer
        // changed nothing.
        let _turn = self.writers.wait_on(self.dialect()).await;
        self.upkeep
            .write(|| Connection::Pool(&self.pool).execute(statement))
            .await
    }

    /// Reads all rows of a query.
    pub(crate) async fn fetch_all<'a>(
        &self,
        statement: impl Into<Statement<'a>>,
    ) -> Result<Vec<Row>, StoreError> {
        let statement = statement.into();
        self.statements.record(&statement, self.dialect());
        Connection::Pool(&self.pool).fetch_all(statement).await
    }

    /// Reads all rows of a query that may take `limit` at most (SEC-17): PostgreSQL stops it with
    /// `statement_timeout` for the transaction it runs in, SQLite with a progress handler on the
    /// connection that runs it. A read that is stopped is [`StoreError::TimedOut`].
    pub(crate) async fn fetch_all_within<'a>(
        &self,
        statement: impl Into<Statement<'a>>,
        limit: Duration,
    ) -> Result<Vec<Row>, StoreError> {
        let statement = statement.into();
        self.statements.record(&statement, self.dialect());
        let stopped = |error: sqlx::Error| match &error {
            sqlx::Error::Database(found)
                if matches!(
                    found.code().as_deref(),
                    Some(QUERY_CANCELED | SQLITE_INTERRUPT)
                ) =>
            {
                StoreError::TimedOut
            }
            _ => StoreError::from(error),
        };
        match &self.pool {
            Pool::Sqlite(pool) => {
                let (sql, values) = statement.build(Dialect::Sqlite);
                let mut timed = Timed::new(pool.acquire().await?);
                let deadline = Instant::now() + limit;
                let cancelled = Arc::clone(&timed.cancelled);
                timed
                    .connection()?
                    .lock_handle()
                    .await?
                    .set_progress_handler(PROGRESS_STEPS, move || {
                        !cancelled.load(Ordering::Relaxed) && Instant::now() < deadline
                    });
                let rows = sqlx::query_with(AssertSqlSafe(sql), values)
                    .fetch_all(&mut **timed.connection()?)
                    .await;
                timed
                    .connection()?
                    .lock_handle()
                    .await?
                    .remove_progress_handler();
                timed.finished = true;
                Ok(rows
                    .map_err(stopped)?
                    .into_iter()
                    .map(Row::Sqlite)
                    .collect())
            }
            Pool::Postgres(pool) => {
                let (sql, values) = statement.build(Dialect::Postgres);
                let mut transaction = pool.begin().await?;
                sqlx::query("SELECT set_config('statement_timeout', $1, true)")
                    .bind(limit.as_millis().to_string())
                    .execute(&mut *transaction)
                    .await?;
                let rows = sqlx::query_with(AssertSqlSafe(sql), values)
                    .fetch_all(&mut *transaction)
                    .await
                    .map_err(stopped)?;
                transaction.commit().await?;
                Ok(rows.into_iter().map(Row::Postgres).collect())
            }
        }
    }

    /// Reads the first row of a query, if there is one.
    pub(crate) async fn fetch_optional<'a>(
        &self,
        statement: impl Into<Statement<'a>>,
    ) -> Result<Option<Row>, StoreError> {
        Ok(self.fetch_all(statement).await?.into_iter().next())
    }
}

/// Where a statement runs: on the pool, or inside a transaction. The two ways to run a
/// statement are written once, here, for both databases.
enum Connection<'c> {
    Pool(&'c Pool),
    Transaction(&'c mut TxInner),
    /// A PostgreSQL connection of its own, outside the pool (the hold of a server).
    Session(&'c mut PgConnection),
}

impl Connection<'_> {
    /// Runs a statement and tells how many rows it changed.
    async fn execute(self, statement: Statement<'_>) -> Result<u64, StoreError> {
        macro_rules! run {
            ($dialect:expr, $executor:expr) => {{
                let (sql, values) = statement.build($dialect);
                Ok(sqlx::query_with(AssertSqlSafe(sql), values)
                    .execute($executor)
                    .await?
                    .rows_affected())
            }};
        }
        match self {
            Self::Pool(Pool::Sqlite(pool)) => run!(Dialect::Sqlite, pool),
            Self::Pool(Pool::Postgres(pool)) => run!(Dialect::Postgres, pool),
            Self::Transaction(TxInner::Sqlite(transaction)) => {
                run!(Dialect::Sqlite, &mut **transaction)
            }
            Self::Transaction(TxInner::Postgres(transaction)) => {
                run!(Dialect::Postgres, &mut **transaction)
            }
            Self::Session(connection) => run!(Dialect::Postgres, connection),
        }
    }

    /// Runs a statement and reads all rows it answers (also a statement with `RETURNING`).
    async fn fetch_all(self, statement: Statement<'_>) -> Result<Vec<Row>, StoreError> {
        macro_rules! run {
            ($dialect:expr, $executor:expr, $row:path) => {{
                let (sql, values) = statement.build($dialect);
                let rows = sqlx::query_with(AssertSqlSafe(sql), values)
                    .fetch_all($executor)
                    .await?;
                Ok(rows.into_iter().map($row).collect())
            }};
        }
        match self {
            Self::Pool(Pool::Sqlite(pool)) => run!(Dialect::Sqlite, pool, Row::Sqlite),
            Self::Pool(Pool::Postgres(pool)) => run!(Dialect::Postgres, pool, Row::Postgres),
            Self::Transaction(TxInner::Sqlite(transaction)) => {
                run!(Dialect::Sqlite, &mut **transaction, Row::Sqlite)
            }
            Self::Transaction(TxInner::Postgres(transaction)) => {
                run!(Dialect::Postgres, &mut **transaction, Row::Postgres)
            }
            Self::Session(connection) => run!(Dialect::Postgres, connection, Row::Postgres),
        }
    }
}

enum TxInner {
    Sqlite(sqlx::Transaction<'static, Sqlite>),
    Postgres(sqlx::Transaction<'static, Postgres>),
}

/// A database transaction. Domain data is written only inside one, and only through
/// [`Database::record_change`](crate::events), so every change comes with its event and its
/// outbox entry. A transaction that is dropped without `commit` is rolled back.
pub struct Transaction {
    inner: TxInner,
    /// The counter of the statements of the database this transaction belongs to.
    statements: Arc<Statements>,
    /// Events the change adds besides its own; `record_change` appends them after the event of
    /// the change, so the lock on the outbox stays the last lock of the transaction.
    pub(crate) follow_ups: Vec<DomainEvent>,
    /// Entries of the security log the change adds: the sign-in storage appends them at the end of
    /// the change, so they are committed with it or not at all.
    pub(crate) security_log: Vec<NewSecurityLogEntry>,
    /// The advisory locks it took (also on SQLite, which needs none), to refuse a wrong order.
    locks: Held,
    /// The turn in the queue of the writers on SQLite. Last, so it ends after the transaction has
    /// been committed or rolled back and the next writer finds the lock free.
    _turn: Option<Turn>,
}

impl std::fmt::Debug for Transaction {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Transaction")
            .finish_non_exhaustive()
    }
}

impl Transaction {
    /// Which database this transaction runs on.
    pub(crate) const fn dialect(&self) -> Dialect {
        match self.inner {
            TxInner::Sqlite(_) => Dialect::Sqlite,
            TxInner::Postgres(_) => Dialect::Postgres,
        }
    }

    pub(crate) async fn commit(self) -> Result<(), StoreError> {
        match self.inner {
            TxInner::Sqlite(transaction) => transaction.commit().await?,
            TxInner::Postgres(transaction) => transaction.commit().await?,
        }
        Ok(())
    }

    /// Runs a statement that writes, and returns how many rows it changed.
    pub(crate) async fn execute<'a>(
        &mut self,
        statement: impl Into<Statement<'a>>,
    ) -> Result<u64, StoreError> {
        let statement = statement.into();
        self.statements.record(&statement, self.dialect());
        Connection::Transaction(&mut self.inner)
            .execute(statement)
            .await
    }

    /// Reads all rows of a query inside the transaction (also statements with `RETURNING`).
    pub(crate) async fn fetch_all<'a>(
        &mut self,
        statement: impl Into<Statement<'a>>,
    ) -> Result<Vec<Row>, StoreError> {
        let statement = statement.into();
        self.statements.record(&statement, self.dialect());
        Connection::Transaction(&mut self.inner)
            .fetch_all(statement)
            .await
    }

    /// Reads the first row of a query inside the transaction, if there is one.
    pub(crate) async fn fetch_optional<'a>(
        &mut self,
        statement: impl Into<Statement<'a>>,
    ) -> Result<Option<Row>, StoreError> {
        Ok(self.fetch_all(statement).await?.into_iter().next())
    }

    /// The rows of a query of PostgreSQL as the database sends them, inside the transaction, so a
    /// table of any size is read in one statement and never held in memory whole (the copy of a
    /// backup, ADR-0046). The database waits while the rows are not taken.
    pub(crate) fn stream_rows(
        &mut self,
        statement: &SelectStatement,
    ) -> Result<impl Stream<Item = Result<Row, StoreError>> + Send + '_, StoreError> {
        let statement = Statement::from(statement);
        self.statements.record(&statement, self.dialect());
        let TxInner::Postgres(transaction) = &mut self.inner else {
            return Err(StoreError::UnsupportedDatabase);
        };
        let (sql, values) = statement.build(Dialect::Postgres);
        Ok(sqlx::query_with(AssertSqlSafe(sql), values)
            .fetch(&mut **transaction)
            .map_ok(Row::Postgres)
            .map_err(StoreError::from))
    }

    /// Lifts the limits of time that the role or the database of PostgreSQL may set
    /// (`statement_timeout`, `idle_in_transaction_session_timeout`) for this transaction alone:
    /// the copy of a backup reads each table in one statement, which takes as long as the table
    /// is large, and waits for the disk between pieces (ADR-0046). SQLite sets none.
    pub(crate) async fn without_time_limits(&mut self) -> Result<(), StoreError> {
        if let TxInner::Postgres(transaction) = &mut self.inner {
            sqlx::query(
                "SELECT set_config('statement_timeout', '0', true),                  set_config('idle_in_transaction_session_timeout', '0', true)",
            )
            .execute(&mut **transaction)
            .await?;
        }
        Ok(())
    }

    /// Takes a lock that lasts until the end of the transaction, so processes that do the same
    /// thing (migrate) run one after the other. Dialect-specific on purpose (ADR-0003): on
    /// SQLite the transaction already holds the write lock (`BEGIN IMMEDIATE`), so nothing is
    /// needed; PostgreSQL has many writers and needs an advisory lock.
    pub(crate) async fn advisory_lock(
        &mut self,
        class: LockClass,
        key: i32,
    ) -> Result<(), StoreError> {
        self.locks.take(class, Mode::Alone)?;
        if matches!(self.inner, TxInner::Postgres(_)) {
            let mut select = SelectStatement::new();
            select.expr(
                Func::cust(sea_query::Alias::new("pg_advisory_xact_lock"))
                    .arg(class as i32)
                    .arg(key),
            );
            self.fetch_all(&select).await?;
        }
        Ok(())
    }

    /// Like [`Self::advisory_lock`], but shared: any number of transactions hold it together, and
    /// one that takes it alone waits for them all.
    pub(crate) async fn advisory_lock_shared(
        &mut self,
        class: LockClass,
        key: i32,
    ) -> Result<(), StoreError> {
        self.locks.take(class, Mode::Shared)?;
        if matches!(self.inner, TxInner::Postgres(_)) {
            let mut select = SelectStatement::new();
            select.expr(
                Func::cust(sea_query::Alias::new("pg_advisory_xact_lock_shared"))
                    .arg(class as i32)
                    .arg(key),
            );
            self.fetch_all(&select).await?;
        }
        Ok(())
    }

    /// Runs a schema statement (create a table or an index) built for this database.
    pub(crate) async fn execute_schema(
        &mut self,
        statement: &impl SchemaStatementBuilder,
    ) -> Result<(), StoreError> {
        self.execute_text(|dialect| match dialect {
            Dialect::Sqlite => statement.build(SqliteQueryBuilder),
            Dialect::Postgres => statement.build(PostgresQueryBuilder),
        })
        .await
    }

    /// Runs the fixed statements of a migration step that belong to this database.
    pub(crate) async fn execute_fixed(&mut self, sql: &FixedSql) -> Result<(), StoreError> {
        let dialect = match self.inner {
            TxInner::Sqlite(_) => Dialect::Sqlite,
            TxInner::Postgres(_) => Dialect::Postgres,
        };
        for text in sql.of(dialect) {
            self.execute_text(|_| (*text).to_owned()).await?;
        }
        Ok(())
    }

    /// Runs a text of schema or migration, as the database of this transaction needs it. The one
    /// place that runs such a text.
    async fn execute_text(
        &mut self,
        text: impl FnOnce(Dialect) -> String,
    ) -> Result<(), StoreError> {
        match &mut self.inner {
            TxInner::Sqlite(tx) => {
                let sql = text(Dialect::Sqlite);
                sqlx::query(AssertSqlSafe(sql)).execute(&mut **tx).await?;
            }
            TxInner::Postgres(tx) => {
                let sql = text(Dialect::Postgres);
                sqlx::query(AssertSqlSafe(sql)).execute(&mut **tx).await?;
            }
        }
        Ok(())
    }
}

pub(crate) use crate::rows::{Row, StoredId, from_micros, to_micros};

#[cfg(test)]
impl Database {
    /// The journal mode and the foreign-key switch of a SQLite connection, for the tests.
    pub(crate) async fn sqlite_pragmas(&self) -> Result<(String, i64, i64), StoreError> {
        let Pool::Sqlite(pool) = &self.pool else {
            return Err(StoreError::UnsupportedDatabase);
        };
        let mode = sqlx::query_scalar::<_, String>("PRAGMA journal_mode")
            .fetch_one(pool)
            .await?;
        let keys = sqlx::query_scalar::<_, i64>("PRAGMA foreign_keys")
            .fetch_one(pool)
            .await?;
        let sync = sqlx::query_scalar::<_, i64>("PRAGMA synchronous")
            .fetch_one(pool)
            .await?;
        Ok((mode, keys, sync))
    }

    /// Runs fixed statements on one SQLite connection, for the tests of the statistics.
    pub(crate) async fn sqlite_run(&self, texts: &[&'static str]) -> Result<(), StoreError> {
        let Pool::Sqlite(pool) = &self.pool else {
            return Err(StoreError::UnsupportedDatabase);
        };
        let mut connection = pool.acquire().await?;
        for text in texts {
            sqlx::query(*text).execute(&mut *connection).await?;
        }
        Ok(())
    }

    /// The plan the database makes for `select`, a line for each step (test builds only): what a
    /// test reads to see that an index answers a query. On PostgreSQL the plan is made with
    /// `enable_seqscan` off unless `defaults`, so a table is read whole only when no index can
    /// answer; with `defaults` the planner chooses by its own settings and statistics.
    ///
    /// # Errors
    /// Fails when the database fails.
    #[cfg(test)]
    pub(crate) async fn plan_for_tests(
        &self,
        select: &SelectStatement,
        defaults: bool,
    ) -> Result<Vec<String>, StoreError> {
        use sea_query_sqlx::SqlxBinder as _;
        match &self.pool {
            Pool::Sqlite(pool) => {
                let (sql, values) = select.build_sqlx(SqliteQueryBuilder);
                let rows: Vec<(i64, i64, i64, String)> =
                    sqlx::query_as_with(AssertSqlSafe(format!("EXPLAIN QUERY PLAN {sql}")), values)
                        .fetch_all(pool)
                        .await?;
                Ok(rows.into_iter().map(|(_, _, _, detail)| detail).collect())
            }
            Pool::Postgres(pool) => {
                let (sql, values) = select.build_sqlx(PostgresQueryBuilder);
                let mut transaction = pool.begin().await?;
                if !defaults {
                    sqlx::query("SET LOCAL enable_seqscan = off")
                        .execute(&mut *transaction)
                        .await?;
                }
                let rows: Vec<(String,)> =
                    sqlx::query_as_with(AssertSqlSafe(format!("EXPLAIN {sql}")), values)
                        .fetch_all(&mut *transaction)
                        .await?;
                transaction.rollback().await?;
                Ok(rows.into_iter().map(|(line,)| line).collect())
            }
        }
    }

    /// The statistics SQLite keeps of an index, if any.
    pub(crate) async fn sqlite_statistics_of(
        &self,
        index: &str,
    ) -> Result<Option<String>, StoreError> {
        let Pool::Sqlite(pool) = &self.pool else {
            return Err(StoreError::UnsupportedDatabase);
        };
        Ok(
            sqlx::query_scalar("SELECT stat FROM sqlite_stat1 WHERE idx = ?")
                .bind(index)
                .fetch_optional(pool)
                .await?,
        )
    }
}

/// How the lock of a running server is asked for (`server_hold`). The SQL of every advisory lock
/// is written in this file (ARCH-LOCK-ORDER); this one is taken outside a transaction.
#[derive(Clone, Copy)]
enum Ask {
    /// By a server: shared, waiting for a restore to end.
    Shared,
    /// By a server that looks first, and by the watch of a hold that takes it again: shared, and
    /// only when no restore holds it.
    SharedIfFree,
    /// By a restore: alone, and only when nobody holds it.
    AloneIfFree,
}

impl Ask {
    const fn function(self) -> &'static str {
        match self {
            Self::Shared => "pg_advisory_lock_shared",
            Self::SharedIfFree => "pg_try_advisory_lock_shared",
            Self::AloneIfFree => "pg_try_advisory_lock",
        }
    }
}

#[cfg(test)]
mod tests;
