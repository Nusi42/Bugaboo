//! The tests of the database: opening it, its files, and the statistics SQLite plans with.

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
