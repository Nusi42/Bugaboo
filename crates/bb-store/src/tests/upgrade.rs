//! A newer program started on an older database, on both databases (OPS-06, ADR-0046 decision 5):
//! the backup before the first migration, none when nothing is due, the backup put back when a
//! migration fails, and what is refused before anything changes.

use bb_core::backup::name::{BackupName, Purpose};
use bb_testing::{Failure, TestResult, each_database};

use super::super::tests::{CHANGES_NOTHING, CHANGES_SAVED, FAILS, program_with};
use super::*;
use bb_app::auth::AuthStore as _;

use crate::SchemaState;
use crate::backup::backups_in;
use crate::backup::tests::{Installation, save, saved, signed_in};
use crate::chain::Verification;
use crate::test_support::at;

/// The names of the backups in the folder of the installation.
fn backups(installation: &Installation) -> Result<Vec<BackupName>, Failure> {
    Ok(backups_in(&installation.places.folder)?
        .into_iter()
        .map(|(name, _, _)| name)
        .collect())
}

/// A start that writes nothing down before the first migration.
fn unmarked() -> impl FnOnce(&CreatedBackup) -> std::io::Result<()> + Send {
    |_| Ok(())
}

async fn applied_versions(database: &Database) -> Result<Vec<i64>, StoreError> {
    Ok(database
        .applied_migrations()
        .await?
        .iter()
        .map(|record| record.version)
        .collect())
}

#[tokio::test]
async fn ops_06_an_upgrade_makes_a_backup_before_the_first_migration() -> TestResult {
    each_database("ops_06_upgrade_backup", async |db| {
        let installation = Installation::new(db)?;
        let database = installation.open_unchecked().await?;
        let older = crate::latest_version() - 1;
        database.migrate_up_to(older, at(0)).await?;
        save(&database, 1).await?;
        let Upgrade::Upgraded { backup, migrations } = database
            .upgrade(&installation.places, at(10), unmarked())
            .await
            .map_err(|error| format!("{error:?}"))?
        else {
            return Err("the older database was not upgraded".into());
        };
        assert_eq!(migrations, [crate::latest_version()]);
        assert_eq!(backup.file.name.purpose, Purpose::BeforeUpgrade);
        assert_eq!(
            backups(&installation)?,
            std::slice::from_ref(&backup.file.name)
        );
        assert_eq!(database.schema_state().await?, SchemaState::Current);
        assert_eq!(saved(&database).await?, [1]);
        database.close().await;
        // The backup holds the database as it was before: putting it back runs the migration again.
        let report = installation
            .restore(&backup.path, &[], at(20))
            .await
            .map_err(|failure| failure.error.to_string())?;
        assert_eq!(report.migrations_run, [crate::latest_version()]);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ops_06_no_backup_is_made_when_no_migration_is_due() -> TestResult {
    each_database("ops_06_no_backup", async |db| {
        let installation = Installation::new(db)?;
        let database = installation.open_unchecked().await?;
        // A new database gets every migration and needs no backup.
        let first = database
            .upgrade(&installation.places, at(0), unmarked())
            .await
            .map_err(|error| format!("{error:?}"))?;
        assert!(
            matches!(&first, Upgrade::New { migrations } if migrations.len() == ALL.len()),
            "{first:?}"
        );
        // A database that has every migration gets nothing.
        let again = database
            .upgrade(&installation.places, at(1), unmarked())
            .await
            .map_err(|error| format!("{error:?}"))?;
        assert!(matches!(again, Upgrade::Current), "{again:?}");
        assert!(backups(&installation)?.is_empty());
        database.close().await;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ops_06_a_failed_migration_restores_the_backup_and_stops_in_plain_words() -> TestResult {
    each_database("ops_06_failed_migration", async |db| {
        let installation = Installation::new(db)?;
        let database = installation.open().await?;
        for number in 1..=3 {
            save(&database, number).await?;
        }
        installation
            .storage(&database)
            .append_security_log(&signed_in(1))
            .await?;
        let before = applied_versions(&database).await?;
        // The first migration of the upgrade commits its change; the second fails.
        let program = program_with(&[CHANGES_SAVED, FAILS]);
        let Err(UpgradeError::MigrationFailed {
            backup, put_back, ..
        }) = database
            .upgrade_with(&program, &installation.places, at(10), unmarked())
            .await
        else {
            return Err("the failing migration did not stop the upgrade".into());
        };
        put_back.map_err(|error| error.to_string())?;
        assert_eq!(
            BackupName::parse(&backup.file_name().unwrap_or_default().to_string_lossy())
                .map(|name| name.purpose),
            Some(Purpose::BeforeUpgrade)
        );
        database.close().await;
        // The database reads as before: the change of the first migration is gone, the record of
        // migrations is the one it had, and the security log verifies with no new anchor.
        let restored = installation.open_unchecked().await?;
        assert_eq!(saved(&restored).await?, [1, 2, 3]);
        assert_eq!(applied_versions(&restored).await?, before);
        let log = restored.verify_security_log(&installation.seal).await?;
        assert!(
            matches!(log, Verification::Valid { anchored: true, .. }),
            "{log:?}"
        );
        restored.close().await;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ops_06_a_backup_that_fails_stops_the_start_before_any_migration() -> TestResult {
    each_database("ops_06_backup_fails", async |db| {
        let installation = Installation::new(db)?;
        let database = installation.open_unchecked().await?;
        database
            .migrate_up_to(crate::latest_version() - 1, at(0))
            .await?;
        let before = applied_versions(&database).await?;
        // A file where the folder of the backups should be: no backup can be made.
        let mut places = installation.places.clone();
        places.folder = installation.places.folder.join("not-a-folder");
        std::fs::write(&places.folder, "a file")?;
        let result = database.upgrade(&places, at(10), unmarked()).await;
        assert!(
            matches!(result, Err(UpgradeError::BackupFailed(_))),
            "{result:?}"
        );
        assert_eq!(applied_versions(&database).await?, before, "nothing ran");
        database.close().await;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ops_06_an_upgrade_is_written_down_after_the_backup_and_before_the_first_migration()
-> TestResult {
    each_database("ops_06_upgrade_written_down", async |db| {
        let installation = Installation::new(db)?;
        let database = installation.open_unchecked().await?;
        database
            .migrate_up_to(crate::latest_version() - 1, at(0))
            .await?;
        let before = applied_versions(&database).await?;
        // What cannot be written down stops the start before any migration.
        let mut first = None;
        let result = database
            .upgrade(&installation.places, at(10), |backup| {
                first = Some(backup.path.clone());
                Err(std::io::ErrorKind::PermissionDenied.into())
            })
            .await;
        assert!(
            matches!(result, Err(UpgradeError::BackupFailed(_))),
            "{result:?}"
        );
        assert!(
            first.is_some_and(|path| path.is_file()),
            "the backup came first"
        );
        assert_eq!(applied_versions(&database).await?, before, "nothing ran");
        // Written down once, with the backup that is there, and then the migrations run.
        let mut written = Vec::new();
        let upgraded = database
            .upgrade(&installation.places, at(20), |backup| {
                written.push(backup.path.clone());
                Ok(())
            })
            .await
            .map_err(|error| format!("{error:?}"))?;
        let Upgrade::Upgraded { backup, .. } = upgraded else {
            return Err("the older database was not upgraded".into());
        };
        assert_eq!(written, [backup.path]);
        database.close().await;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ops_06_a_newer_database_is_refused_with_its_version() -> TestResult {
    each_database("ops_06_newer_database", async |db| {
        let installation = Installation::new(db)?;
        let database = installation.open().await?;
        database.record_newer_migration(at(1)).await?;
        let result = database
            .upgrade(&installation.places, at(10), unmarked())
            .await;
        assert!(
            matches!(
                result,
                Err(UpgradeError::NotStarted(StoreError::SchemaTooNew { found, .. }))
                    if found == crate::latest_version() + 1
            ),
            "{result:?}"
        );
        assert!(backups(&installation)?.is_empty());
        database.close().await;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ops_06_a_database_with_a_gap_in_its_migrations_is_migrated_and_one_with_an_unknown_migration_refused()
-> TestResult {
    each_database("ops_06_gap", async |db| {
        let installation = Installation::new(db)?;
        let database = installation.open().await?;
        // A newer migration came first (as one given out beside another may).
        let newer_first = program_with(&[CHANGES_NOTHING]);
        database
            .upgrade_with(&newer_first, &installation.places, at(1), unmarked())
            .await
            .map_err(|error| format!("{error:?}"))?;
        // Its newest is the program's newest, but one is missing: it is due, by the set.
        let program = program_with(&[CHANGES_SAVED, CHANGES_NOTHING]);
        let upgraded = database
            .upgrade_with(&program, &installation.places, at(2), unmarked())
            .await
            .map_err(|error| format!("{error:?}"))?;
        assert!(
            matches!(&upgraded, Upgrade::Upgraded { migrations, .. } if migrations == &[CHANGES_SAVED.version]),
            "{upgraded:?}"
        );
        // A program that lacks a migration the database has, older than its own newest, refuses it.
        let backups_before = backups(&installation)?.len();
        let result = database
            .upgrade_with(&newer_first, &installation.places, at(3), unmarked())
            .await;
        assert!(
            matches!(
                result,
                Err(UpgradeError::NotStarted(StoreError::MigrationMismatch { version }))
                    if version == CHANGES_SAVED.version
            ),
            "{result:?}"
        );
        assert_eq!(backups(&installation)?.len(), backups_before);
        database.close().await;
        Ok(())
    })
    .await
}
