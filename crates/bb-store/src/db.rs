//! Pools and transactions for both databases, and the one place where the output of the query
//! builder meets sqlx (ADR-0003).
//!
//! Every query is written once with sea-query and built for the dialect of the pool it runs on.
//! The text the builder returns holds placeholders only; all values travel as bound arguments.
//! This is the only file that may wrap query text in `AssertSqlSafe`, and the only one that
//! names the sqlx query functions (ARCH-SQL-EDGE).

use std::future::Future;
use std::str::FromStr as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use bb_core::event::DomainEvent;
use bb_core::id::{
    AccessKeyId, AssignmentId, BulkRunId, ChoiceId, CommentId, EditorId, EntityId, EventId,
    FieldId, GroupId, IdError, InvitationId, IssueId, IssueTypeId, JobId, LabelGroupId, LabelId,
    LayoutId, LockToken, PasskeyId, ProjectId, RelationId, RelationKindId, RoleId, SessionId,
    TagId, UserId,
};
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

/// How many values one statement binds at most. SQLite allows 32,766 and PostgreSQL 65,535, so the
/// smaller counts, less a little room for the values a statement binds besides its list. This is
/// the one limit: what is asked about or written in bulk is cut into chunks that fit it, here in
/// the store, and no caller chunks for it.
pub(crate) const BIND_LIMIT: usize = 32_000;

/// How many things one statement names when each is one bound value (people, items).
pub(crate) const STATEMENT_ITEMS: usize = BIND_LIMIT;

/// How many rows one insert writes at most when each row binds `columns` values.
pub(crate) const fn rows_per_insert(columns: usize) -> usize {
    BIND_LIMIT / columns
}

/// How long SQLite waits for another writer before it gives up.
const SQLITE_BUSY_TIMEOUT: Duration = Duration::from_secs(10);
/// The most connections SQLite gets: many readers; writers queue up on the database lock.
const SQLITE_MAX_CONNECTIONS: u32 = 8;
/// The most connections PostgreSQL gets when the number of processors is unknown.
const POSTGRES_FALLBACK_CONNECTIONS: u32 = 8;

/// How often opening a SQLite file is tried when another process holds it, and how long to wait
/// between tries. Switching a brand-new file to WAL mode needs a lock that the busy timeout does
/// not cover, so two programs that start at the same moment can fail with "database is locked";
/// the second one only has to wait a moment (M1-F24).
const SQLITE_OPEN_TRIES: u32 = 20;
const SQLITE_OPEN_WAIT: Duration = Duration::from_millis(100);

/// Whether the database refused because another connection holds the file (`SQLITE_BUSY` is 5,
/// `SQLITE_LOCKED` is 6).
fn is_locked(error: &sqlx::Error) -> bool {
    matches!(
        error,
        sqlx::Error::Database(database)
            if matches!(database.code().as_deref(), Some("5" | "6"))
    )
}

/// Opens a SQLite pool in WAL mode with foreign keys on and a busy timeout. A file that another
/// process is just setting up is tried again for a little while.
async fn open_sqlite(options: SqliteConnectOptions) -> Result<SqlitePool, StoreError> {
    let mut tries = 1;
    loop {
        match open_sqlite_once(options.clone()).await {
            Err(error) if is_locked(&error) && tries < SQLITE_OPEN_TRIES => {
                tries += 1;
                tokio::time::sleep(SQLITE_OPEN_WAIT).await;
            }
            other => return Ok(other?),
        }
    }
}

async fn open_sqlite_once(options: SqliteConnectOptions) -> Result<SqlitePool, sqlx::Error> {
    let options = options
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        // FULL: a committed change survives a power cut (security first). It costs one
        // more disk flush per commit than NORMAL does in WAL mode (M1-F23).
        .synchronous(SqliteSynchronous::Full)
        .foreign_keys(true)
        .busy_timeout(SQLITE_BUSY_TIMEOUT);
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
fn create_private_file(file: &std::path::Path) -> Result<(), StoreError> {
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
    statements: Arc<AtomicU64>,
}

impl Database {
    /// Opens the database named by `url`: `sqlite:<file>` (made if missing) or
    /// `postgres://…`. SQLite runs in WAL mode with foreign keys on and a busy timeout.
    ///
    /// # Errors
    /// Fails for another kind of URL or when the database cannot be reached.
    pub async fn connect(url: &str) -> Result<Self, StoreError> {
        let pool = if url.starts_with("sqlite:") {
            Pool::Sqlite(open_sqlite(SqliteConnectOptions::from_str(url)?).await?)
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
            pool: Pool::Sqlite(open_sqlite(SqliteConnectOptions::new().filename(file)).await?),
            statements: Arc::default(),
        })
    }

    /// Which database this is.
    #[must_use]
    pub const fn dialect(&self) -> Dialect {
        match self.pool {
            Pool::Sqlite(_) => Dialect::Sqlite,
            Pool::Postgres(_) => Dialect::Postgres,
        }
    }

    /// How many statements this database has run since it was opened: a counter for the tests of
    /// PERF-05 (no N+1 queries). Statements that change the schema or take a lock are not counted.
    #[must_use]
    pub fn statements_run(&self) -> u64 {
        self.statements.load(Ordering::Relaxed)
    }

    /// Closes all connections.
    pub async fn close(&self) {
        match &self.pool {
            Pool::Sqlite(pool) => pool.close().await,
            Pool::Postgres(pool) => pool.close().await,
        }
    }

    /// Renames a table. For tests only: they break one table on purpose, to see how a use case
    /// behaves when the database fails half way, and put it right again afterwards. Nothing in
    /// the program calls it.
    ///
    /// # Errors
    /// Fails when the table does not exist or the name is taken.
    #[cfg(feature = "testing")]
    pub async fn rename_table_for_tests(&self, from: &str, to: &str) -> Result<(), StoreError> {
        let rename = sea_query::Table::rename()
            .table(sea_query::Alias::new(from), sea_query::Alias::new(to))
            .to_owned();
        let mut transaction = self.begin().await?;
        transaction.execute_schema(&rename).await?;
        transaction.commit().await
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
            follow_ups: Vec::new(),
            statements: Arc::clone(&self.statements),
            inner: match &self.pool {
                Pool::Sqlite(pool) => TxInner::Sqlite(pool.begin_with("BEGIN IMMEDIATE").await?),
                Pool::Postgres(pool) => TxInner::Postgres(pool.begin().await?),
            },
        })
    }

    /// Starts a read-only transaction that sees one consistent snapshot. It takes no write lock
    /// on SQLite (readers and the writer do not wait for each other in WAL mode) and uses
    /// repeatable read on PostgreSQL.
    pub(crate) async fn begin_read(&self) -> Result<Transaction, StoreError> {
        Ok(Transaction {
            follow_ups: Vec::new(),
            statements: Arc::clone(&self.statements),
            inner: match &self.pool {
                Pool::Sqlite(pool) => TxInner::Sqlite(pool.begin().await?),
                Pool::Postgres(pool) => TxInner::Postgres(
                    pool.begin_with("BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY")
                        .await?,
                ),
            },
        })
    }

    /// Runs a statement that changes nothing else but one row or a few, outside a transaction.
    pub(crate) async fn execute<'a>(
        &self,
        statement: impl Into<Statement<'a>>,
    ) -> Result<u64, StoreError> {
        self.statements.fetch_add(1, Ordering::Relaxed);
        Connection::Pool(&self.pool).execute(statement.into()).await
    }

    /// Reads all rows of a query.
    pub(crate) async fn fetch_all<'a>(
        &self,
        statement: impl Into<Statement<'a>>,
    ) -> Result<Vec<Row>, StoreError> {
        self.statements.fetch_add(1, Ordering::Relaxed);
        Connection::Pool(&self.pool)
            .fetch_all(statement.into())
            .await
    }

    /// Reads the first row of a query, if there is one.
    pub(crate) async fn fetch_optional<'a>(
        &self,
        statement: impl Into<Statement<'a>>,
    ) -> Result<Option<Row>, StoreError> {
        Ok(self.fetch_all(statement).await?.into_iter().next())
    }
}

/// Something that runs the statements of a read: the pool (one statement, no snapshot) or a
/// transaction (a snapshot, or a change that is open). A read is written once over this, so there
/// is no pool version and no transaction version of it.
pub(crate) trait Reader: Send {
    /// Reads all rows of a query.
    fn fetch_all(
        &mut self,
        select: &SelectStatement,
    ) -> impl Future<Output = Result<Vec<Row>, StoreError>> + Send;

    /// Reads the first row of a query, if there is one.
    fn fetch_optional(
        &mut self,
        select: &SelectStatement,
    ) -> impl Future<Output = Result<Option<Row>, StoreError>> + Send {
        async move { Ok(self.fetch_all(select).await?.into_iter().next()) }
    }
}

impl Reader for &Database {
    fn fetch_all(
        &mut self,
        select: &SelectStatement,
    ) -> impl Future<Output = Result<Vec<Row>, StoreError>> + Send {
        Database::fetch_all(self, select)
    }
}

impl Reader for Transaction {
    fn fetch_all(
        &mut self,
        select: &SelectStatement,
    ) -> impl Future<Output = Result<Vec<Row>, StoreError>> + Send {
        Self::fetch_all(self, select)
    }
}

/// Where a statement runs: on the pool, or inside a transaction. The two ways to run a
/// statement are written once, here, for both databases.
enum Connection<'c> {
    Pool(&'c Pool),
    Transaction(&'c mut TxInner),
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
        }
    }
}

enum TxInner {
    Sqlite(sqlx::Transaction<'static, Sqlite>),
    Postgres(sqlx::Transaction<'static, Postgres>),
}

/// The kinds of advisory lock of the program, the one home of their numbers: PostgreSQL takes
/// the two-key form `pg_advisory_xact_lock(class, key)`, so locks of different kinds never meet.
#[derive(Clone, Copy, Debug)]
#[repr(i32)]
pub(crate) enum LockClass {
    /// Processes that migrate one after the other (key 0).
    Migration = 1,
    /// Changes of roles and rows that could leave no administrator (key 0).
    Policy = 2,
    /// The codes of the issues of one project (key: four bytes of the project's ID).
    IssueCodes = 3,
    /// Changes of which issue sits under which (key 0): two changes at the same moment cannot
    /// make a circle between them.
    IssueParents = 4,
    /// Adding relations (key 0): two relations added at the same moment cannot both pass the
    /// limit of relations of an issue.
    IssueRelations = 5,
    /// Changes of the names of tags, labels and groups of labels (key 0): two changes at the same
    /// moment cannot both take a name, and a *Pick one* group is judged on what is stored.
    TagsAndLabels = 6,
}

/// A database transaction. Domain data is written only inside one, and only through
/// [`Database::record_change`](crate::events), so every change comes with its event and its
/// outbox entry. A transaction that is dropped without `commit` is rolled back.
pub struct Transaction {
    inner: TxInner,
    /// The counter of the statements of the database this transaction belongs to.
    statements: Arc<AtomicU64>,
    /// Events the change adds besides its own; `record_change` appends them after the event of
    /// the change, so the lock on the outbox stays the last lock of the transaction.
    pub(crate) follow_ups: Vec<DomainEvent>,
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
        self.statements.fetch_add(1, Ordering::Relaxed);
        Connection::Transaction(&mut self.inner)
            .execute(statement.into())
            .await
    }

    /// Reads all rows of a query inside the transaction (also statements with `RETURNING`).
    pub(crate) async fn fetch_all<'a>(
        &mut self,
        statement: impl Into<Statement<'a>>,
    ) -> Result<Vec<Row>, StoreError> {
        self.statements.fetch_add(1, Ordering::Relaxed);
        Connection::Transaction(&mut self.inner)
            .fetch_all(statement.into())
            .await
    }

    /// Reads the first row of a query inside the transaction, if there is one.
    pub(crate) async fn fetch_optional<'a>(
        &mut self,
        statement: impl Into<Statement<'a>>,
    ) -> Result<Option<Row>, StoreError> {
        Ok(self.fetch_all(statement).await?.into_iter().next())
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
        if matches!(self.inner, TxInner::Postgres(_)) {
            let mut select = SelectStatement::new();
            select.expr(
                sea_query::Func::cust(sea_query::Alias::new("pg_advisory_xact_lock"))
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
    pub(crate) fn id<T: StoredId>(&self, column: impl Iden + Copy) -> Result<T, StoreError> {
        let bytes: Vec<u8> = self.get(column)?;
        T::read(bytes)
    }

    /// An ID that may be missing.
    pub(crate) fn optional_id<T: StoredId>(
        &self,
        column: impl Iden + Copy,
    ) -> Result<Option<T>, StoreError> {
        self.get::<Option<Vec<u8>>>(column)?
            .map(T::read)
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

/// An ID as the database holds it: 16 bytes. Writing and reading go through here, so no module
/// has its own copy of the conversion or its own wording for "this row cannot be read".
pub(crate) trait StoredId: Copy {
    /// What the ID is, for the report of a stored value that is not one.
    const WHAT: &'static str;

    /// The ID with these bytes, if they are one.
    fn from_stored(bytes: [u8; 16]) -> Result<Self, IdError>;

    /// The 16 bytes of the ID.
    fn to_stored(self) -> [u8; 16];

    /// The bytes as a value to bind to a query.
    fn stored(self) -> Vec<u8> {
        self.to_stored().to_vec()
    }

    /// The ID stored in `bytes`, or the report that the row is corrupt.
    fn read(bytes: Vec<u8>) -> Result<Self, StoreError> {
        <[u8; 16]>::try_from(bytes)
            .ok()
            .and_then(|bytes| Self::from_stored(bytes).ok())
            .ok_or(StoreError::Corrupt(Self::WHAT))
    }
}

macro_rules! stored_ids {
    ($($ty:ident => $what:literal),+ $(,)?) => {
        $(impl StoredId for $ty {
            const WHAT: &'static str = $what;

            fn from_stored(bytes: [u8; 16]) -> Result<Self, IdError> {
                Self::from_bytes(bytes)
            }

            fn to_stored(self) -> [u8; 16] {
                self.to_bytes()
            }
        })+
    };
}

stored_ids!(
    AccessKeyId => "an access key ID",
    AssignmentId => "an assignment ID",
    BulkRunId => "a run ID",
    CommentId => "a comment ID",
    EditorId => "an editor ID",
    EntityId => "a record ID",
    EventId => "an event ID",
    FieldId => "a field ID",
    ChoiceId => "a choice ID",
    GroupId => "a group ID",
    InvitationId => "an invitation ID",
    IssueId => "an issue ID",
    IssueTypeId => "an issue type ID",
    JobId => "a job ID",
    LabelGroupId => "a label group ID",
    LabelId => "a label ID",
    LayoutId => "a layout ID",
    LockToken => "an edit lock token",
    PasskeyId => "a passkey ID",
    ProjectId => "a project ID",
    TagId => "a tag ID",
    RelationId => "a relation ID",
    RelationKindId => "a kind of relation ID",
    RoleId => "a role ID",
    SessionId => "a session ID",
    UserId => "a person ID",
);

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
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Many programs that start at the same moment on a brand-new SQLite file all get a working
    /// database; none of them fails with "database is locked" (M1-F24).
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn m1_f24_programs_that_start_together_on_a_new_file_all_get_the_database()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for round in 0..5 {
            let folder = tempfile::tempdir()?;
            let file = folder.path().join(format!("round-{round}.db"));
            let mut starts = Vec::new();
            for _ in 0..12 {
                let file = file.clone();
                starts.push(tokio::spawn(async move {
                    let database = Database::connect_file(&file).await?;
                    database.ping().await?;
                    Ok::<_, StoreError>(())
                }));
            }
            for start in starts {
                start.await??;
            }
        }
        Ok(())
    }

    #[test]
    fn m1_f24_only_a_busy_or_locked_database_is_tried_again() {
        assert!(!is_locked(&sqlx::Error::RowNotFound));
        assert!(!is_locked(&sqlx::Error::PoolTimedOut));
    }
    /// The built-in database and the files SQLite keeps next to it are for the owner only, also
    /// under the usual umask (SEC-16, M1-F65).
    #[cfg(unix)]
    #[tokio::test]
    async fn sec_16_the_database_file_and_its_companions_are_private()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use std::os::unix::fs::PermissionsExt as _;
        let folder = tempfile::tempdir()?;
        let file = folder.path().join("bugaboo.db");
        let database = Database::connect_file(&file).await?;
        database.ping().await?;
        database.migrate(Timestamp::now()).await?;
        for suffix in ["", "-wal", "-shm"] {
            let mut name = file.clone().into_os_string();
            name.push(suffix);
            let mode = std::fs::metadata(&name)?.permissions().mode() & 0o777;
            assert_eq!(mode & 0o077, 0, "{suffix}: {mode:o}");
        }
        // A file that exists keeps the rights it has.
        database.close().await;
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o640))?;
        Database::connect_file(&file).await?;
        assert_eq!(
            std::fs::metadata(&file)?.permissions().mode() & 0o777,
            0o640
        );
        Ok(())
    }
}
