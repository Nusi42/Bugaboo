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
struct Installation {
    folder: tempfile::TempDir,
    place: Place,
    places: BackupPlaces,
    seal: LogSeal,
}

impl Installation {
    fn new(db: &TestDatabase) -> Result<Self, Failure> {
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
        let keys = KeyRing::open_or_create(&folder.path().join("keys"))?;
        let seal = LogSeal::new(&keys, folder.path().join("logs").join("security-head.log"));
        let files = folder.path().join("files");
        let places = BackupPlaces {
            folder: folder.path().join("backups"),
            files,
            replay_list: folder.path().join("privacy").join("replay.list"),
            file_paths: Arc::new(|hash: &Digest| {
                let name = hex::encode(hash.as_bytes());
                vec![format!("content/{}/{name}", &name[..2])]
            }),
        };
        Ok(Self {
            folder,
            place,
            places,
            seal,
        })
    }

    fn keys(&self) -> Result<Arc<KeyRing>, StoreError> {
        Ok(Arc::new(KeyRing::open_or_create(
            &self.folder.path().join("keys"),
        )?))
    }

    async fn open(&self) -> Result<Database, StoreError> {
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
        let result = restore(
            RestoreInput {
                archive,
                target: &target,
                places: &self.places,
                keys,
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

    fn storage(&self, database: &Database) -> Result<AuthStorage, StoreError> {
        Ok(AuthStorage::new(
            database.clone(),
            self.keys()?,
            self.seal.clone(),
        ))
    }

    async fn restore(
        &self,
        archive: &Path,
        more: &[PathBuf],
        now: Timestamp,
    ) -> Result<RestoreReport, RestoreFailure> {
        let keys = self.keys().map_err(|error| RestoreFailure {
            error,
            before: None,
        })?;
        self.restore_with(archive, &keys, more, now).await
    }
}

/// Saves the change numbered `number`: an event whose reason is the number.
async fn save(database: &Database, number: i64) -> Result<(), StoreError> {
    let mut saved = event(&mut bb_app::context::SystemIds, number);
    saved.reason = Some(number.to_string());
    database.record_change(&saved, async |_| Ok(())).await
}

/// The numbers of the saved changes, in order.
async fn saved(database: &Database) -> Result<Vec<i64>, StoreError> {
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
fn signed_in(seconds: i64) -> bb_app::auth::SecurityEvent {
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
        let storage = installation.storage(&database)?;
        let name = bb_core::technical::TechnicalName::new("mail.password")?;
        database
            .put_secret(&*installation.keys()?, &name, b"hunter2-mail", at(1))
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
    async fn open_unchecked(&self) -> Result<Database, StoreError> {
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
async fn ops_07_a_restore_replays_removals_switch_offs_and_taken_back_keys_made_after_the_backup()
-> TestResult {
    each_database("ops_07_replay", async |db| {
        let installation = Installation::new(db)?;
        let database = installation.open().await?;
        let storage = installation.storage(&database)?;
        let mut source = ids();
        let (ada, key) = a_key(&storage, &mut source).await?;
        let moment: Timestamp = "2026-11-10T10:00:00Z".parse()?;
        let backup = database
            .create_backup(&installation.places, Purpose::Regular, moment)
            .await?;
        database.close().await;
        // The lines a later change wrote (the writer comes with *Remove a person*): one of each
        // kind after the backup, and one before it, which the backup holds already.
        let later = |minutes: i64| moment.checked_add(SignedDuration::from_mins(minutes));
        let lines = [
            ReplayLine {
                moment: moment.checked_sub(SignedDuration::from_mins(5))?,
                kind: ReplayKind::AccessKey,
                id: AccessKeyId::generate(&mut source).to_string(),
                choice: None,
            },
            ReplayLine {
                moment: later(1)?,
                kind: ReplayKind::AccessKey,
                id: key.to_string(),
                choice: None,
            },
            ReplayLine {
                moment: later(2)?,
                kind: ReplayKind::SwitchOff,
                id: ada.to_string(),
                choice: None,
            },
            ReplayLine {
                moment: later(3)?,
                kind: ReplayKind::Removal,
                id: ada.to_string(),
                choice: Some("keep".to_owned()),
            },
        ];
        // A line whose ID is not a key's: it is counted as unreadable and stops nothing.
        let unreadable = format!(
            "{} access-key not-a-key
",
            later(4)?
        );
        std::fs::write(
            &installation.places.replay_list,
            bb_core::backup::replay::write(&lines) + &unreadable,
        )?;
        let report = installation
            .restore(&backup.path, &[], later(10)?)
            .await
            .map_err(failed)?;
        assert_eq!(report.unreadable_lines, 1);
        assert_eq!(report.replayed, [lines[1].clone()]);
        // A removal and a switch-off are named in the report until the program can run them.
        assert_eq!(report.not_replayed, [lines[2].clone(), lines[3].clone()]);
        let restored = installation.open().await?;
        assert!(!has_key(&restored, key).await?, "the key stays taken back");
        restored.close().await;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ops_07_every_restore_signs_everybody_out() -> TestResult {
    each_database("ops_07_sign_out", async |db| {
        let installation = Installation::new(db)?;
        let database = installation.open().await?;
        let storage = installation.storage(&database)?;
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
        let storage = installation.storage(&database)?;
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

#[tokio::test]
async fn ops_07_a_wrong_master_key_stops_the_restore_before_anything_is_replaced() -> TestResult {
    each_database("ops_07_wrong_key", async |db| {
        let installation = Installation::new(db)?;
        let database = installation.open().await?;
        let name = bb_core::technical::TechnicalName::new("mail.password")?;
        database
            .put_secret(&*installation.keys()?, &name, b"hunter2-mail", at(1))
            .await?;
        save(&database, 1).await?;
        let backup = database
            .create_backup(&installation.places, Purpose::Regular, at(10))
            .await?;
        save(&database, 2).await?;
        database.close().await;
        // Another installation's keys.
        let other = tempfile::tempdir()?;
        let keys = Arc::new(KeyRing::open_or_create(other.path())?);
        let failure = installation
            .restore_with(&backup.path, &keys, &[], at(20))
            .await
            .expect_err("the keys do not open the secrets");
        assert!(matches!(failure.error, StoreError::KeysDoNotOpen));
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

#[path = "backup_copy.rs"]
mod copy;
