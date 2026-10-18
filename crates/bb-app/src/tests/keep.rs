//! Keeping and deleting (PRIV-05, M1-F21, M1-F43, M1-F61): the jobs remove what outlived its keep
//! period, and each is safe to run again (ADR-0017).

use std::sync::{Arc, Mutex, PoisonError};

use bb_core::job::{DEAD_PAYLOAD_KEEP_DAYS, JobState};
use bb_core::notification::KEEP_DAYS;
use bb_core::settings::SettingValue;
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
    world.clock.advance((KEEP_DAYS - 1) * SECONDS_PER_DAY);
    world.ctx().start_daily_jobs().await?;
    run_jobs(world).await?;
    assert_eq!(world.store.lock().state.notifications.len(), 1);
    // A day later it goes, and a second round has nothing more to delete.
    world.clock.advance(2 * SECONDS_PER_DAY);
    world.ctx().start_daily_jobs().await?;
    run_jobs(world).await?;
    assert_eq!(world.store.lock().state.notifications.len(), 0);
    let again = world.ctx().purge_old_notifications().await?;
    assert_eq!((again.notifications, again.payloads), (0, 0));
    Ok(())
}

#[tokio::test]
async fn m1_f21_the_payload_of_a_dead_job_is_emptied_after_its_keep_period() -> Outcome {
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
    assert_eq!(world.ctx().purge_old_notifications().await?.payloads, 0);
    world
        .clock
        .advance((DEAD_PAYLOAD_KEEP_DAYS + 1) * SECONDS_PER_DAY);
    assert_eq!(world.ctx().purge_old_notifications().await?.payloads, 1);
    assert!(world.store.lock().state.jobs[0].payload.is_null());
    assert_eq!(
        world.store.lock().state.jobs[0].state,
        JobState::Dead,
        "the job stays for an administrator to look at"
    );
    assert_eq!(world.ctx().purge_old_notifications().await?.payloads, 0);
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
