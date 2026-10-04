//! Pools and transactions for both databases, and the one place where the output of the query
//! builder meets sqlx (ADR-0003).
//!
//! Every query is written once with sea-query and built for the dialect of the pool it runs on.
//! The text the builder returns holds placeholders only; all values travel as bound arguments.
//! This is the only file that may wrap query text in `AssertSqlSafe`, and the only one that
//! names the sqlx query functions (ARCH-SQL-EDGE).

use std::str::FromStr as _;
use std::time::Duration;

use jiff::Timestamp;
use sea_query::{
    DeleteStatement, Iden, InsertStatement, PostgresQueryBuilder, SchemaStatementBuilder,
    SelectStatement, SqliteQueryBuilder, UpdateStatement,
};
use sea_query_sqlx::{SqlxBinder as _, SqlxValues};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgRow};
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteRow, SqliteSynchronous,
};
use sqlx::{AssertSqlSafe, PgPool, Postgres, Row as _, Sqlite, SqlitePool};

use crate::error::StoreError;

/// How long SQLite waits for another writer before it gives up.
const SQLITE_BUSY_TIMEOUT: Duration = Duration::from_secs(10);
/// The most connections SQLite gets: many readers; writers queue up on the database lock.
const SQLITE_MAX_CONNECTIONS: u32 = 8;
/// The most connections PostgreSQL gets when the number of processors is unknown.
const POSTGRES_FALLBACK_CONNECTIONS: u32 = 8;

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
}

impl Database {
    /// Opens the database named by `url`: `sqlite:<file>` (made if missing) or
    /// `postgres://…`. SQLite runs in WAL mode with foreign keys on and a busy timeout.
    ///
    /// # Errors
    /// Fails for another kind of URL or when the database cannot be reached.
    pub async fn connect(url: &str) -> Result<Self, StoreError> {
        let pool = if url.starts_with("sqlite:") {
            let options = SqliteConnectOptions::from_str(url)?
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal)
                .synchronous(SqliteSynchronous::Normal)
                .foreign_keys(true)
                .busy_timeout(SQLITE_BUSY_TIMEOUT);
            Pool::Sqlite(
                SqlitePoolOptions::new()
                    .max_connections(SQLITE_MAX_CONNECTIONS)
                    .connect_with(options)
                    .await?,
            )
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
        Ok(Self { pool })
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
        Ok(Transaction {
            inner: match &self.pool {
                Pool::Sqlite(pool) => TxInner::Sqlite(pool.begin_with("BEGIN IMMEDIATE").await?),
                Pool::Postgres(pool) => TxInner::Postgres(pool.begin().await?),
            },
        })
    }

    /// Runs a statement that changes nothing else but one row or a few, outside a transaction.
    pub(crate) async fn execute<'a>(
        &self,
        statement: impl Into<Statement<'a>>,
    ) -> Result<u64, StoreError> {
        let statement = statement.into();
        match &self.pool {
            Pool::Sqlite(pool) => {
                let (sql, values) = statement.build(Dialect::Sqlite);
                Ok(sqlx::query_with(AssertSqlSafe(sql), values)
                    .execute(pool)
                    .await?
                    .rows_affected())
            }
            Pool::Postgres(pool) => {
                let (sql, values) = statement.build(Dialect::Postgres);
                Ok(sqlx::query_with(AssertSqlSafe(sql), values)
                    .execute(pool)
                    .await?
                    .rows_affected())
            }
        }
    }

    /// Reads all rows of a query.
    pub(crate) async fn fetch_all<'a>(
        &self,
        statement: impl Into<Statement<'a>>,
    ) -> Result<Vec<Row>, StoreError> {
        let statement = statement.into();
        match &self.pool {
            Pool::Sqlite(pool) => {
                let (sql, values) = statement.build(Dialect::Sqlite);
                let rows = sqlx::query_with(AssertSqlSafe(sql), values)
                    .fetch_all(pool)
                    .await?;
                Ok(rows.into_iter().map(Row::Sqlite).collect())
            }
            Pool::Postgres(pool) => {
                let (sql, values) = statement.build(Dialect::Postgres);
                let rows = sqlx::query_with(AssertSqlSafe(sql), values)
                    .fetch_all(pool)
                    .await?;
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

enum TxInner {
    Sqlite(sqlx::Transaction<'static, Sqlite>),
    Postgres(sqlx::Transaction<'static, Postgres>),
}

/// A database transaction. Domain data is written only inside one, and only through
/// [`Database::record_change`](crate::events), so every change comes with its event and its
/// outbox entry. A transaction that is dropped without `commit` is rolled back.
pub struct Transaction {
    inner: TxInner,
}

impl std::fmt::Debug for Transaction {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Transaction")
            .finish_non_exhaustive()
    }
}

impl Transaction {
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
        match &mut self.inner {
            TxInner::Sqlite(transaction) => {
                let (sql, values) = statement.build(Dialect::Sqlite);
                Ok(sqlx::query_with(AssertSqlSafe(sql), values)
                    .execute(&mut **transaction)
                    .await?
                    .rows_affected())
            }
            TxInner::Postgres(transaction) => {
                let (sql, values) = statement.build(Dialect::Postgres);
                Ok(sqlx::query_with(AssertSqlSafe(sql), values)
                    .execute(&mut **transaction)
                    .await?
                    .rows_affected())
            }
        }
    }

    /// Reads all rows of a query inside the transaction (also statements with `RETURNING`).
    pub(crate) async fn fetch_all<'a>(
        &mut self,
        statement: impl Into<Statement<'a>>,
    ) -> Result<Vec<Row>, StoreError> {
        let statement = statement.into();
        match &mut self.inner {
            TxInner::Sqlite(transaction) => {
                let (sql, values) = statement.build(Dialect::Sqlite);
                let rows = sqlx::query_with(AssertSqlSafe(sql), values)
                    .fetch_all(&mut **transaction)
                    .await?;
                Ok(rows.into_iter().map(Row::Sqlite).collect())
            }
            TxInner::Postgres(transaction) => {
                let (sql, values) = statement.build(Dialect::Postgres);
                let rows = sqlx::query_with(AssertSqlSafe(sql), values)
                    .fetch_all(&mut **transaction)
                    .await?;
                Ok(rows.into_iter().map(Row::Postgres).collect())
            }
        }
    }

    /// Reads the first row of a query inside the transaction, if there is one.
    pub(crate) async fn fetch_optional<'a>(
        &mut self,
        statement: impl Into<Statement<'a>>,
    ) -> Result<Option<Row>, StoreError> {
        Ok(self.fetch_all(statement).await?.into_iter().next())
    }

    /// Runs a schema statement (create a table or an index) built for this database.
    pub(crate) async fn execute_schema(
        &mut self,
        statement: &impl SchemaStatementBuilder,
    ) -> Result<(), StoreError> {
        match &mut self.inner {
            TxInner::Sqlite(transaction) => {
                let sql = statement.build(SqliteQueryBuilder);
                sqlx::query(AssertSqlSafe(sql))
                    .execute(&mut **transaction)
                    .await?;
            }
            TxInner::Postgres(transaction) => {
                let sql = statement.build(PostgresQueryBuilder);
                sqlx::query(AssertSqlSafe(sql))
                    .execute(&mut **transaction)
                    .await?;
            }
        }
        Ok(())
    }
}

/// A query built with sea-query, ready to be built for a dialect.
pub(crate) enum Statement<'a> {
    Select(&'a SelectStatement),
    Insert(&'a InsertStatement),
    Update(&'a UpdateStatement),
    Delete(&'a DeleteStatement),
}

impl Statement<'_> {
    fn build(&self, dialect: Dialect) -> (String, SqlxValues) {
        match (self, dialect) {
            (Self::Select(query), Dialect::Sqlite) => query.build_sqlx(SqliteQueryBuilder),
            (Self::Select(query), Dialect::Postgres) => query.build_sqlx(PostgresQueryBuilder),
            (Self::Insert(query), Dialect::Sqlite) => query.build_sqlx(SqliteQueryBuilder),
            (Self::Insert(query), Dialect::Postgres) => query.build_sqlx(PostgresQueryBuilder),
            (Self::Update(query), Dialect::Sqlite) => query.build_sqlx(SqliteQueryBuilder),
            (Self::Update(query), Dialect::Postgres) => query.build_sqlx(PostgresQueryBuilder),
            (Self::Delete(query), Dialect::Sqlite) => query.build_sqlx(SqliteQueryBuilder),
            (Self::Delete(query), Dialect::Postgres) => query.build_sqlx(PostgresQueryBuilder),
        }
    }
}

macro_rules! statement_from {
    ($($query:ident => $variant:ident),+) => {
        $(impl<'a> From<&'a $query> for Statement<'a> {
            fn from(query: &'a $query) -> Self {
                Self::$variant(query)
            }
        })+
    };
}

statement_from!(
    SelectStatement => Select,
    InsertStatement => Insert,
    UpdateStatement => Update,
    DeleteStatement => Delete
);

/// One row of an answer, from either database.
pub(crate) enum Row {
    Sqlite(SqliteRow),
    Postgres(PgRow),
}

impl Row {
    /// The value of a column. Whole numbers are always `i64` (`BIGINT`), so both databases give
    /// the same type.
    pub(crate) fn get<'r, T>(&'r self, column: impl Iden + Copy) -> Result<T, StoreError>
    where
        T: sqlx::Decode<'r, Sqlite>
            + sqlx::Type<Sqlite>
            + sqlx::Decode<'r, Postgres>
            + sqlx::Type<Postgres>,
    {
        let name = column.to_string();
        match self {
            Self::Sqlite(row) => row.try_get(name.as_str()),
            Self::Postgres(row) => row.try_get(name.as_str()),
        }
        .map_err(StoreError::from)
    }

    /// An ID stored as 16 bytes.
    pub(crate) fn id_bytes(&self, column: impl Iden + Copy) -> Result<[u8; 16], StoreError> {
        let bytes: Vec<u8> = self.get(column)?;
        <[u8; 16]>::try_from(bytes).map_err(|_| StoreError::Corrupt("an ID"))
    }

    /// An ID that may be missing.
    pub(crate) fn optional_id_bytes(
        &self,
        column: impl Iden + Copy,
    ) -> Result<Option<[u8; 16]>, StoreError> {
        self.get::<Option<Vec<u8>>>(column)?
            .map(|bytes| <[u8; 16]>::try_from(bytes).map_err(|_| StoreError::Corrupt("an ID")))
            .transpose()
    }

    /// A moment stored as microseconds since 1970.
    pub(crate) fn time(&self, column: impl Iden + Copy) -> Result<Timestamp, StoreError> {
        from_micros(self.get(column)?)
    }

    /// A moment that may be missing.
    pub(crate) fn optional_time(
        &self,
        column: impl Iden + Copy,
    ) -> Result<Option<Timestamp>, StoreError> {
        self.get::<Option<i64>>(column)?
            .map(from_micros)
            .transpose()
    }
}

/// A moment as it is stored: microseconds since 1970 (UTC), the same whole number in both databases.
pub(crate) fn to_micros(time: Timestamp) -> i64 {
    time.as_microsecond()
}

/// The moment stored as `micros`.
pub(crate) fn from_micros(micros: i64) -> Result<Timestamp, StoreError> {
    Timestamp::from_microsecond(micros).map_err(|_| StoreError::Corrupt("a time"))
}

#[cfg(test)]
impl Database {
    /// The journal mode and the foreign-key switch of a SQLite connection, for the tests.
    pub(crate) async fn sqlite_pragmas(&self) -> Result<(String, i64), StoreError> {
        let Pool::Sqlite(pool) = &self.pool else {
            return Err(StoreError::UnsupportedDatabase);
        };
        let mode = sqlx::query_scalar::<_, String>("PRAGMA journal_mode")
            .fetch_one(pool)
            .await?;
        let keys = sqlx::query_scalar::<_, i64>("PRAGMA foreign_keys")
            .fetch_one(pool)
            .await?;
        Ok((mode, keys))
    }
}
