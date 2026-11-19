//! The start of a newer program on the database of an older one (OPS-06, ADR-0046 decision 5): the
//! record of the backup before the upgrade, the words of a start that stopped, and its exit code.
//! Which migrations are due, the backup and the putting back are proven in the store on both
//! databases (`bb-store`, `tests/upgrade.rs`).

use bb_core::backup::name::{BackupName, Purpose};
use bb_core::backup::run::BackupMaker;
use bb_store::backup::{VERSION, backups_in};
use bb_testing::{Failure, TestResult, each_database};
use jiff::Timestamp;

use super::*;
use crate::wiring::tests::settings;
use crate::wiring::{Serving, wire};

/// The words and the exit code of a start that stopped.
fn stop_of(error: &anyhow::Error) -> Result<String, Failure> {
    let stopped = error
        .downcast_ref::<UpgradeStopped>()
        .ok_or_else(|| format!("not a stop at the upgrade: {error:#}"))?;
    Ok(stopped.to_string())
}

#[tokio::test]
async fn ops_06_an_upgrade_makes_a_backup_before_the_first_migration() -> TestResult {
    each_database("ops_06_server_upgrade", async |db| {
        let folder = tempfile::tempdir()?;
        let data = DataFolder::prepare(&folder.path().join("data"))?;
        let settings = settings(data.root(), Some(db.url()))?;
        let older = crate::wiring::connect(&settings, &data).await?;
        older
            .migrate_up_to(bb_store::latest_version() - 1, Timestamp::now())
            .await?;
        older.open_keys(&data.keys()).await?;

        let wired = wire(&settings, &data, Serving::default()).await?;
        drop(wired);
        let found = backups_in(&data.backups())?;
        let [(name, _, _)] = found.as_slice() else {
            return Err(format!("one backup was expected: {found:?}").into());
        };
        assert_eq!(name.purpose, Purpose::BeforeUpgrade);
        // Like every backup it has its record, made by the upgrade.
        let runs = older.backup_runs(10).await?;
        assert!(
            runs.iter()
                .any(|run| run.maker == BackupMaker::Upgrade && &run.name == name),
            "{runs:?}"
        );
        assert_eq!(std::fs::read_to_string(data.version_file())?, VERSION);
        older.close().await;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ops_06_a_newer_database_is_refused_with_its_version() -> TestResult {
    let folder = tempfile::tempdir()?;
    let data = DataFolder::prepare(&folder.path().join("data"))?;
    let settings = settings(data.root(), None)?;
    drop(wire(&settings, &data, Serving::default()).await?);
    // A newer program started on the data and changed the database.
    let database = crate::wiring::connect(&settings, &data).await?;
    database.record_newer_migration(Timestamp::now()).await?;
    database.close().await;
    std::fs::write(data.version_file(), "9.9.9\n")?;

    let error = wire(&settings, &data, Serving::default())
        .await
        .err()
        .ok_or("the newer database was opened")?;
    let words = stop_of(&error)?;
    assert!(words.contains("version 9.9.9"), "{words}");
    assert!(words.contains(VERSION), "{words}");
    assert_eq!(
        std::fs::read_to_string(data.version_file())?,
        "9.9.9\n",
        "the version of the newer program stays written down"
    );
    Ok(())
}

/// What `version.txt` says after a start of `version` that ended well.
fn after(version: &str) -> Written {
    Written {
        version: Some(version.to_owned()),
        upgrading: None,
    }
}

/// A backup before an upgrade in the backups folder of `data`, made at `seconds`.
fn backup_of(data: &DataFolder, seconds: i64) -> Result<std::path::PathBuf, Failure> {
    let moment = Timestamp::from_second(seconds)?;
    Ok(data
        .backups()
        .join(BackupName::new(Purpose::BeforeUpgrade, moment, VERSION).to_string()))
}

#[test]
fn ops_06_a_failed_migration_restores_the_backup_and_stops_in_plain_words() -> TestResult {
    let folder = tempfile::tempdir()?;
    let data = DataFolder::prepare(&folder.path().join("data"))?;
    let backup = backup_of(&data, 0)?;
    let failed = |put_back| UpgradeError::MigrationFailed {
        error: StoreError::Corrupt("a migration"),
        backup: backup.clone(),
        put_back,
    };

    // What failed, where the backup is, and that the previous program works with it.
    let words = stop_of(&stopped(failed(Ok(())), &after("0.5.0"), &data))?;
    for part in [
        "a migration",
        &backup.display().to_string(),
        "The previous program, version 0.5.0, works with this database",
    ] {
        assert!(words.contains(part), "{part} is missing: {words}");
    }
    // Each value stands where its sentence says.
    assert!(
        words.starts_with(&format!("Bugaboo {VERSION} could not change")),
        "{words}"
    );
    assert!(
        words.contains(&format!("first, {}, and stopped", backup.display())),
        "{words}"
    );
    let words = stop_of(&stopped(failed(Ok(())), &Written::default(), &data))?;
    assert!(
        words.contains("The program you used before works with this database"),
        "{words}"
    );

    // When putting the backup back failed too, nobody is told to start on the database again.
    let words = stop_of(&stopped(
        failed(Err(StoreError::Corrupt("the copy"))),
        &after("0.5.0"),
        &data,
    ))?;
    assert!(words.contains("do not start Bugaboo"), "{words}");
    assert!(!words.contains("works with this database"), "{words}");

    // A backup that failed stops before anything changed.
    let words = stop_of(&stopped(
        UpgradeError::BackupFailed(StoreError::Corrupt("no room")),
        &after("0.5.0"),
        &data,
    ))?;
    assert!(words.contains("Nothing was changed"), "{words}");
    Ok(())
}

#[test]
fn ops_06_a_start_after_an_upgrade_that_did_not_finish_names_the_backup_before_it() -> TestResult {
    let folder = tempfile::tempdir()?;
    let data = DataFolder::prepare(&folder.path().join("data"))?;
    // An earlier start made the backup at 0, began to migrate and was cut off; this start made
    // its own backup at 10, of the half-changed database.
    let older = backup_of(&data, 0)?;
    let newer = backup_of(&data, 10)?;
    let written = Written {
        version: Some("0.5.0".to_owned()),
        upgrading: older
            .file_name()
            .map(|name| name.to_string_lossy().into_owned()),
    };
    let failed = |put_back| UpgradeError::MigrationFailed {
        error: StoreError::Corrupt("a migration"),
        backup: newer.clone(),
        put_back,
    };
    let errors = || {
        [
            failed(Ok(())),
            failed(Err(StoreError::Corrupt("the copy"))),
            UpgradeError::BackupFailed(StoreError::Corrupt("no room")),
        ]
    };
    std::fs::write(&older, b"a backup")?;
    for error in errors() {
        let words = stop_of(&stopped(error, &written, &data))?;
        assert!(words.contains("did not finish"), "{words}");
        assert!(words.contains("do not start Bugaboo on it"), "{words}");
        assert!(
            words.contains(&older.display().to_string()),
            "the backup before the first try is named: {words}"
        );
        assert!(!words.contains("works with this database"), "{words}");
    }
    // A backup the rotation took away is not named: the words are the usual ones.
    std::fs::remove_file(&older)?;
    for error in errors() {
        let words = stop_of(&stopped(error, &written, &data))?;
        assert!(!words.contains("did not finish"), "{words}");
        assert!(!words.contains(&older.display().to_string()), "{words}");
    }
    let words = stop_of(&stopped(failed(Ok(())), &written, &data))?;
    assert!(
        words.contains("The previous program, version 0.5.0, works with this database"),
        "{words}"
    );
    Ok(())
}

#[tokio::test]
async fn ops_06_version_txt_marks_an_upgrade_until_a_start_ends_well() -> TestResult {
    let folder = tempfile::tempdir()?;
    let data = DataFolder::prepare(&folder.path().join("data"))?;
    let settings = settings(data.root(), None)?;
    // A start cut off part way through an upgrade left its mark.
    let first_backup = backup_of(&data, 0)?;
    let first_name = first_backup
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .ok_or("a name")?;
    let marked = format!("0.5.0\nupgrading {first_name}");
    std::fs::write(data.version_file(), &marked)?;
    let written = Written::read(&data);
    assert_eq!(written.version.as_deref(), Some("0.5.0"));
    assert_eq!(written.upgrading.as_deref(), Some(first_name.as_str()));
    // The mark of the first try stays while later tries go on.
    let later = CreatedBackup {
        file: bb_core::backup::run::CreatedFile {
            name: BackupName::new(Purpose::BeforeUpgrade, Timestamp::from_second(10)?, VERSION),
            size: 0,
            checksum: bb_core::security_log::Digest::new([0; 32]),
        },
        path: backup_of(&data, 10)?,
        log_head: None,
    };
    begin_upgrade(&written, &later, &data)?;
    assert_eq!(std::fs::read_to_string(data.version_file())?, marked);
    // A first try writes the mark with its own backup and keeps the version before it.
    begin_upgrade(&after("0.5.0"), &later, &data)?;
    assert_eq!(
        Written::read(&data),
        Written {
            version: Some("0.5.0".to_owned()),
            upgrading: Some(later.file.name.to_string()),
        }
    );
    // A start that ends well clears it.
    drop(wire(&settings, &data, Serving::default()).await?);
    assert_eq!(std::fs::read_to_string(data.version_file())?, VERSION);
    Ok(())
}

#[tokio::test]
async fn ops_06_a_database_with_a_migration_this_program_does_not_know_stops_the_start()
-> TestResult {
    let folder = tempfile::tempdir()?;
    let data = DataFolder::prepare(&folder.path().join("data"))?;
    let settings = settings(data.root(), None)?;
    drop(wire(&settings, &data, Serving::default()).await?);
    // Another program recorded a migration of the same number in another form.
    let database = crate::wiring::connect(&settings, &data).await?;
    database.change_recorded_migration(1).await?;
    database.close().await;

    let error = wire(&settings, &data, Serving::default())
        .await
        .err()
        .ok_or("the database was opened")?;
    let words = stop_of(&error)?;
    assert!(words.contains("number 1"), "{words}");
    assert!(words.contains("Nothing was changed"), "{words}");
    assert_eq!(
        crate::cli::exit_code_of(&error),
        std::process::ExitCode::from(UPGRADE_STOPPED)
    );
    Ok(())
}

#[test]
fn ops_06_a_newer_database_is_named_by_the_version_written_only_when_it_is_another() -> TestResult {
    let newer = || StoreError::SchemaTooNew { found: 2, known: 1 };
    // The version written down is this program's own, so the newer program wrote none: no
    // version is named.
    let words = stop_of(&refused(newer(), &after(VERSION)))?;
    assert!(words.contains("Start the newer version"), "{words}");
    let words = stop_of(&refused(newer(), &Written::default()))?;
    assert!(words.contains("Start the newer version"), "{words}");
    let words = stop_of(&refused(newer(), &after("9.9.9")))?;
    assert!(words.contains("Start version 9.9.9 or newer"), "{words}");
    Ok(())
}

#[test]
fn ops_06_a_start_that_stopped_at_the_upgrade_has_an_exit_code_of_its_own() {
    let stop: anyhow::Error = UpgradeStopped("stopped".to_owned()).into();
    assert_eq!(
        crate::cli::exit_code_of(&stop),
        std::process::ExitCode::from(UPGRADE_STOPPED)
    );
    assert_eq!(
        crate::cli::exit_code_of(&anyhow::anyhow!("another failure")),
        std::process::ExitCode::FAILURE
    );
}
