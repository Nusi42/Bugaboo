//! The jobs that come back every day (ADR-0017, M3-F30, M3-F71): clearing what outlived its keep
//! period, and the history catching up with the events. They run from the queue, the same runner
//! as the steps of a change to many issues, and no timer of the server does them.

use bb_core::edit::{DRAFT_KEEP_DAYS, Draft, DraftTarget};
use bb_core::event::EntityKind;
use bb_core::job::JobState;
use bb_core::time::SECONDS_PER_DAY as DAY;

use super::scenarios::a_project;
use super::world::{Outcome, World};
use crate::jobs::{DAILY, kind};

async fn run_all(world: &World) -> Result<usize, crate::auth::AppError> {
    let mut total = 0;
    loop {
        let taken = world.ctx().run_due_jobs().await?;
        if taken == 0 {
            return Ok(total);
        }
        total += taken;
    }
}

/// Every kind that comes back every day, in the order the queue lists them.
fn every_day() -> Vec<String> {
    let mut kinds: Vec<String> = kind::DAILY_JOBS
        .iter()
        .map(|kind| (*kind).to_owned())
        .collect();
    kinds.sort();
    kinds
}

fn kinds_waiting(world: &World) -> Vec<String> {
    let mut kinds: Vec<String> = world
        .store
        .lock()
        .state
        .jobs
        .iter()
        .filter(|job| job.state == JobState::Queued)
        .map(|job| job.kind.as_str().to_owned())
        .collect();
    kinds.sort();
    kinds
}

#[tokio::test]
async fn m3_f30_the_daily_jobs_run_at_the_start_and_come_back_every_day() -> Outcome {
    let world = World::new().await?;
    world.ctx().start_daily_jobs().await?;
    assert_eq!(kinds_waiting(&world), every_day());
    // At the start every kind runs, and each leaves its next round behind, a day away.
    assert_eq!(run_all(&world).await?, kind::DAILY_JOBS.len());
    assert_eq!(kinds_waiting(&world), every_day());
    assert_eq!(run_all(&world).await?, 0, "the next round is not due yet");
    world.clock.advance(DAILY.as_secs() + 1);
    assert_eq!(run_all(&world).await?, kind::DAILY_JOBS.len());
    assert_eq!(
        kinds_waiting(&world).len(),
        kind::DAILY_JOBS.len(),
        "never more than one each"
    );
    Ok(())
}

#[tokio::test]
async fn m3_f30_a_start_while_the_next_round_waits_does_not_add_a_second_chain() -> Outcome {
    let world = World::new().await?;
    world.ctx().start_daily_jobs().await?;
    run_all(&world).await?;
    // The server starts again within the day: it runs once more at once, and the chain stays one.
    world.ctx().start_daily_jobs().await?;
    assert_eq!(run_all(&world).await?, kind::DAILY_JOBS.len());
    assert_eq!(kinds_waiting(&world).len(), kind::DAILY_JOBS.len());
    Ok(())
}

#[tokio::test]
async fn m3_f30_the_clearing_job_deletes_a_draft_past_its_keep_period() -> Outcome {
    let world = World::new().await?;
    let author = world.admin.person.id;
    let project = a_project(&world, "ACME").await?;
    let draft = Draft {
        author,
        target: DraftTarget::NewRecord {
            project,
            kind: EntityKind::new("issue")?,
        },
        text: "old".to_owned(),
        base_version: None,
        saved_at: world.ctx().now(),
    };
    world.store.lock().state.drafts.push(draft);
    world.clock.advance((DRAFT_KEEP_DAYS + 1) * DAY);
    world.ctx().start_daily_jobs().await?;
    run_all(&world).await?;
    assert!(world.store.lock().state.drafts.is_empty());
    Ok(())
}

#[tokio::test]
async fn m3_f71_the_history_job_reads_the_events_that_wait() -> Outcome {
    let world = World::new().await?;
    let events = world.store.event_count();
    assert!(events > 0, "the world has events");
    assert_eq!(world.store.lock().state.history_position, 0);
    world.ctx().start_daily_jobs().await?;
    run_all(&world).await?;
    assert!(world.store.lock().state.history_position > 0);
    Ok(())
}
