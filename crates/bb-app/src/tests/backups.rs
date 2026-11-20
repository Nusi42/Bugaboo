//! Backups as the use cases see them (OPS-07, ADR-0046): the list, *Create a backup now* with its
//! event and its line in the security log, and the schedule with the clock moved.

use bb_core::backup::name::{BackupName, Purpose};
use bb_core::backup::run::{BackupEvent, BackupMaker};
use bb_core::error_code::ErrorCode;
use bb_core::settings::{BACKUPS_DAILY, SettingValue};
use bb_core::sign_in::SecurityAction;
use jiff::Timestamp;

use super::fake_backups::MemoryBackups;
use super::scenarios::{Added, measured, token};
use super::world::{Outcome, World};
use crate::auth::AppError;
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
    // The person who pressed the button is named once, the way *Who sees people* shows them.
    assert_eq!(list.people.len(), 1);
    assert_eq!(list.people[0].0, world.admin.person.id);
    assert!(matches!(
        list.people[0].1,
        bb_policy::PersonRef::Name { .. }
    ));
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
async fn ops_07_one_backup_is_made_at_a_time() -> Outcome {
    let world = World::new().await?;
    let ctx = world.ctx();
    let ask = || {
        create_backup(
            &ctx,
            CreateBackup {
                session_token: token(&world),
            },
        )
    };
    // While a backup is being made, another is turned away with its code and nothing is made.
    world
        .services
        .backup_running
        .store(true, std::sync::atomic::Ordering::SeqCst);
    assert!(matches!(
        ask().await,
        Err(AppError::Refused(ErrorCode::BackupRunning))
    ));
    assert!(world.store.lock().state.backup_runs.is_empty());
    world
        .services
        .backup_running
        .store(false, std::sync::atomic::Ordering::SeqCst);
    // Once it is done, the next one goes ahead, and lets the mark go again after it.
    ask().await?;
    ask().await?;
    assert_eq!(world.store.lock().state.backup_runs.len(), 2);
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
    // Every round, and only the round, first removes what a backup that never finished left.
    let cleaned = world
        .backups
        .cleaned
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    assert_eq!(cleaned, [None]);
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

#[tokio::test]
async fn ops_07_a_rotation_that_deletes_many_backups_queues_no_backup() -> Outcome {
    use crate::jobs::JobKind;
    use crate::keeping::KEEP_STEP_ROWS;

    let world = World::new().await?;
    let start = world.ctx().now().as_second();
    // More old backups than a step of the other purges may delete, each on a day of its own.
    let many = KEEP_STEP_ROWS + 50;
    {
        let mut files = world
            .backups
            .files
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for day in 0..many {
            files.push(BackupFile {
                name: BackupName::new(
                    Purpose::Regular,
                    Timestamp::from_second(start - i64::try_from(day)? * DAY - DAY)?,
                    "0.6.0",
                ),
                size: 1,
            });
        }
    }
    let jobs = world.store.lock().state.jobs.len();
    let step = world.ctx().keep_step(JobKind::BackupsDaily).await?;
    // All it no longer keeps goes at once, and no further step is queued: one would make a backup.
    assert!(step.rows > KEEP_STEP_ROWS, "{step:?}");
    assert!(!step.more);
    assert_eq!(world.store.lock().state.jobs.len(), jobs);
    Ok(())
}

/// A second administrator and the session of that person, who looks at the backups the first one
/// created.
async fn another_viewer(world: &World) -> Result<String, AppError> {
    use bb_policy::StartingRole;

    use super::scenarios_searches::with_role;

    let ben = world.person("ben@example.test").await?;
    with_role(world, &ben, StartingRole::Administrator, None)
        .await
        .map_err(AppError::Internal)?;
    Ok(ben.session_token)
}

async fn backups_seen_by(
    world: &World,
    session_token: &str,
) -> Result<crate::backups::Backups, AppError> {
    Ok(list_backups(
        &world.ctx(),
        ListBackupsQuery {
            session_token: session_token.to_owned(),
        },
    )
    .await?
    .into_value())
}

#[tokio::test]
async fn priv_12_the_list_names_the_person_as_who_sees_people_says() -> Outcome {
    use bb_core::access::PeopleLevel;
    use bb_core::people_kinds::PeopleKind;
    use bb_policy::{PersonRef, StartingRole};

    use super::workflows_rules::starting_role;
    use crate::roles::{SetPeopleLevel, set_level};

    let world = World::new().await?;
    let ben = another_viewer(&world).await?;
    create_backup_scenario(&world).await?;
    // By name at first.
    let seen = backups_seen_by(&world, &ben).await?;
    assert_eq!(seen.people.len(), 1);
    assert!(matches!(seen.people[0].1, PersonRef::Name { .. }));
    // When the role of the viewer is not told who people are, the backup was created by *Someone*.
    let role = starting_role(&world, StartingRole::Administrator)
        .await
        .map_err(AppError::Internal)?;
    set_level(
        &world.ctx(),
        SetPeopleLevel {
            session_token: token(&world),
            role,
            project: None,
            kind: PeopleKind::Authors,
            level: Some(PeopleLevel::Hidden),
        },
    )
    .await?;
    let seen = backups_seen_by(&world, &ben).await?;
    assert_eq!(seen.people[0].1, PersonRef::Someone);
    Ok(())
}

#[tokio::test]
async fn priv_12_a_person_who_was_removed_is_a_former_user_in_the_list() -> Outcome {
    use bb_policy::PersonRef;

    use crate::people::{SwitchOff, switch_off};
    use crate::privacy::removal::{RemovePerson, remove_person};

    let world = World::new().await?;
    let ben = another_viewer(&world).await?;
    create_backup_scenario(&world).await?;
    let creator = world.admin.person.id;
    switch_off(
        &world.ctx(),
        SwitchOff {
            session_token: ben.clone(),
            person: creator,
        },
    )
    .await?;
    remove_person(
        &world.ctx(),
        RemovePerson {
            session_token: ben.clone(),
            person: creator,
            comments: None,
        },
    )
    .await?;
    let seen = backups_seen_by(&world, &ben).await?;
    assert!(
        matches!(seen.people[0].1, PersonRef::Former { .. }),
        "{:?}",
        seen.people
    );
    Ok(())
}
