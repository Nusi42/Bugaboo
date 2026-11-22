//! Backups and restores on both databases (OPS-07, ADR-0046): a backup made while people save,
//! what it holds and what it does not, the checks before a restore, and everything a restore does
//! after it put the database back.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

use bb_app::auth::{AccessKeyRecord, AccountStore as _, AuthStore as _, SessionStore as _};
use bb_app::ports::ChangeStore as _;
use bb_core::access_key::{AccessKey, KeyChoice};
use bb_core::backup::manifest::BackupProblem;
use bb_core::backup::name::{BackupName, Purpose};
use bb_core::backup::replay::{ReplayKind, ReplayLine};
use bb_core::id::{AccessKeyId, IdSource, SessionId, UserId};
use bb_core::security_log::Digest;
use bb_core::sign_in::{SessionStage, SignInSession};
use bb_testing::{Failure, TestDatabase, TestResult, each_database};
use jiff::{SignedDuration, Timestamp};
use sea_query::{Expr, ExprTrait as _, Query};

use super::*;
use crate::anchor::LogSeal;
use crate::auth::AuthStorage;
use crate::chain::Verification;
use crate::keys::KeyRing;
use crate::schema::{AccessKeyRow, Event, SignInSession as SessionColumns};
use crate::test_support::{at, event, ids, person};

/// The database of an installation of the test.
enum Place {
    /// A SQLite file of its own.
    File(PathBuf),
    /// The PostgreSQL database of the test, by its address.
    Postgres(String),
}

/// An installation of the test: its data folder, its database and its keys.
pub(crate) struct Installation {
    folder: tempfile::TempDir,
    place: Place,
    pub(crate) places: BackupPlaces,
    pub(crate) seal: LogSeal,
}

impl Installation {
    pub(crate) fn new(db: &TestDatabase) -> Result<Self, Failure> {
        let folder = tempfile::tempdir()?;
        for name in ["keys", "logs", "backups", "files", "privacy"] {
            std::fs::create_dir(folder.path().join(name))?;
        }
        // The harness keeps the SQLite file of the test open, which Windows does not let a restore
        // replace; the installation gets a file of its own.
        let place = match db.kind() {
            bb_testing::DatabaseKind::Sqlite => Place::File(folder.path().join("bugaboo.db")),
            bb_testing::DatabaseKind::Postgres => Place::Postgres(db.url().to_owned()),
        };
        let keys = Arc::new(KeyRing::open_or_create(&folder.path().join("keys"))?);
        let seal = LogSeal::new(&keys, folder.path().join("logs").join("security-head.log"));
        let files = folder.path().join("files");
        let places = BackupPlaces {
            folder: folder.path().join("backups"),
            files,
            replay_list: folder.path().join("privacy").join("replay.list"),
            write_replay_list: Arc::new(|path: &Path, bytes: &[u8]| {
                if let Some(folder) = path.parent() {
                    std::fs::create_dir_all(folder)?;
                }
                std::fs::write(path, bytes)
            }),
            file_paths: Arc::new(|hash: &Digest| {
                let name = hex::encode(hash.as_bytes());
                vec![format!("content/{}/{name}", &name[..2])]
            }),
            keys,
        };
        Ok(Self {
            folder,
            place,
            places,
            seal,
        })
    }

    fn keys(&self) -> Arc<KeyRing> {
        Arc::clone(&self.places.keys)
    }

    pub(crate) async fn open(&self) -> Result<Database, StoreError> {
        let database = self.open_unchecked().await?;
        database.migrate(at(0)).await?;
        Ok(database)
    }

    /// The database to restore into, as the command line finds it: by opening it.
    async fn target(&self) -> Result<RestoreTarget, StoreError> {
        Ok(RestoreTarget::of(self.open_unchecked().await?).await)
    }

    /// Restores with `keys`, and lets the target go after.
    async fn restore_with(
        &self,
        archive: &Path,
        keys: &Arc<KeyRing>,
        more: &[PathBuf],
        now: Timestamp,
    ) -> Result<RestoreReport, RestoreFailure> {
        let target = self.target().await.map_err(|error| RestoreFailure {
            error,
            before: None,
        })?;
        let places = BackupPlaces {
            keys: Arc::clone(keys),
            ..self.places.clone()
        };
        let result = restore(
            RestoreInput {
                archive,
                target: &target,
                places: &places,
                seal: &self.seal,
                more_replay_lists: more,
                now,
            },
            &mut bb_app::context::SystemIds,
        )
        .await;
        if let RestoreTarget::Postgres(database) = &target {
            database.close().await;
        }
        result
    }

    pub(crate) fn storage(&self, database: &Database) -> AuthStorage {
        AuthStorage::new(database.clone(), self.keys(), self.seal.clone())
    }

    pub(crate) async fn restore(
        &self,
        archive: &Path,
        more: &[PathBuf],
        now: Timestamp,
    ) -> Result<RestoreReport, RestoreFailure> {
        self.restore_with(archive, &self.keys(), more, now).await
    }
}

/// Saves the change numbered `number`: an event whose reason is the number.
pub(crate) async fn save(database: &Database, number: i64) -> Result<(), StoreError> {
    let mut saved = event(&mut bb_app::context::SystemIds, number);
    saved.reason = Some(number.to_string());
    database.record_change(&saved, async |_| Ok(())).await
}

/// The numbers of the saved changes, in order.
pub(crate) async fn saved(database: &Database) -> Result<Vec<i64>, StoreError> {
    let mut select = Query::select();
    select.column(Event::Reason).from(Event::Table);
    let mut numbers: Vec<i64> = database
        .fetch_all(&select)
        .await?
        .iter()
        .filter_map(|row| row.get::<Option<String>>(Event::Reason).ok().flatten())
        .filter_map(|reason| reason.parse().ok())
        .collect();
    numbers.sort_unstable();
    Ok(numbers)
}

/// A sign-in at `seconds`, for the security log.
pub(crate) fn signed_in(seconds: i64) -> bb_app::auth::SecurityEvent {
    bb_app::auth::SecurityEvent {
        time: at(seconds),
        person: None,
        action: bb_core::sign_in::SecurityAction::SignInSucceeded,
        subject: None,
        network: None,
    }
}

fn refused(failure: RestoreFailure) -> Result<BackupProblem, Failure> {
    assert!(failure.before.is_none(), "a refused backup made no copy");
    match failure.error {
        StoreError::BackupRefused(problem) => Ok(problem),
        other => Err(format!("expected a refusal, got {other}").into()),
    }
}

fn failed(failure: RestoreFailure) -> Failure {
    let RestoreFailure { error, .. } = failure;
    error.to_string().into()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ops_07_a_backup_made_while_people_save_restores_every_saved_change_up_to_its_moment()
-> TestResult {
    each_database("ops_07_backup_while_saving", async |db| {
        let installation = Installation::new(db)?;
        let database = installation.open().await?;
        for number in 1..=20 {
            save(&database, number).await?;
        }
        let stop = Arc::new(AtomicBool::new(false));
        let last = Arc::new(AtomicI64::new(20));
        let writer = {
            let (database, stop, last) = (database.clone(), stop.clone(), last.clone());
            tokio::spawn(async move {
                let mut number = 20;
                while !stop.load(Ordering::SeqCst) {
                    number += 1;
                    save(&database, number).await?;
                    last.store(number, Ordering::SeqCst);
                }
                Ok::<_, StoreError>(())
            })
        };
        let before = last.load(Ordering::SeqCst);
        let backup = database
            .create_backup(&installation.places, Purpose::Regular, Timestamp::now())
            .await?;
        let after = last.load(Ordering::SeqCst);
        stop.store(true, Ordering::SeqCst);
        writer.await??;
        let written = last.load(Ordering::SeqCst);
        database.close().await;
        let report = installation
            .restore(&backup.path, &[], Timestamp::now())
            .await
            .map_err(failed)?;
        assert_eq!(report.sessions_ended, 0);
        let restored = installation.open().await?;
        let numbers = saved(&restored).await?;
        let count = i64::try_from(numbers.len())?;
        // One consistent state: every change saved before the copy, none missing in between.
        assert_eq!(numbers, (1..=count).collect::<Vec<_>>());
        assert!(
            count >= before && count <= Ord::max(after, before) + 1,
            "{before} {count} {after}"
        );
        assert!(written >= count);
        restored.close().await;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ops_07_a_backup_holds_no_keys() -> TestResult {
    each_database("ops_07_no_keys", async |db| {
        let installation = Installation::new(db)?;
        let database = installation.open().await?;
        let storage = installation.storage(&database);
        let name = bb_core::technical::TechnicalName::new("mail.password")?;
        database
            .put_secret(&installation.keys(), &name, b"hunter2-mail", at(1))
            .await?;
        let _ = storage;
        let backup = database
            .create_backup(&installation.places, Purpose::Regular, at(2))
            .await?;
        database.close().await;
        let bytes = std::fs::read(&backup.path)?;
        let keys = installation.folder.path().join("keys");
        for entry in std::fs::read_dir(keys)? {
            let key = std::fs::read_to_string(entry?.path())?;
            for line in key.lines() {
                let secret = line.rsplit(' ').next().unwrap_or(line);
                assert!(
                    !bytes
                        .windows(secret.len())
                        .any(|window| window == secret.as_bytes()),
                    "a key is in the backup"
                );
            }
        }
        assert!(!bytes.windows(12).any(|window| window == b"hunter2-mail"));
        assert!(!bytes.windows(5).any(|window| window == b"keys/"));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ops_07_a_damaged_or_newer_backup_is_refused_and_nothing_changes() -> TestResult {
    each_database("ops_07_refused", async |db| {
        let installation = Installation::new(db)?;
        let database = installation.open().await?;
        save(&database, 1).await?;
        let backup = database
            .create_backup(&installation.places, Purpose::Regular, at(10))
            .await?;
        // A newer Bugaboo: a migration this program does not know.
        database.record_newer_migration(at(11)).await?;
        let newer = database
            .create_backup(&installation.places, Purpose::Regular, at(12))
            .await?;
        save(&database, 2).await?;
        database.close().await;
        let mut bytes = std::fs::read(&backup.path)?;
        let middle = bytes.len() / 2;
        bytes[middle] ^= 0xff;
        let damaged = installation.folder.path().join("damaged.backup");
        std::fs::write(&damaged, bytes)?;
        let problem = refused(
            installation
                .restore(&damaged, &[], at(20))
                .await
                .expect_err("a damaged backup"),
        )?;
        assert!(
            matches!(
                problem,
                BackupProblem::Damaged(_) | BackupProblem::Unreadable | BackupProblem::Missing(_)
            ),
            "{problem:?}"
        );
        let problem = refused(
            installation
                .restore(&newer.path, &[], at(20))
                .await
                .expect_err("a newer backup"),
        )?;
        assert_eq!(
            problem,
            BackupProblem::NewerMigration(crate::latest_version() + 1)
        );
        // Nothing changed, and no copy of the present was made.
        let database = installation.open_unchecked().await?;
        assert_eq!(saved(&database).await?, [1, 2]);
        assert_eq!(backups_in(&installation.places.folder)?.len(), 2);
        database.close().await;
        Ok(())
    })
    .await
}

impl Installation {
    /// Opens the database without migrating it (it may hold a newer migration).
    pub(crate) async fn open_unchecked(&self) -> Result<Database, StoreError> {
        match &self.place {
            Place::File(file) => Database::connect_file(file).await,
            Place::Postgres(url) => Database::connect(url).await,
        }
    }
}

#[tokio::test]
async fn ops_07_a_restore_keeps_a_backup_of_the_present_first() -> TestResult {
    each_database("ops_07_before_restore", async |db| {
        let installation = Installation::new(db)?;
        let database = installation.open().await?;
        save(&database, 1).await?;
        let backup = database
            .create_backup(&installation.places, Purpose::Regular, at(10))
            .await?;
        save(&database, 2).await?;
        database.close().await;
        let report = installation
            .restore(&backup.path, &[], at(20))
            .await
            .map_err(failed)?;
        assert_eq!(report.before.file.name.purpose, Purpose::BeforeRestore);
        assert!(report.before.path.is_file());
        // The copy of the present holds the change the restore took away: restoring it puts it
        // back.
        let restored = installation.open().await?;
        assert_eq!(saved(&restored).await?, [1]);
        restored.close().await;
        installation
            .restore(&report.before.path, &[], at(30))
            .await
            .map_err(failed)?;
        let again = installation.open().await?;
        assert_eq!(saved(&again).await?, [1, 2]);
        // The record of that restore names the backup by its own kind, made before a restore.
        let runs = again.backup_runs(10).await?;
        let restored = runs
            .iter()
            .find(|run| run.event == bb_core::backup::run::BackupEvent::Restored)
            .ok_or("no record of the restore")?;
        assert_eq!(restored.name, report.before.file.name);
        assert_eq!(restored.name.purpose, Purpose::BeforeRestore);
        again.close().await;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ops_07_a_restore_of_an_older_backup_runs_its_migrations() -> TestResult {
    each_database("ops_07_older_backup", async |db| {
        let installation = Installation::new(db)?;
        let database = installation.open_unchecked().await?;
        let older = crate::latest_version() - 1;
        database.migrate_up_to(older, at(0)).await?;
        save(&database, 1).await?;
        let backup = database
            .create_backup(&installation.places, Purpose::Regular, at(10))
            .await?;
        database.migrate(at(11)).await?;
        database.close().await;
        let report = installation
            .restore(&backup.path, &[], at(20))
            .await
            .map_err(failed)?;
        assert_eq!(report.migrations_run, [crate::latest_version()]);
        let restored = installation.open_unchecked().await?;
        assert_eq!(restored.schema_state().await?, crate::SchemaState::Current);
        assert_eq!(saved(&restored).await?, [1]);
        restored.close().await;
        Ok(())
    })
    .await
}

async fn a_key(
    storage: &AuthStorage,
    source: &mut impl IdSource,
) -> Result<(UserId, AccessKeyId), Failure> {
    let ada = person(source, "ada@example.test");
    let key = AccessKeyId::generate(source);
    let record = AccessKeyRecord {
        key: AccessKey {
            id: key,
            owner: ada.id,
            name: "Script".to_owned(),
            prefix: "00000001".to_owned(),
            secret_hash: Digest::new([1; 32]),
            choice: KeyChoice::SeeOnly,
            created_at: at(1),
            expires_at: at(100_000),
            last_used_at: None,
            agent: None,
        },
        permissions: Vec::new(),
        projects: Vec::new(),
    };
    let session = SignInSession {
        id: SessionId::generate(source),
        person: ada.id,
        token_hash: Digest::new([2; 32]),
        stage: SessionStage::Active,
        created_at: at(1),
        last_seen_at: at(1),
        network: None,
        device: "Firefox on Windows".to_owned(),
    };
    let change = event(source, 1);
    let id = ada.id;
    storage
        .record_change(&change, move |tx| {
            Box::pin(async move {
                AuthStorage::insert_person(tx, &ada, "hash", at(0)).await?;
                crate::access_keys::insert_access_key(tx, &record).await?;
                AuthStorage::insert_session(tx, &session).await
            })
        })
        .await?;
    Ok((id, key))
}

async fn has_key(database: &Database, key: AccessKeyId) -> Result<bool, StoreError> {
    let mut select = Query::select();
    select
        .column(AccessKeyRow::Id)
        .from(AccessKeyRow::Table)
        .and_where(Expr::col(AccessKeyRow::Id).eq(crate::db::StoredId::stored(key)));
    Ok(database.fetch_optional(&select).await?.is_some())
}

#[tokio::test]
async fn ops_07_every_restore_signs_everybody_out() -> TestResult {
    each_database("ops_07_sign_out", async |db| {
        let installation = Installation::new(db)?;
        let database = installation.open().await?;
        let storage = installation.storage(&database);
        let backup_before = database
            .create_backup(&installation.places, Purpose::Regular, at(5))
            .await?;
        a_key(&storage, &mut ids()).await?;
        // A backup that holds the session, restored: the session ends all the same.
        let backup = database
            .create_backup(&installation.places, Purpose::Regular, at(10))
            .await?;
        database.close().await;
        let report = installation
            .restore(&backup.path, &[], at(20))
            .await
            .map_err(failed)?;
        assert_eq!(report.sessions_ended, 1);
        let restored = installation.open().await?;
        let mut select = Query::select();
        select
            .column(SessionColumns::Id)
            .from(SessionColumns::Table);
        assert!(restored.fetch_all(&select).await?.is_empty());
        restored.close().await;
        let _ = backup_before;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_15_after_a_restore_the_log_verifies_and_names_what_is_gone() -> TestResult {
    each_database("sec_15_restore_log", async |db| {
        let installation = Installation::new(db)?;
        let database = installation.open().await?;
        let storage = installation.storage(&database);
        storage.append_security_log(&signed_in(1)).await?;
        let backup = database
            .create_backup(&installation.places, Purpose::Regular, at(10))
            .await?;
        for seconds in 11..14 {
            storage.append_security_log(&signed_in(seconds)).await?;
        }
        let head = match database.verify_security_log(&installation.seal).await? {
            Verification::Valid { head, .. } => head.map_or(0, |(seq, _)| seq),
            broken @ Verification::Broken { .. } => return Err(format!("{broken:?}").into()),
        };
        database.close().await;
        let report = installation
            .restore(&backup.path, &[], at(20))
            .await
            .map_err(failed)?;
        // The entries after the backup's head are named, and the log verifies anchored.
        let backup_head = backup.log_head.map_or(0, |(seq, _)| seq);
        assert_eq!(report.lost, Some((backup_head + 1, head)));
        assert!(
            matches!(report.log, Verification::Valid { anchored: true, .. }),
            "{:?}",
            report.log
        );
        let restored = installation.open().await?;
        let runs = restored.backup_runs(10).await?;
        let restore = runs
            .iter()
            .find(|run| run.event == bb_core::backup::run::BackupEvent::Restored)
            .ok_or("no record of the restore")?;
        assert_eq!(
            (restore.lost_from, restore.lost_to),
            (Some(backup_head + 1), Some(head))
        );
        let entries = restored.security_log_entries(0, 100).await?;
        assert!(
            entries
                .iter()
                .any(|entry| entry.action.as_str() == "backup.restored")
        );
        restored.close().await;
        Ok(())
    })
    .await
}

/// Keys of another installation in `folder`; with `log_key_of`, its log key is that folder's.
fn other_keys(folder: &Path, log_key_of: Option<&Path>) -> Result<Arc<KeyRing>, Failure> {
    if let Some(keys) = log_key_of {
        std::fs::copy(keys.join("log.key"), folder.join("log.key"))?;
    }
    Ok(Arc::new(KeyRing::open_or_create(folder)?))
}

#[tokio::test]
async fn ops_07_a_wrong_master_key_stops_the_restore_before_anything_is_replaced() -> TestResult {
    each_database("ops_07_wrong_key", async |db| {
        let installation = Installation::new(db)?;
        let database = installation.open().await?;
        save(&database, 1).await?;
        // A backup with no stored secret: the check value of the master key shows a foreign one.
        let bare = database
            .create_backup(&installation.places, Purpose::Regular, at(5))
            .await?;
        let name = bb_core::technical::TechnicalName::new("mail.password")?;
        database
            .put_secret(&installation.keys(), &name, b"hunter2-mail", at(6))
            .await?;
        let backup = database
            .create_backup(&installation.places, Purpose::Regular, at(10))
            .await?;
        save(&database, 2).await?;
        database.close().await;
        let own = installation.folder.path().join("keys");
        for archive in [&bare.path, &backup.path] {
            // A master key of another installation beside this installation's log key.
            let other = tempfile::tempdir()?;
            let keys = other_keys(other.path(), Some(&own))?;
            let failure = installation
                .restore_with(archive, &keys, &[], at(20))
                .await
                .expect_err("the master key is another installation's");
            assert!(
                matches!(failure.error, StoreError::KeysDoNotOpen),
                "{}",
                failure.error
            );
            assert!(failure.before.is_none());
        }
        let database = installation.open().await?;
        assert_eq!(saved(&database).await?, [1, 2]);
        database.close().await;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ops_07_a_backup_another_installation_sealed_is_refused_before_anything_is_replaced()
-> TestResult {
    each_database("ops_07_foreign_seal", async |db| {
        // A backup made by another installation, for example one whose database plants an
        // administrator, does not carry this installation's seal.
        let installation = Installation::new(db)?;
        let database = installation.open().await?;
        save(&database, 1).await?;
        database.close().await;
        let other = tempfile::tempdir()?;
        let elsewhere = BackupPlaces {
            keys: other_keys(other.path(), None)?,
            ..installation.places.clone()
        };
        let database = installation.open().await?;
        let foreign = database
            .create_backup(&elsewhere, Purpose::Regular, at(10))
            .await?;
        save(&database, 2).await?;
        database.close().await;
        let failure = installation
            .restore(&foreign.path, &[], at(20))
            .await
            .expect_err("the backup is another installation's");
        assert!(
            matches!(failure.error, StoreError::BackupNotSealed),
            "{}",
            failure.error
        );
        assert!(failure.before.is_none());
        let database = installation.open().await?;
        assert_eq!(saved(&database).await?, [1, 2]);
        database.close().await;
        Ok(())
    })
    .await
}

#[test]
fn a_backup_name_of_the_folder_reads_back() {
    let name = BackupName::new(Purpose::Regular, at(0), VERSION);
    assert_eq!(BackupName::parse(&name.to_string()), Some(name));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ops_07_two_backups_made_at_the_same_moment_take_two_names() -> TestResult {
    // The command line beside the server: both look for a free name at once (M9-F17).
    each_database("ops_07_two_at_once", async |db| {
        let installation = Installation::new(db)?;
        let database = installation.open().await?;
        let (first, second) = tokio::join!(
            database.create_backup(&installation.places, Purpose::Regular, at(10)),
            database.create_backup(&installation.places, Purpose::Regular, at(10)),
        );
        let mut moments = [first?.file.name.moment, second?.file.name.moment];
        moments.sort_unstable();
        assert_eq!(moments, [at(9), at(10)]);
        assert_eq!(backups_in(&installation.places.folder)?.len(), 2);
        database.close().await;
        Ok(())
    })
    .await
}

#[test]
fn ops_07_an_empty_file_is_a_name_being_taken_and_not_listed() -> TestResult {
    // A drive without links takes the name with an empty file until the copy is renamed onto it
    // (M9-F16).
    let folder = tempfile::tempdir()?;
    let name = BackupName::new(Purpose::Regular, at(0), VERSION);
    std::fs::write(folder.path().join(name.to_string()), b"")?;
    assert!(backups_in(folder.path())?.is_empty());
    Ok(())
}

#[test]
fn ops_07_a_backup_deletes_what_one_that_never_finished_left_behind() -> TestResult {
    // A program stopped while it copied a backup onto a drive without links leaves the empty
    // name and the copy beside it; the next backup deletes them once they have stood an hour.
    let folder = tempfile::tempdir()?;
    let named = |seconds| BackupName::new(Purpose::Regular, at(seconds), VERSION).to_string();
    let long_ago = std::time::SystemTime::now() - 2 * LEFTOVER_AFTER;
    let file = |name: &str, contents: &[u8], old: bool| -> Result<(), Failure> {
        let path = folder.path().join(name);
        std::fs::write(&path, contents)?;
        if old {
            std::fs::File::options()
                .write(true)
                .open(&path)?
                .set_modified(long_ago)?;
        }
        Ok(())
    };
    // Left behind: an old copy, and an old empty name with no copy.
    file(&format!("{}{PARTIAL}", named(1)), b"half", true)?;
    file(&named(2), b"", true)?;
    // Still at work: an old empty name whose copy is written now, and a new empty name.
    file(&named(3), b"", true)?;
    file(&format!("{}{PARTIAL}", named(3)), b"half", false)?;
    file(&named(4), b"", false)?;
    // Not a backup's: an old finished backup, and an old file of another name.
    file(&named(5), b"a backup", true)?;
    file(&format!("notes.txt{PARTIAL}"), b"", true)?;

    remove_leftovers(folder.path());
    let mut left: Vec<String> = std::fs::read_dir(folder.path())?
        .map(|entry| Ok(entry?.file_name().to_string_lossy().into_owned()))
        .collect::<Result<_, std::io::Error>>()?;
    left.sort();
    let mut kept = vec![
        named(3),
        format!("{}{PARTIAL}", named(3)),
        named(4),
        named(5),
        format!("notes.txt{PARTIAL}"),
    ];
    kept.sort();
    assert_eq!(left, kept);
    Ok(())
}

#[path = "backup_copy.rs"]
mod copy;

#[path = "backup_replay.rs"]
mod replay;
