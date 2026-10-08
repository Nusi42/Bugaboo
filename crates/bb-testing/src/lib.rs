//! Database test harness: runs one async test body against SQLite and PostgreSQL, each time with
//! a brand-new empty database (ADR-0003, architecture §11 "Database tests").
//!
//! ```no_run
//! #[tokio::test]
//! async fn adds_an_issue() -> bb_testing::TestResult {
//!     bb_testing::each_database("adds_an_issue", async |db| {
//!         // `db.pool()` is a typed pool (`Pool::Sqlite` or `Pool::Postgres`); see the
//!         // example tests in `tests/example_repository.rs`.
//!         let _ = db.kind();
//!         Ok(())
//!     })
//!     .await
//! }
//! ```
//!
//! Which databases run is chosen by `BB_TEST_DATABASES` (`sqlite`, `postgres`, or both separated
//! by a comma; default `sqlite`). `cargo xtask db run -- cargo test` and `cargo xtask ci` set
//! everything up, including `BB_TEST_POSTGRES_URL` (the throwaway cluster) and `BB_DB_PARITY_FILE`
//! (the record that ARCH-DB-PARITY reads). A PostgreSQL database is dropped after the test also
//! when the body panics.

#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt::Write as _;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub use bb_dev_words::database::DatabaseKind;
use bb_dev_words::database::{
    PARITY_RECORD_VARIABLE, TEST_DATABASES_VARIABLE, TEST_POSTGRES_URL_VARIABLE,
    parse as parse_database,
};
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{AssertSqlSafe, Connection as _, Executor as _, PgConnection, PgPool, SqlitePool};

/// The error of a test body or a test helper: any error, so `?` works on all of them.
pub type Failure = Box<dyn Error + Send + Sync>;

/// What a test body returns.
pub type TestResult = Result<(), Failure>;

/// The typed connection pool of a test database. Queries are written once (sea-query, ADR-0003)
/// and run on whichever pool the database has.
#[derive(Debug)]
pub enum Pool {
    /// A SQLite pool.
    Sqlite(SqlitePool),
    /// A PostgreSQL pool.
    Postgres(PgPool),
}

/// One empty database for one test run.
pub struct TestDatabase {
    kind: DatabaseKind,
    url: String,
    pool: Pool,
}

// Written by hand: the URL of a PostgreSQL database holds the password.
impl std::fmt::Debug for TestDatabase {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TestDatabase")
            .field("kind", &self.kind)
            .field("url", &"<hidden>")
            .finish_non_exhaustive()
    }
}

impl TestDatabase {
    /// Which database this is, for the rare dialect-specific assertion.
    #[must_use]
    pub const fn kind(&self) -> DatabaseKind {
        self.kind
    }

    /// The connection URL of this database (it contains the password for PostgreSQL), for code
    /// that builds its own pool.
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    /// The typed pool of this database.
    #[must_use]
    pub const fn pool(&self) -> &Pool {
        &self.pool
    }

    /// The SQLite pool, if this is the SQLite database.
    #[must_use]
    pub const fn sqlite_pool(&self) -> Option<&SqlitePool> {
        match &self.pool {
            Pool::Sqlite(pool) => Some(pool),
            Pool::Postgres(_) => None,
        }
    }

    /// The PostgreSQL pool, if this is the PostgreSQL database.
    #[must_use]
    pub const fn postgres_pool(&self) -> Option<&PgPool> {
        match &self.pool {
            Pool::Postgres(pool) => Some(pool),
            Pool::Sqlite(_) => None,
        }
    }
}

/// Runs `body` once for every database selected by `BB_TEST_DATABASES`, each with a new empty
/// database that is removed afterwards, also when the body panics. `test_name` is what the
/// parity record shows.
///
/// # Errors
///
/// Returns the first error of the setup, the body or the cleanup.
pub async fn each_database<F>(test_name: &str, body: F) -> TestResult
where
    F: AsyncFn(&TestDatabase) -> TestResult,
{
    let selected = selected(std::env::var(TEST_DATABASES_VARIABLE).ok().as_deref())?;
    for kind in selected {
        match kind {
            DatabaseKind::Sqlite => run_sqlite(test_name, &body).await?,
            DatabaseKind::Postgres => run_postgres(test_name, &body).await?,
        }
        if let Ok(path) = std::env::var(PARITY_RECORD_VARIABLE) {
            record_to(Path::new(&path), kind, test_name)?;
        }
    }
    Ok(())
}

/// Reads the value of `BB_TEST_DATABASES`; no value means SQLite only.
fn selected(value: Option<&str>) -> Result<Vec<DatabaseKind>, String> {
    let Some(value) = value else {
        return Ok(vec![DatabaseKind::Sqlite]);
    };
    let mut kinds = Vec::new();
    for name in value
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        let kind = parse_database(name).map_err(|unknown| {
            format!("{TEST_DATABASES_VARIABLE} names the unknown database: {unknown}.")
        })?;
        if !kinds.contains(&kind) {
            kinds.push(kind);
        }
    }
    if kinds.is_empty() {
        return Err(format!(
            "{TEST_DATABASES_VARIABLE} is empty. Use {} or both separated by a comma.",
            DatabaseKind::names(", ")
        ));
    }
    Ok(kinds)
}

async fn run_sqlite<F>(test_name: &str, body: &F) -> TestResult
where
    F: AsyncFn(&TestDatabase) -> TestResult,
{
    // The folder (and with it the database file) is deleted when this function ends.
    let folder = tempfile::Builder::new().prefix("bb-test-").tempdir()?;
    let file = folder.path().join("test.db");
    let options = SqliteConnectOptions::new()
        .filename(&file)
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;
    let database = TestDatabase {
        kind: DatabaseKind::Sqlite,
        url: sqlite_url(&file),
        pool: Pool::Sqlite(pool.clone()),
    };
    let result = body(&database).await;
    pool.close().await;
    result.map_err(|error| with_context(test_name, DatabaseKind::Sqlite, &*error))
}

/// Removes a PostgreSQL test database when it goes out of scope, also while a panic unwinds.
struct PostgresGuard {
    admin_url: String,
    name: String,
    dropped: bool,
}

impl PostgresGuard {
    /// Drops the database on a fresh connection.
    async fn drop_now(&mut self) -> Result<(), sqlx::Error> {
        self.dropped = true;
        drop_database(&self.admin_url, &self.name).await
    }
}

impl Drop for PostgresGuard {
    fn drop(&mut self) {
        if self.dropped {
            return;
        }
        // Only reached when the body panicked or setup failed half way. There is no way to wait
        // inside `drop`, so a thread with its own small runtime does the work.
        let (admin_url, name) = (self.admin_url.clone(), self.name.clone());
        let _ = std::thread::spawn(move || {
            if let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                let _ = runtime.block_on(drop_database(&admin_url, &name));
            }
        })
        .join();
    }
}

async fn drop_database(admin_url: &str, name: &str) -> Result<(), sqlx::Error> {
    let mut admin = PgConnection::connect(admin_url).await?;
    // The name is made here from digits and letters only (`unique_database_name`), so it is safe.
    let dropped = admin
        .execute(AssertSqlSafe(format!(
            "DROP DATABASE IF EXISTS \"{name}\" WITH (FORCE)"
        )))
        .await;
    admin.close().await?;
    dropped.map(|_| ())
}

async fn run_postgres<F>(test_name: &str, body: &F) -> TestResult
where
    F: AsyncFn(&TestDatabase) -> TestResult,
{
    let admin_url = std::env::var(TEST_POSTGRES_URL_VARIABLE).map_err(|_| {
        format!(
            "{TEST_POSTGRES_URL_VARIABLE} is not set, but {TEST_DATABASES_VARIABLE} asks for postgres. Run \
             the tests with `cargo xtask db run -- cargo test` (or `cargo xtask ci`)."
        )
    })?;
    let name = unique_database_name();
    let mut guard = PostgresGuard {
        admin_url: admin_url.clone(),
        name: name.clone(),
        dropped: false,
    };
    let mut admin = PgConnection::connect(&admin_url).await?;
    // The name is made here from digits and letters only (`unique_database_name`), so it is safe.
    let created = admin
        .execute(AssertSqlSafe(format!("CREATE DATABASE \"{name}\"")))
        .await;
    // Closed at once: a test holds no connection it does not need (max_connections).
    admin.close().await?;
    created?;
    let url = with_database(&admin_url, &name);
    let outcome = run_on_new_postgres(&url, body).await;
    let dropped = guard.drop_now().await;
    outcome.map_err(|error| with_context(test_name, DatabaseKind::Postgres, &*error))?;
    dropped?;
    Ok(())
}

async fn run_on_new_postgres<F>(url: &str, body: &F) -> TestResult
where
    F: AsyncFn(&TestDatabase) -> TestResult,
{
    let pool = PgPoolOptions::new().max_connections(5).connect(url).await?;
    let database = TestDatabase {
        kind: DatabaseKind::Postgres,
        url: url.to_owned(),
        pool: Pool::Postgres(pool.clone()),
    };
    let result = body(&database).await;
    pool.close().await;
    result
}

fn with_context(test_name: &str, kind: DatabaseKind, error: &(dyn Error + Send + Sync)) -> Failure {
    format!("{test_name} failed on {}: {error}", kind.code()).into()
}

/// A database name that cannot clash with another test run, even in parallel processes. All
/// names start with `bbtest_`, which is how `cargo xtask db run` finds leftovers.
fn unique_database_name() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |time| time.as_nanos());
    format!(
        "bbtest_{}_{}_{nanos}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

/// The URL with its database (the last path segment) replaced; a query string is kept.
fn with_database(url: &str, database: &str) -> String {
    let (base, query) = url
        .split_once('?')
        .map_or((url, None), |(base, query)| (base, Some(query)));
    let server = base.rsplit_once('/').map_or(base, |(server, _)| server);
    let mut result = format!("{server}/{database}");
    if let Some(query) = query {
        result.push('?');
        result.push_str(query);
    }
    result
}

/// `sqlite:<path>?mode=rwc` (no `//`, which would read `C:` as a host name) with the characters
/// that mean something in a URL escaped.
fn sqlite_url(path: &Path) -> String {
    let mut url = String::from("sqlite:");
    for character in path.to_string_lossy().replace('\\', "/").chars() {
        match character {
            ' ' | '%' | '?' | '#' => {
                let _ = write!(url, "%{:02X}", u32::from(character));
            }
            other => url.push(other),
        }
    }
    url.push_str("?mode=rwc");
    url
}

/// Appends `<database> <test name>` to the parity record.
fn record_to(path: &Path, kind: DatabaseKind, test_name: &str) -> TestResult {
    let line = format!(
        "{} {}\n",
        kind.code(),
        test_name.replace(char::is_whitespace, "_")
    );
    // One write call per line keeps lines whole when test programs run side by side.
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?
        .write_all(line.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_a_setting_only_sqlite_runs() {
        assert_eq!(selected(None).unwrap(), [DatabaseKind::Sqlite]);
    }

    #[test]
    fn both_databases_can_be_selected_in_any_order_without_repeats() {
        assert_eq!(
            selected(Some(" postgres, sqlite ,postgres")).unwrap(),
            [DatabaseKind::Postgres, DatabaseKind::Sqlite]
        );
    }

    #[test]
    fn bad_example_unknown_or_empty_selections_are_refused() {
        assert!(selected(Some("mysql")).unwrap_err().contains("mysql"));
        assert!(selected(Some(" , ")).is_err());
    }

    #[test]
    fn the_database_of_a_url_is_replaced_and_the_query_kept() {
        assert_eq!(
            with_database("postgres://u:p@127.0.0.1:1/postgres", "t1"),
            "postgres://u:p@127.0.0.1:1/t1"
        );
        assert_eq!(
            with_database("postgres://u:p@h:1/postgres?sslmode=disable", "t1"),
            "postgres://u:p@h:1/t1?sslmode=disable"
        );
    }

    #[test]
    fn sqlite_urls_use_forward_slashes_and_escape_url_characters() {
        let url = sqlite_url(Path::new("C:\\Users\\a b\\test.db"));
        assert_eq!(url, "sqlite:C:/Users/a%20b/test.db?mode=rwc");
    }

    #[test]
    fn database_names_are_unique_and_safe_in_sql() {
        let first = unique_database_name();
        assert_ne!(first, unique_database_name());
        assert!(first.starts_with("bbtest_"));
        assert!(first.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
    }

    #[test]
    fn the_record_gets_one_line_per_run_and_names_have_no_spaces() {
        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join("record.txt");
        record_to(&file, DatabaseKind::Postgres, "a b").unwrap();
        record_to(&file, DatabaseKind::Sqlite, "a b").unwrap();
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "postgres a_b\nsqlite a_b\n"
        );
    }

    #[tokio::test]
    async fn debug_output_hides_the_password() {
        let pool = SqlitePoolOptions::new().connect_lazy_with(SqliteConnectOptions::new());
        let database = TestDatabase {
            kind: DatabaseKind::Postgres,
            url: "postgres://u:secret@h/d".to_owned(),
            pool: Pool::Sqlite(pool),
        };
        assert!(!format!("{database:?}").contains("secret"));
    }
}
