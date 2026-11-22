//! The commands of the exit demo of M9 (docs/milestones/M9/README.md, steps 4 and 8): `bugaboo
//! backup` made while people work and `bugaboo restore` with the server stopped, and the start of
//! this program on the data of an older one. The screens of step 4 are `web/app/e2e/m9-exit-demo.spec.ts`;
//! the upgrade from the program of the previous milestone is `cargo xtask upgrade-test` (TEST-UPGRADE).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use bb_core::backup::name::Purpose;
use bb_core::sign_in::SecurityAction;
use bb_core::technical::TechnicalName;
use bb_store::backup::{VERSION, backups_in};
use bb_store::{Database, LogSeal, NewSecurityLogEntry};
use bb_testing::{Failure, TestResult, each_database};
use jiff::Timestamp;

use super::super::secrets::tests::installation;
use super::*;
use crate::wiring::tests::settings;
use crate::wiring::{Serving, open_store, wire};

/// One thing a person does that the store keeps: a line of the security log, which a backup must
/// hold whole or not at all.
async fn work(database: &Database, seal: &LogSeal) -> Result<(), StoreError> {
    database
        .append_security_log(
            seal,
            &NewSecurityLogEntry {
                time: Timestamp::now(),
                person: None,
                action: TechnicalName::new(SecurityAction::SignInFailed.code())
                    .map_err(|_| StoreError::Corrupt("an action"))?,
                subject: None,
                network: None,
            },
        )
        .await?;
    Ok(())
}

/// More lines of the security log than a test writes: they are read in one go.
const MOST_LINES: u64 = 1_000_000;

/// How many lines the security log holds.
async fn lines(database: &Database) -> Result<u64, Failure> {
    Ok(database.security_log_entries(0, MOST_LINES).await?.len() as u64)
}

#[tokio::test]
async fn m9_exit_demo_4_a_backup_made_while_people_work_is_restored_with_everything_before_it()
-> TestResult {
    let folder = tempfile::tempdir()?;
    let (store, data) = installation(folder.path()).await?;
    let opened = {
        let settings = settings(data.root(), None)?;
        open_store(&settings, &data).await?
    };
    // People work while the command makes the backup.
    let stop = Arc::new(AtomicBool::new(false));
    let done = Arc::new(AtomicU64::new(0));
    let people = {
        let (database, seal) = (opened.database.clone(), opened.seal.clone());
        let (stop, done) = (Arc::clone(&stop), Arc::clone(&done));
        tokio::spawn(async move {
            while !stop.load(Ordering::SeqCst) {
                work(&database, &seal).await?;
                done.fetch_add(1, Ordering::SeqCst);
            }
            Ok::<_, StoreError>(())
        })
    };
    let before = lines(&opened.database).await?;
    backup(&BackupArgs {
        store: store.clone(),
        to: None,
        command: None,
    })
    .await?;
    stop.store(true, Ordering::SeqCst);
    people.await??;
    let worked = done.load(Ordering::SeqCst);
    assert!(worked > 0, "nobody worked while the backup was made");
    let after = lines(&opened.database).await?;
    opened.database.close().await;
    // The server is stopped; the backup is restored from the command line.
    let found = backups_in(&data.backups())?;
    let made = found
        .iter()
        .find(|(name, _, _)| name.purpose == Purpose::Regular)
        .ok_or("the backup of the command is not there")?;
    restore(&RestoreArgs {
        store: store.clone(),
        file: made.1.clone(),
        replay_list: None,
    })
    .await?;
    // Everything kept before the backup is in it, as one consistent state; the log verifies, and
    // the data of before the restore is kept in a backup of its own.
    let restored = open_store(&settings(data.root(), None)?, &data).await?;
    let held = lines(&restored.database).await?;
    assert!(
        held >= before && held <= after + 1,
        "{before} {held} {after}"
    );
    restored.database.close().await;
    super::super::secrets::verify_security_log(&store).await?;
    assert!(
        backups_in(&data.backups())?
            .iter()
            .any(|(name, _, _)| name.purpose == Purpose::BeforeRestore)
    );
    Ok(())
}

#[tokio::test]
async fn m9_exit_demo_8_an_older_installation_is_upgraded_without_losing_anything() -> TestResult {
    each_database("m9_exit_demo_8_upgrade", async |db| {
        let folder = tempfile::tempdir()?;
        let data = DataFolder::prepare(&folder.path().join("data"))?;
        let settings = settings(data.root(), Some(db.url()))?;
        // The data of the version before: its migrations, its keys and what people did.
        let older = connect(&settings, &data).await?;
        older
            .migrate_up_to(bb_store::latest_version() - 1, Timestamp::now())
            .await?;
        let keys = Arc::new(older.open_keys(&data.keys()).await?);
        let seal = LogSeal::new(&keys, data.security_log_anchor());
        for _ in 0..3 {
            work(&older, &seal).await?;
        }
        let kept = lines(&older).await?;
        // This program starts on it: a backup first, then every migration.
        drop(wire(&settings, &data, Serving::default()).await?);
        let before_upgrade = backups_in(&data.backups())?;
        assert!(
            before_upgrade
                .iter()
                .any(|(name, _, _)| name.purpose == Purpose::BeforeUpgrade),
            "{before_upgrade:?}"
        );
        assert_eq!(std::fs::read_to_string(data.version_file())?, VERSION);
        // Nothing was lost: every line is there, the backup's own line after them.
        assert!(lines(&older).await? > kept);
        assert!(matches!(
            older.verify_security_log(&seal).await?,
            bb_store::Verification::Valid { .. }
        ));
        older.close().await;
        Ok(())
    })
    .await
}
