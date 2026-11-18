//! Keeping and deleting (PRIV-05, M1-F21, M1-F43, M1-F61): the jobs remove what outlived its keep
//! period, and each is safe to run again (ADR-0017).

use std::sync::{Arc, Mutex, PoisonError};

use bb_core::job::JobState;
use bb_core::settings::SettingValue;
use bb_core::settings::keeping::{KEEPING_DEAD_JOB_DAYS, KEEPING_NOTIFICATION_DAYS};
use bb_core::time::SECONDS_PER_DAY;
use serde_json::json;

use super::notifications::{Change, Crew, Status};
use super::scenarios::{text, token};
use super::world::{Outcome, World};
use crate::jobs::{JobKind, JobStore as _};
use crate::server_log::ServerLogFile;
use crate::settings::{ChangeSetting, SettingsScope, change_setting};

/// A log file that keeps what it was asked.
#[derive(Clone, Default)]
struct FakeLogFile(Arc<Mutex<Vec<(u64, u32)>>>);

impl ServerLogFile for FakeLogFile {
    fn rotate(&self, max_bytes: u64, keep_files: u32) -> std::io::Result<bool> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((max_bytes, keep_files));
        Ok(true)
    }
}

/// The starting value of a keep period in days.
fn days_of(setting: &bb_core::settings::Setting<bb_core::settings::Integer>) -> i64 {
    setting.definition().default_integer().unwrap_or_default()
}

async fn run_jobs(world: &World) -> Result<(), String> {
    loop {
        if world.ctx().run_due_jobs().await.map_err(text)? == 0 {
            return Ok(());
        }
    }
}

#[tokio::test]
async fn priv_05_notifications_older_than_ninety_days_are_deleted_by_the_job_and_only_those()
-> Outcome {
    let mut crew = Crew::new().await?;
    let (ada, bob) = (token(&crew.world), crew.bob.session_token.clone());
    crew.change(&bob, Change::Status(Status::Done)).await?;
    assert_eq!(crew.heard(&ada).await?.len(), 1);
    let world = &crew.world;
    // A day before the 90 days are over, the job leaves it alone.
    world
        .clock
        .advance((days_of(&KEEPING_NOTIFICATION_DAYS) - 1) * SECONDS_PER_DAY);
    world.ctx().start_daily_jobs().await?;
    run_jobs(world).await?;
    assert_eq!(world.store.lock().state.notifications.len(), 1);
    // A day later it goes, and a second round has nothing more to delete.
    world.clock.advance(2 * SECONDS_PER_DAY);
    world.ctx().start_daily_jobs().await?;
    run_jobs(world).await?;
    assert_eq!(world.store.lock().state.notifications.len(), 0);
    assert_eq!(world.ctx().purge_old_notifications().await?, 0);
    Ok(())
}

#[tokio::test]
async fn priv_05_a_dead_job_loses_its_payload_after_its_period() -> Outcome {
    let world = World::new().await?;
    let id = bb_core::id::JobId::generate(&mut bb_core::id::SequentialIds::default())?;
    let now = world.ctx().now();
    world
        .store
        .queue_job(
            id,
            &JobKind::MailSend.name()?,
            &json!({"what": "changes"}),
            now,
            now,
        )
        .await
        .map_err(|error| format!("{error:?}"))?;
    world.store.lock().state.jobs[0].state = JobState::Dead;
    assert_eq!(world.ctx().purge_old_dead_jobs().await?, 0);
    world
        .clock
        .advance((days_of(&KEEPING_DEAD_JOB_DAYS) + 1) * SECONDS_PER_DAY);
    assert_eq!(world.ctx().purge_old_dead_jobs().await?, 1);
    assert!(
        !world.store.lock().state.jobs.iter().any(|job| job.id == id),
        "the job went with its payload"
    );
    assert_eq!(
        world.ctx().purge_old_dead_jobs().await?,
        0,
        "safe to run again"
    );
    Ok(())
}

#[tokio::test]
async fn m1_f21_the_security_log_keeps_the_days_the_setting_says() -> Outcome {
    let world = World::new().await?;
    let entries = world.store.log_count();
    assert!(entries > 0, "setting the world up wrote entries");
    // The year of the setting has not passed: nothing goes.
    assert_eq!(world.ctx().prune_old_security_log().await?, 0);
    world.clock.advance(400 * SECONDS_PER_DAY);
    // A new entry stays whatever the age of the others.
    world
        .ctx()
        .note(bb_core::sign_in::SecurityAction::SignInFailed, None, None)
        .await;
    let gone = world.ctx().prune_old_security_log().await?;
    assert_eq!(
        world.store.log_count(),
        entries + 1 - usize::try_from(gone)?
    );
    assert!(gone > 0);
    assert_eq!(world.ctx().prune_old_security_log().await?, 0);
    Ok(())
}

#[tokio::test]
async fn m1_f21_a_longer_keep_period_of_the_security_log_keeps_more() -> Outcome {
    let world = World::new().await?;
    change_setting(
        &world.ctx(),
        ChangeSetting {
            version: None,
            session_token: token(&world),
            scope: SettingsScope::Organization,
            key: "security.log-keep-days".to_owned(),
            value: SettingValue::Integer(1000),
        },
    )
    .await?;
    world.clock.advance(400 * SECONDS_PER_DAY);
    world
        .ctx()
        .note(bb_core::sign_in::SecurityAction::SignInFailed, None, None)
        .await;
    assert_eq!(world.ctx().prune_old_security_log().await?, 0);
    Ok(())
}

#[tokio::test]
async fn m1_f43_the_job_removes_the_sessions_nobody_can_use() -> Outcome {
    let world = World::new().await?;
    let sessions = || world.store.lock().state.sessions.len();
    assert_eq!(sessions(), 1);
    world.clock.advance(2 * SECONDS_PER_DAY);
    world.ctx().start_daily_jobs().await?;
    run_jobs(&world).await?;
    assert_eq!(sessions(), 0, "a session lasts a day at most");
    world.ctx().purge_dead_sessions().await?;
    assert_eq!(sessions(), 0);
    Ok(())
}

#[tokio::test]
async fn m1_f61_the_job_asks_the_log_file_to_rotate_with_the_size_and_the_files_of_the_settings()
-> Outcome {
    let file = FakeLogFile::default();
    let mut world = World::new().await?;
    world.services = world.services.with_server_log(Arc::new(file.clone()));
    world.ctx().start_daily_jobs().await?;
    run_jobs(&world).await?;
    let asked = file
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    assert_eq!(asked, [(20 * 1_048_576, 5)], "the starting values");
    for (key, value) in [
        ("logs.server-log-megabytes", 3),
        ("logs.server-log-files", 2),
    ] {
        change_setting(
            &world.ctx(),
            ChangeSetting {
                version: None,
                session_token: token(&world),
                scope: SettingsScope::Organization,
                key: key.to_owned(),
                value: SettingValue::Integer(value),
            },
        )
        .await?;
    }
    world.clock.advance(SECONDS_PER_DAY + 1);
    run_jobs(&world).await?;
    let asked = file
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    assert_eq!(asked.last(), Some(&(3 * 1_048_576, 2)));
    Ok(())
}

#[tokio::test]
async fn m1_f61_a_server_that_logs_to_its_console_has_nothing_to_rotate() -> Outcome {
    let world = World::new().await?;
    assert!(!world.ctx().rotate_old_server_log().await?);
    Ok(())
}

#[test]
fn priv_05_every_kind_of_data_has_one_row_and_one_group() {
    use std::collections::BTreeSet;

    use bb_core::keeping::{KEEP_KINDS, KeepGroup};

    // Each row Bugaboo deletes by itself has exactly one daily job; a business record has none.
    for row in KEEP_KINDS {
        let jobs: Vec<_> = JobKind::ALL
            .iter()
            .filter(|kind| kind.keeps().contains(&row.kind))
            .collect();
        match row.group {
            KeepGroup::ByItself => {
                assert_eq!(jobs.len(), 1, "{} needs one job", row.kind.code());
                assert!(jobs[0].is_daily(), "{}", jobs[0].code());
            }
            KeepGroup::ByAnAdministrator => {
                assert!(jobs.is_empty(), "a job deletes {}", row.kind.code());
            }
        }
    }
    // A daily job without a row deletes nothing past a keep period: these only.
    let without_rows: BTreeSet<_> = JobKind::daily()
        .filter(|kind| kind.keeps().is_empty())
        .collect();
    assert_eq!(
        without_rows,
        BTreeSet::from([
            JobKind::CatchUpHistory,
            JobKind::BackupsDaily,
            JobKind::RefreshStatistics,
            JobKind::RotateServerLog,
            // The daily round of *Download my data* deletes old exports with its own call until
            // it moves under the one shape of deleting (M9-F24).
            JobKind::DeleteOldExports,
        ])
    );
}

/// `count` jobs that gave up long ago, for the steps of the purge.
fn dead_jobs(world: &World, count: usize) -> Result<(), String> {
    let mut ids = bb_core::id::SequentialIds::default();
    let start = world.ctx().now();
    let mut store = world.store.lock();
    for _ in 0..count {
        store.state.jobs.push(bb_core::job::Job {
            id: bb_core::id::JobId::generate(&mut ids).map_err(text)?,
            kind: JobKind::MailSend.name().map_err(text)?,
            payload: json!({"what": "changes"}),
            run_at: start,
            attempts: 5,
            max_attempts: 5,
            state: JobState::Dead,
            lease_owner: None,
            lease_until: None,
            last_failure: None,
            created_at: start,
        });
    }
    Ok(())
}

#[tokio::test]
async fn priv_05_each_purge_deletes_in_steps_and_queues_the_next_step() -> Outcome {
    use crate::keeping::KEEP_STEP_ROWS;

    let world = World::new().await?;
    let many = usize::try_from(KEEP_STEP_ROWS)? * 2 + 5;
    dead_jobs(&world, many)?;
    let dead = || {
        world
            .store
            .lock()
            .state
            .jobs
            .iter()
            .filter(|job| job.state == JobState::Dead)
            .count()
    };
    world
        .clock
        .advance((days_of(&KEEPING_DEAD_JOB_DAYS) + 1) * SECONDS_PER_DAY);
    // One step deletes one step's rows and queues the next step for now.
    let step = world.ctx().keep_step(JobKind::PurgeDeadJobs).await?;
    assert_eq!((step.rows, step.more), (KEEP_STEP_ROWS, true));
    assert_eq!(dead(), many - usize::try_from(KEEP_STEP_ROWS)?);
    // The queued steps finish the work; the last one queues no further step.
    run_jobs(&world).await?;
    assert_eq!(dead(), 0);
    let waiting = world
        .store
        .lock()
        .state
        .jobs
        .iter()
        .filter(|job| {
            job.kind
                == JobKind::PurgeDeadJobs
                    .name()
                    .unwrap_or_else(|_| job.kind.clone())
        })
        .count();
    assert!(waiting <= 1, "at most the next daily round waits");
    Ok(())
}

#[tokio::test]
async fn priv_05_a_deletion_stopped_after_a_step_has_written_down_what_that_step_deleted() -> Outcome
{
    use bb_core::event::{EntityRef, RecordKind};
    use bb_core::id::UserId;
    use bb_core::keeping::KeepKind;
    use bb_core::sign_in::SecurityAction;

    use crate::keeping::{DeleteOldRecords, KEEP_STEP_ROWS, OldRecordsQuery};

    let world = World::new().await?;
    let ada = world.admin.person.id;
    let bea = UserId::from_random([9; 16]);
    let long_ago = jiff::civil::Date::new(2000, 1, 3)?;
    // The first step takes all of Ada's entries and some of Bea's.
    let (of_ada, of_bea) = (KEEP_STEP_ROWS / 2 + 100, KEEP_STEP_ROWS / 2 + 100);
    for _ in 0..of_ada {
        world.store.book(ada, long_ago, 30);
    }
    for _ in 0..of_bea {
        world.store.book(bea, long_ago, 30);
    }
    let before = crate::keeping::old_records(
        &world.ctx(),
        OldRecordsQuery {
            session_token: token(&world),
            kind: KeepKind::TimeEntries,
        },
    )
    .await?
    .into_value()
    .before
    .to_string();
    let delete = async || {
        crate::keeping::delete_old_records(
            &world.ctx(),
            DeleteOldRecords {
                session_token: token(&world),
                kind: KeepKind::TimeEntries,
                person: None,
                before: before.clone(),
            },
        )
        .await
    };
    let written = |world: &World| -> Vec<Option<EntityRef>> {
        world
            .store
            .lock()
            .log
            .iter()
            .filter(|entry| entry.action == SecurityAction::TimeEntriesDeleted)
            .map(|entry| entry.subject.clone())
            .collect()
    };
    let about = |person: UserId| Some(crate::auth::entity(RecordKind::Person, person));
    // The request stops after its first step, as one cut off at its time limit does.
    world.store.keep_a_change_busy_after(1);
    assert!(delete().await.is_err());
    let left = world.store.lock().state.time_entries.len();
    assert_eq!(
        u64::try_from(left)?,
        of_ada + of_bea - KEEP_STEP_ROWS,
        "one step went"
    );
    // That step is written down, for each person whose entries it deleted, and nothing more.
    let first = written(&world);
    assert_eq!(first.len(), 2, "{first:?}");
    assert!(first.contains(&about(ada)) && first.contains(&about(bea)));
    // Asked again with the same date, the rest goes, written down too.
    let deleted = delete().await?.into_value();
    assert_eq!(deleted.deleted, u64::try_from(left)?);
    assert!(world.store.lock().state.time_entries.is_empty());
    assert_eq!(written(&world)[2..], [about(bea)]);
    Ok(())
}

#[tokio::test]
async fn priv_05_every_purge_is_safe_to_run_again() -> Outcome {
    let world = World::new().await?;
    world.clock.advance(800 * SECONDS_PER_DAY);
    let deleting: Vec<_> = JobKind::daily()
        .filter(|kind| !kind.keeps().is_empty())
        .collect();
    for kind in &deleting {
        world.ctx().keep_step(*kind).await?;
    }
    for kind in &deleting {
        let again = world.ctx().keep_step(*kind).await?;
        assert_eq!(again.rows, 0, "{} deleted again", kind.code());
        assert!(!again.more);
    }
    Ok(())
}

#[tokio::test]
async fn priv_05_no_purge_takes_a_business_record() -> Outcome {
    let world = World::new().await?;
    let person = world.admin.person.id;
    // Far older than the ten years of the start: past its keep period now.
    let long_ago = jiff::civil::Date::new(2000, 1, 3)?;
    world.store.book(person, long_ago, 30);
    let deleting: Vec<_> = JobKind::daily()
        .filter(|kind| !kind.keeps().is_empty())
        .collect();
    for kind in deleting {
        world.ctx().keep_step(kind).await?;
    }
    assert_eq!(
        world.store.lock().state.time_entries.len(),
        1,
        "a time entry is deleted only by an administrator"
    );
    // Neither can the use case of old records be pointed at a kind Bugaboo deletes by itself.
    let refused = crate::keeping::old_records(
        &world.ctx(),
        crate::keeping::OldRecordsQuery {
            session_token: token(&world),
            kind: bb_core::keeping::KeepKind::Notifications,
        },
    )
    .await;
    assert!(matches!(
        refused,
        Err(crate::auth::AppError::Refused(bb_core::ErrorCode::NotFound))
    ));
    let old = crate::keeping::old_records(
        &world.ctx(),
        crate::keeping::OldRecordsQuery {
            session_token: token(&world),
            kind: bb_core::keeping::KeepKind::TimeEntries,
        },
    )
    .await?
    .into_value();
    assert_eq!(old.total, 1);
    let deleted = crate::keeping::delete_old_records(
        &world.ctx(),
        crate::keeping::DeleteOldRecords {
            session_token: token(&world),
            kind: bb_core::keeping::KeepKind::TimeEntries,
            person: None,
            before: old.before.to_string(),
        },
    )
    .await?
    .into_value();
    assert_eq!(deleted.deleted, 1);
    assert!(world.store.lock().state.time_entries.is_empty());
    Ok(())
}

#[tokio::test]
async fn priv_05_the_port_of_keep_periods_answers_with_the_organizations_settings() -> Outcome {
    use bb_core::keeping::KeepKind;
    use jiff::civil::date;

    use crate::ports::KeepPeriods as _;

    let world = World::new().await?;
    let recorded = date(2026, 3, 10);
    let periods = world.ctx().keep_periods().await?;
    // The starting periods: ten years for time entries, a month for the trash.
    assert_eq!(
        periods.kept_until(KeepKind::TimeEntries, recorded),
        Some(date(2036, 3, 11))
    );
    assert_eq!(
        periods.kept_until(KeepKind::Trash, recorded),
        Some(date(2026, 4, 10))
    );
    // The organization keeps time entries for eight years: the port follows the setting.
    change_setting(
        &world.ctx(),
        ChangeSetting {
            version: None,
            session_token: token(&world),
            scope: SettingsScope::Organization,
            key: bb_core::settings::keeping::KEEPING_TIME_ENTRY_YEARS
                .definition()
                .key()
                .to_owned(),
            value: SettingValue::Integer(8),
        },
    )
    .await?;
    let periods = world.ctx().keep_periods().await?;
    assert_eq!(
        periods.kept_until(KeepKind::TimeEntries, recorded),
        Some(date(2034, 3, 11))
    );
    Ok(())
}
