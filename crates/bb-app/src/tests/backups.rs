//! Backups as the use cases see them (OPS-07, ADR-0046): the list, *Create a backup now* with its
//! event and its line in the security log, and the schedule with the clock moved.

use bb_core::backup::name::{BackupName, Purpose};
use bb_core::backup::run::{BackupEvent, BackupMaker};
use bb_core::settings::{BACKUPS_DAILY, SettingValue};
use bb_core::sign_in::SecurityAction;
use jiff::Timestamp;

use super::fake_backups::MemoryBackups;
use super::scenarios::{Added, measured, token};
use super::world::{Outcome, World};
use crate::backups::{BackupFile, CreateBackup, ListBackupsQuery, create_backup, list_backups};
use crate::settings::{ChangeSetting, SettingsScope, change_setting};

/// The seconds of a day.
const DAY: i64 = 86_400;

pub(crate) async fn create_backup_scenario(world: &World) -> Added {
    measured(
        &world.store,
        create_backup(
            &world.ctx(),
            CreateBackup {
                session_token: token(world),
            },
        ),
    )
    .await
}

fn names(backups: &MemoryBackups) -> Vec<BackupName> {
    backups
        .files
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .iter()
        .map(|file| file.name.clone())
        .collect()
}

#[tokio::test]
async fn ops_07_a_backup_now_is_recorded_with_its_line_in_the_security_log() -> Outcome {
    let world = World::new().await?;
    let created = create_backup(
        &world.ctx(),
        CreateBackup {
            session_token: token(&world),
        },
    )
    .await?;
    let list = list_backups(
        &world.ctx(),
        ListBackupsQuery {
            session_token: token(&world),
        },
    )
    .await?;
    let list = list.into_value();
    assert_eq!(list.files.len(), 1);
    assert_eq!(list.files[0].name, created.into_value().name);
    assert_eq!(list.runs.len(), 1);
    assert_eq!(list.runs[0].event, BackupEvent::Made);
    assert_eq!(list.runs[0].maker, BackupMaker::Person);
    assert_eq!(list.runs[0].person, Some(world.admin.person.id));
    let logged = world
        .store
        .lock()
        .log
        .iter()
        .any(|entry| entry.action == SecurityAction::BackupMade);
    assert!(logged, "the backup has its line in the security log");
    Ok(())
}

#[tokio::test]
async fn ops_07_the_schedule_keeps_seven_daily_and_four_weekly_backups() -> Outcome {
    let world = World::new().await?;
    // Switched off, the schedule creates nothing.
    change_setting(
        &world.ctx(),
        ChangeSetting {
            version: None,
            session_token: token(&world),
            scope: SettingsScope::Organization,
            key: BACKUPS_DAILY.definition().key().to_owned(),
            value: SettingValue::Bool(false),
        },
    )
    .await?;
    assert!(!world.ctx().backup_round().await?.created);
    change_setting(
        &world.ctx(),
        ChangeSetting {
            version: None,
            session_token: token(&world),
            scope: SettingsScope::Organization,
            key: BACKUPS_DAILY.definition().key().to_owned(),
            value: SettingValue::Bool(true),
        },
    )
    .await?;
    for _ in 0..40 {
        world.ctx().backup_round().await?;
        world.clock.advance(DAY);
    }
    let kept = names(&world.backups);
    // The newest of each of seven days, and of each week before them up to four weeks.
    assert!((8..=10).contains(&kept.len()), "{kept:?}");
    let newest = kept
        .iter()
        .map(|name| name.moment)
        .max()
        .ok_or("none kept")?;
    let days: Vec<i64> = kept
        .iter()
        .map(|name| (newest.as_second() - name.moment.as_second()) / DAY)
        .collect();
    for day in 0..7 {
        assert!(days.contains(&day), "day {day} is kept: {days:?}");
    }
    assert!(days.iter().all(|day| *day < 28), "{days:?}");
    Ok(())
}

#[tokio::test]
async fn ops_07_the_backup_before_a_restore_stays_for_the_keep_period_of_the_security_log()
-> Outcome {
    let world = World::new().await?;
    let start = Timestamp::from_second(world.ctx().now().as_second())?;
    world
        .backups
        .files
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(BackupFile {
            name: BackupName::new(Purpose::BeforeRestore, start, "0.6.0"),
            size: 1,
        });
    let before_restore = |world: &World| {
        names(&world.backups)
            .iter()
            .any(|name| name.purpose == Purpose::BeforeRestore)
    };
    // Far past the seven days and four weeks of the rotation, inside the 365 days of the log.
    world.clock.advance(100 * DAY);
    world.ctx().backup_round().await?;
    assert!(before_restore(&world), "it stays inside the keep period");
    world.clock.advance(300 * DAY);
    world.ctx().backup_round().await?;
    assert!(!before_restore(&world), "it goes after the keep period");
    Ok(())
}
