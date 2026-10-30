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
