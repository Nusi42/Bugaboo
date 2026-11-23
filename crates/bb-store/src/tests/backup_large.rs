//! A larger backup (OPS-07, ADR-0046 decision 2): thousands of rows go into a copy that is written
//! in many pieces and come back whole, a copy as the program wrote it before it streamed the rows
//! still restores, and the work folder goes however the backup ends.

use std::fmt::Write as _;
use std::time::{Duration, Instant};

use sea_query::{Alias, Expr, Order, Query};

use super::copy::changed_backup;
use super::*;
use crate::backup::catalog;
use crate::backup::copy::table_head;
use crate::backup::value::row_to_json;
use crate::schema::ProjectRow;

/// How many projects the larger backup holds.
const PROJECTS: usize = 5_000;

/// How long a stopped backup may take to let its work folder go.
const LET_GO_WITHIN: Duration = Duration::from_secs(30);

/// A name long enough that the copy of the projects fills several pieces of the copy.
fn long_name(number: usize) -> String {
    format!(
        "{number:05} {}",
        "Printer jams on the third floor ".repeat(40)
    )
}

/// Adds [`PROJECTS`] projects with long names, in a few large inserts.
async fn fill(database: &Database) -> Result<(), Failure> {
    let mut source = ids();
    let mut transaction = database.begin().await?;
    for (number, chunk) in (0..PROJECTS).collect::<Vec<_>>().chunks(250).enumerate() {
        let mut insert = Query::insert();
        insert.into_table(ProjectRow::Table).columns([
            ProjectRow::Id,
            ProjectRow::ShortName,
            ProjectRow::Name,
            ProjectRow::Version,
        ]);
        for place in chunk {
            insert.values([
                Expr::val(crate::db::StoredId::stored(
                    bb_core::id::ProjectId::generate(&mut source)?,
                )),
                Expr::val(format!("L{number}X{place}")),
                Expr::val(long_name(*place)),
                Expr::val(1_i64),
            ])?;
        }
        transaction.execute(&insert).await?;
    }
    transaction.commit().await?;
    Ok(())
}

/// The names of every project, in order.
async fn names(database: &Database) -> Result<Vec<String>, Failure> {
    let mut select = Query::select();
    select.column(ProjectRow::Name).from(ProjectRow::Table);
    let mut names: Vec<String> = database
        .fetch_all(&select)
        .await?
        .iter()
        .map(|row| row.get(ProjectRow::Name))
        .collect::<Result<_, _>>()?;
    names.sort_unstable();
    Ok(names)
}

/// The work folders of backups in `folder`.
fn work_folders(folder: &Path) -> Result<Vec<PathBuf>, Failure> {
    Ok(std::fs::read_dir(folder)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with(".backup-"))
        .map(|entry| entry.path())
        .collect())
}

/// Whether a backup has begun to write its copy into its work folder in `folder`.
fn writing(folder: &Path) -> Result<bool, Failure> {
    let copy = |work: &Path| {
        [DatabaseKind::Sqlite, DatabaseKind::Postgres]
            .iter()
            .any(|kind| work.join(kind.member()).exists())
    };
    Ok(work_folders(folder)?.iter().any(|work| copy(work)))
}

/// Whether a backup has begun to write its archive in its work folder in `folder`.
fn archiving(folder: &Path) -> Result<bool, Failure> {
    Ok(work_folders(folder)?
        .iter()
        .any(|work| staged_archive(work).exists()))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ops_07_a_backup_of_thousands_of_rows_restores_every_row() -> TestResult {
    each_database("ops_07_large_backup", async |db| {
        let installation = Installation::new(db)?;
        let database = installation.open().await?;
        fill(&database).await?;
        for number in 1..=20 {
            save(&database, number).await?;
        }
        let expected = names(&database).await?;
        let backup = database
            .create_backup(&installation.places, Purpose::Regular, at(10))
            .await?;
        assert!(work_folders(&installation.places.folder)?.is_empty());
        database.close().await;
        installation
            .restore(&backup.path, &[], at(20))
            .await
            .map_err(failed)?;
        let restored = installation.open().await?;
        assert_eq!(names(&restored).await?.len(), PROJECTS);
        assert_eq!(names(&restored).await?, expected);
        assert_eq!(saved(&restored).await?, (1..=20).collect::<Vec<_>>());
        restored.close().await;
        Ok(())
    })
    .await
}

/// Whether a backup in `folder` has come to a stage of its work.
type Stage = fn(&Path) -> Result<bool, Failure>;

/// Waits until the work folders in `folder` are gone.
async fn let_go(folder: &Path) -> Result<(), Failure> {
    let started = Instant::now();
    while !work_folders(folder)?.is_empty() {
        assert!(started.elapsed() < LET_GO_WITHIN, "the work folder stays");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ops_07_a_backup_stopped_half_way_leaves_no_work_folder() -> TestResult {
    each_database("ops_07_backup_stopped", async |db| {
        let installation = Installation::new(db)?;
        let database = installation.open().await?;
        fill(&database).await?;
        let folder = installation.places.folder.clone();
        // Stopped while it copies, and while a blocking thread writes the archive, which goes on
        // after the stop and must not put the archive among the backups.
        let stages: [Stage; 2] = [writing, archiving];
        for (stage, reached) in stages.into_iter().enumerate() {
            let begun = async {
                while !reached(&folder)? {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
                Ok::<_, Failure>(())
            };
            // The backup is dropped as a request that runs out of time drops it.
            tokio::select! {
                made = database.create_backup(&installation.places, Purpose::Regular, at(10)) => {
                    made?;
                    return Err(format!("the backup ended before stage {stage}").into());
                }
                seen = begun => seen?,
            }
            let_go(&folder).await?;
            assert!(backups_in(&folder)?.is_empty(), "stage {stage}");
        }
        database.close().await;
        Ok(())
    })
    .await
}

#[test]
fn ops_07_the_work_folder_of_a_backup_whose_program_was_killed_goes_after_an_hour() -> TestResult {
    let folder = tempfile::tempdir()?;
    let long_ago = std::time::SystemTime::now() - 2 * LEFTOVER_AFTER;
    let work = |name: &str, old: bool| -> Result<PathBuf, Failure> {
        let work = folder.path().join(name);
        std::fs::create_dir(&work)?;
        // The lock of a killed program went with it; its file stays.
        for file in [work.join("bugaboo.db"), work.join(work::HELD)] {
            std::fs::write(&file, b"half a copy")?;
            if old {
                std::fs::File::options()
                    .write(true)
                    .open(&file)?
                    .set_modified(long_ago)?;
            }
        }
        Ok(work)
    };
    let killed = work(".backup-00aa", true)?;
    let running = work(".backup-00bb", false)?;
    let restoring = work(".restore-00cc", true)?;
    // A backup beside the server (the command line) that has run for hours, in a folder whose
    // times say nothing changed: it holds its lock, so it stays.
    let long = WorkFolder::new(folder.path())?;
    for entry in std::fs::read_dir(long.path())? {
        std::fs::File::options()
            .write(true)
            .open(entry?.path())?
            .set_modified(long_ago)?;
    }
    remove_leftovers(folder.path());
    assert!(!killed.exists());
    assert!(running.exists());
    assert!(restoring.exists());
    assert!(long.path().exists());
    Ok(())
}

/// A time limit on each statement that an administrator of PostgreSQL may set for the database.
const DATABASE_LIMIT: &str = "1s";

/// A statement that takes longer than [`DATABASE_LIMIT`].
fn longer_than_the_limit() -> sea_query::SelectStatement {
    let mut select = Query::select();
    select.expr(Expr::cust("pg_sleep(1.5) IS NULL"));
    select
}

#[tokio::test]
async fn ops_07_a_time_limit_of_the_database_does_not_stop_a_backup() -> TestResult {
    each_database("ops_07_backup_time_limit", async |db| {
        let Some(pool) = db.postgres_pool() else {
            return Ok(());
        };
        let installation = Installation::new(db)?;
        let database = installation.open().await?;
        fill(&database).await?;
        database.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DO $$ BEGIN EXECUTE format('ALTER DATABASE %I SET statement_timeout = %L',              current_database(), '{DATABASE_LIMIT}'); END $$"
        )))
        .execute(pool)
        .await?;
        // New connections take the limit: a plain read is stopped by it.
        let database = installation.open().await?;
        let mut plain = database.begin_read().await?;
        assert!(plain.fetch_all(&longer_than_the_limit()).await.is_err());
        drop(plain);
        // The transaction of the copy of a backup is not, and neither is a backup.
        let mut copy = database.begin_read().await?;
        copy.without_time_limits().await?;
        copy.fetch_all(&longer_than_the_limit()).await?;
        copy.commit().await?;
        database
            .create_backup(&installation.places, Purpose::Regular, at(10))
            .await?;
        database.close().await;
        Ok(())
    })
    .await
}

/// The copy of PostgreSQL as the program wrote it before it streamed the rows: each table read in
/// batches in the order of the places of its rows (`ctid`).
async fn copy_in_the_order_of_places(database: &Database, file: &Path) -> Result<(), Failure> {
    let mut transaction = database.begin_read().await?;
    let mut text = String::new();
    for table in catalog::tables(&mut transaction).await? {
        let columns = catalog::columns(&mut transaction, &table).await?;
        writeln!(text, "{}", table_head(&table, &columns))?;
        let kinds: Vec<_> = columns.iter().map(|column| column.kind).collect();
        let mut select = Query::select();
        select
            .columns(columns.iter().map(|column| Alias::new(&column.name)))
            .from(Alias::new(&table))
            .order_by_expr(Expr::cust("ctid"), Order::Asc);
        for row in transaction.fetch_all(&select).await? {
            writeln!(text, "{}", row_to_json(&row, &kinds)?)?;
        }
    }
    transaction.commit().await?;
    std::fs::write(file, text)?;
    Ok(())
}

/// The lines of a copy, table by table, each table's rows sorted.
fn tables_of(copy: &str) -> Vec<(String, Vec<String>)> {
    let mut tables: Vec<(String, Vec<String>)> = Vec::new();
    for line in copy.lines() {
        match tables.last_mut() {
            Some((_, rows)) if line.starts_with('[') => rows.push(line.to_owned()),
            _ => tables.push((line.to_owned(), Vec::new())),
        }
    }
    for (_, rows) in &mut tables {
        rows.sort_unstable();
    }
    tables
}

#[tokio::test]
async fn ops_07_a_copy_the_older_program_wrote_in_batches_still_restores() -> TestResult {
    each_database("ops_07_older_copy", async |db| {
        // The copy of SQLite is `VACUUM INTO` as before.
        if db.kind() != bb_testing::DatabaseKind::Postgres {
            return Ok(());
        }
        let installation = Installation::new(db)?;
        let database = installation.open().await?;
        fill(&database).await?;
        for number in 1..=20 {
            save(&database, number).await?;
        }
        let expected = names(&database).await?;
        let backup = database
            .create_backup(&installation.places, Purpose::Regular, at(10))
            .await?;
        let older_copy = installation.folder.path().join("older-copy.jsonl");
        copy_in_the_order_of_places(&database, &older_copy).await?;
        database.close().await;
        let older = installation.folder.path().join("older.backup");
        changed_backup(&installation.places.keys, &backup.path, &older, |copy| {
            // The same lines: only their order within a table may differ.
            assert_eq!(
                tables_of(&std::fs::read_to_string(copy)?),
                tables_of(&std::fs::read_to_string(&older_copy)?)
            );
            std::fs::copy(&older_copy, copy)?;
            Ok(())
        })?;
        installation
            .restore(&older, &[], at(20))
            .await
            .map_err(failed)?;
        let restored = installation.open().await?;
        assert_eq!(names(&restored).await?, expected);
        assert_eq!(saved(&restored).await?, (1..=20).collect::<Vec<_>>());
        restored.close().await;
        Ok(())
    })
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn adr_0017_a_backup_of_a_job_keeps_its_lease_and_stops_when_another_worker_has_it()
-> TestResult {
    use std::sync::atomic::AtomicUsize;

    use bb_app::backups::{BackupFiles as _, KeepLease};

    each_database("adr_0017_backup_lease", async |db| {
        let installation = Installation::new(db)?;
        let database = installation.open().await?;
        fill(&database).await?;
        let folder = installation.places.folder.clone();
        let named = folder.clone();
        let backups = FolderBackups::new(
            database.clone(),
            installation.places.clone(),
            Arc::new(move |_| Ok(named.clone())),
        );
        let renewed = AtomicUsize::new(0);
        let renewals = &renewed;
        let keep = |held: bool| KeepLease {
            every: Duration::from_millis(1),
            renew: Box::new(move || {
                renewals.fetch_add(1, Ordering::SeqCst);
                Box::pin(async move { held })
            }),
        };
        // While the worker holds the job, the lease is renewed and the backup is made.
        backups
            .create(None, Purpose::Regular, at(10), Some(keep(true)))
            .await?;
        assert!(renewed.load(Ordering::SeqCst) > 0, "the lease was renewed");
        assert_eq!(backups_in(&folder)?.len(), 1);
        // Once another worker has it, the backup stops and leaves nothing.
        let lost = backups
            .create(None, Purpose::Regular, at(20), Some(keep(false)))
            .await;
        assert!(lost.is_err());
        let_go(&folder).await?;
        assert_eq!(backups_in(&folder)?.len(), 1);
        // Lost while a blocking thread writes the archive: the thread finishes, and the archive
        // never appears among the backups, where nobody would record it.
        let watched = folder.clone();
        let lost = backups
            .create(
                None,
                Purpose::Regular,
                at(30),
                Some(KeepLease {
                    every: Duration::from_millis(1),
                    renew: Box::new(move || {
                        let held = !archiving(&watched).unwrap_or(false);
                        Box::pin(async move { held })
                    }),
                }),
            )
            .await;
        assert!(lost.is_err());
        let_go(&folder).await?;
        assert_eq!(backups_in(&folder)?.len(), 1);
        database.close().await;
        Ok(())
    })
    .await
}
