//! The tests of the database: opening it, its files, and the statistics SQLite plans with.

use sea_query::{Expr, ExprTrait as _};

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
    assert!(!StoreError::from(sqlx::Error::RowNotFound).is_locked());
    assert!(!StoreError::from(sqlx::Error::PoolTimedOut).is_locked());
    assert!(!StoreError::Busy.is_locked());
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
    database.migrate(jiff::Timestamp::now()).await?;
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

/// An index whose first column has five values of 2,000 rows each: more rows of one value than
/// `PRAGMA optimize` reads of an index, as the issues of one project are among 1 M.
const GROWN_INDEX: &str = "grown_by_k";

/// What the statistics of [`GROWN_INDEX`] are when every row was read.
const EXACT: &str = "10000 2000 1";

/// A table analysed while it was empty (as migration 23 does with `issue`), then filled and read
/// through its index.
async fn grown(file: &std::path::Path) -> Result<Database, StoreError> {
    let database = Database::connect_file(file).await?;
    database
        .sqlite_run(&[
            "CREATE TABLE grown (k INTEGER NOT NULL, v INTEGER NOT NULL)",
            "CREATE INDEX grown_by_k ON grown (k, v)",
            "ANALYZE",
            "WITH RECURSIVE n(i) AS (SELECT 0 UNION ALL SELECT i + 1 FROM n WHERE i < 9999) \
             INSERT INTO grown (k, v) SELECT i / 2000, i FROM n",
            "SELECT count(*) FROM grown WHERE k = 1",
        ])
        .await?;
    Ok(database)
}

/// A connection that closes does not write statistics from a short look at an index, which made
/// SQLite read every project and sort it in a search over 1 M issues.
#[tokio::test]
async fn perf_01_a_connection_that_closes_leaves_no_guessed_statistics()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("closed.db");
    grown(&file).await?.close().await;
    let found = Database::connect_file(&file)
        .await?
        .sqlite_statistics_of(GROWN_INDEX)
        .await?;
    assert!(
        found.is_none() || found.as_deref() == Some(EXACT),
        "statistics from a short look at the index: {found:?}"
    );
    Ok(())
}

#[tokio::test]
async fn perf_01_refreshed_statistics_are_exact_and_made_only_when_out_of_date()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let folder = tempfile::tempdir()?;
    let database = grown(&folder.path().join("refreshed.db")).await?;
    assert!(database.refresh_statistics().await?, "the table grew");
    assert_eq!(
        database.sqlite_statistics_of(GROWN_INDEX).await?.as_deref(),
        Some(EXACT)
    );
    assert!(
        !database.refresh_statistics().await?,
        "nothing changed since"
    );
    Ok(())
}

/// A database the migrations made, with its empty tables, settles after one refresh: the daily
/// job and every start of a server do no work on it afterwards. PostgreSQL keeps its own
/// statistics, so both refreshes do nothing there.
#[tokio::test]
async fn perf_01_a_migrated_database_has_nothing_left_to_refresh_after_one_refresh()
-> bb_testing::TestResult {
    bb_testing::each_database("perf_01_statistics_settle", async |db| {
        let database = crate::test_support::open(db).await?;
        let first = database.refresh_statistics().await?;
        assert!(
            database.dialect() == Dialect::Sqlite || !first,
            "PostgreSQL keeps its own"
        );
        assert!(
            !database.refresh_statistics().await?,
            "a second refresh found a table out of date"
        );
        Ok(())
    })
    .await
}

/// Writers that come in more numbers than the busy timeout can wait for do not fail: they queue in
/// the order they came, before they take a connection of the pool, so none is passed by again and
/// again (the way SQLite's own waiting does) and none of them holds a connection while it waits
/// (PERF-01).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn perf_01_writers_in_a_crowd_queue_in_order_and_none_runs_out_of_patience()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    const WRITERS: u64 = 16;
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("crowd.db");
    // The whole queue takes about 16 x 40 ms, far more than the 100 ms a writer would wait for the
    // lock of SQLite.
    let database = Database::connect_with_busy_timeout(
        &format!("sqlite:{}?mode=rwc", file.display()),
        Duration::from_millis(100),
    )
    .await?;
    let order = Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut writers = Vec::new();
    for number in 0..WRITERS {
        let database = database.clone();
        let order = Arc::clone(&order);
        let (queued, in_line) = tokio::sync::oneshot::channel::<()>();
        writers.push(tokio::spawn(async move {
            // The first poll of `begin` takes the place in the queue; the writer says so then, and
            // not before.
            let begin = database.begin();
            tokio::pin!(begin);
            let mut queued = Some(queued);
            let transaction = std::future::poll_fn(|cx| {
                let polled = begin.as_mut().poll(cx);
                if let Some(queued) = queued.take() {
                    let _ = queued.send(());
                }
                polled
            })
            .await?;
            order
                .lock()
                .map_err(|_| StoreError::Corrupt("the order"))?
                .push(number);
            tokio::time::sleep(Duration::from_millis(40)).await;
            transaction.commit().await?;
            Ok::<_, StoreError>(())
        }));
        // One after the other, so the order they came in is known: the next one starts when this
        // one stands in the queue, however busy the computer is.
        in_line.await?;
    }
    for writer in writers {
        writer.await??;
    }
    let order = order
        .lock()
        .map_err(|_| StoreError::Corrupt("the order"))?
        .clone();
    assert_eq!(order, (0..WRITERS).collect::<Vec<_>>());
    Ok(())
}

/// A restore needs the database alone: a server that runs on it, also on another computer, holds
/// it shared, and the restore refuses until it stops (ADR-0046, decision 4). On SQLite the lock of
/// the data folder does this, and the database answers that it is free.
#[tokio::test]
async fn ops_07_a_restore_finds_the_database_held_while_a_server_runs_on_it()
-> bb_testing::TestResult {
    bb_testing::each_database("ops_07_server_hold", async |db| {
        let server = Database::connect(db.url()).await?;
        let other_server = Database::connect(db.url()).await?;
        let restore = Database::connect(db.url()).await?;
        let held = server.hold_as_server().await?;
        // Several servers share the database.
        let also_held = other_server.hold_as_server().await?;
        let postgres = server.dialect() == Dialect::Postgres;
        assert_eq!(restore.hold_alone().await?.is_none(), postgres);
        drop(held);
        drop(also_held);
        // Once every server has stopped, the restore holds it, and a server that starts waits.
        let alone = restore.hold_alone().await?;
        assert!(alone.is_some());
        if postgres {
            let waiting =
                tokio::time::timeout(Duration::from_millis(300), server.hold_as_server()).await;
            assert!(waiting.is_err(), "a server waits while the restore runs");
        }
        drop(alone);
        let _held_again = server.hold_as_server().await?;
        Ok(())
    })
    .await
}

/// Ends the connections of other clients that hold the lock of running servers on this database,
/// as a broken link or a restart of the database server would. The lock is found by its class,
/// which no lock on a table has (a table is named by its catalog, a number far above it).
async fn end_holders(database: &Database) -> Result<(), StoreError> {
    let column = |name: &str| Expr::col(sea_query::Alias::new(name));
    // `pg_locks` shows the locks of every database of the server; only this one's count, so the
    // tests of other databases go on undisturbed.
    let mut this_database = SelectStatement::new();
    this_database
        .column(sea_query::Alias::new("oid"))
        .from(sea_query::Alias::new("pg_database"))
        .and_where(column("datname").eq(Func::cust(sea_query::Alias::new("current_database"))));
    let mut select = SelectStatement::new();
    select
        .expr(Func::cust(sea_query::Alias::new("pg_terminate_backend")).arg(column("pid")))
        .from(sea_query::Alias::new("pg_locks"))
        .and_where(column("database").in_subquery(this_database))
        .and_where(column("classid").eq(LockClass::ServerRunning as i32))
        .and_where(column("pid").ne(Func::cust(sea_query::Alias::new("pg_backend_pid"))));
    database.fetch_all(&select).await?;
    Ok(())
}

/// The hold of a server lives on a connection of its own (ADR-0046, decision 4). When that
/// connection breaks, the hold is taken again; when a restore took the database meanwhile, the
/// server is told that it lost the hold and must stop. On SQLite there is no such connection.
#[tokio::test]
async fn ops_07_a_server_whose_hold_broke_takes_it_again_or_is_told_to_stop()
-> bb_testing::TestResult {
    bb_testing::each_database("ops_07_hold_broke", async |db| {
        let server = Database::connect(db.url()).await?;
        if server.dialect() != Dialect::Postgres {
            return Ok(());
        }
        let restore = Database::connect(db.url()).await?;
        // Broken while nobody else wants the database: taken again, and the server goes on.
        let mut held = server
            .hold_as_server_checking(Duration::from_millis(100))
            .await?;
        end_holders(&restore).await?;
        tokio::time::sleep(Duration::from_millis(800)).await;
        assert!(restore.hold_alone().await?.is_none(), "taken again");
        assert!(
            tokio::time::timeout(Duration::from_millis(300), held.lost())
                .await
                .is_err(),
            "the server goes on"
        );
        drop(held);
        // Broken while a restore takes the database: the server is told to stop.
        let mut held = server
            .hold_as_server_checking(Duration::from_secs(1))
            .await?;
        end_holders(&restore).await?;
        let mut alone = None;
        for _ in 0..50 {
            alone = restore.hold_alone().await?;
            if alone.is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(alone.is_some(), "the restore took the database");
        assert!(
            tokio::time::timeout(Duration::from_secs(10), held.lost())
                .await
                .is_ok(),
            "the server was told to stop"
        );
        Ok(())
    })
    .await
}
