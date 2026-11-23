//! The steps of the job that finishes a removal against the in-memory store (PRIV-04, ADR-0044,
//! decision 6): a job that gives up stays and goes on with the daily round, a step that runs twice
//! queues no second chain, a job queued before the removal worked in steps still finishes, and the
//! comments that go are hidden from the change of the removal on.

use bb_core::id::{CommentId, IssueId, JobId, SequentialIds};
use bb_core::job::JobState;
use bb_core::settings::privacy::RemovedTexts;
use jiff::Timestamp;

use super::removal::{finishing, pending_of, removed, removing, two_comments};
use super::scenarios::text;
use super::switch_off::switched_off;
use super::world::{Outcome, World};
use crate::jobs::{JobKind, JobStore as _};
use crate::ports::ChangeStore as _;
use crate::privacy::removal::RemovalStore as _;

/// Makes the job of the removal of `person` die, as after its last start.
fn kill_the_job(world: &World, person: bb_core::id::UserId) -> Result<JobId, String> {
    let job = pending_of(world, person)
        .ok_or("the removal is not finished")?
        .job;
    let mut shared = world.store.lock();
    let found = shared
        .state
        .jobs
        .iter_mut()
        .find(|found| found.id == job)
        .ok_or("the job of the removal waits")?;
    found.state = JobState::Dead;
    found.run_at = Timestamp::UNIX_EPOCH;
    Ok(job)
}

#[tokio::test]
async fn priv_04_a_job_that_gives_up_stays_and_goes_on_with_the_daily_round() -> Outcome {
    let world = World::new().await?;
    let bob = world.person("bob@example.test").await?.person.id;
    switched_off(&world, bob).await?;
    removed(&world, bob, None).await?;
    let dead = kill_the_job(&world, bob)?;
    // The purge of dead jobs keeps it while the removal is not finished, and the removal is
    // stopped: the status page counts it, and Bob's page still says *removing…*.
    assert_eq!(
        world
            .store
            .purge_dead_jobs(Timestamp::MAX, 100)
            .await
            .map_err(text)?,
        0
    );
    assert_eq!(world.store.stopped_removals().await.map_err(text)?.len(), 1);
    assert!(removing(&world, bob).await?);
    // A round of the queue does not take a dead job; the daily round gives it a new one.
    world.ctx().run_due_jobs().await.map_err(text)?;
    assert_eq!(world.store.stopped_removals().await.map_err(text)?.len(), 1);
    world.ctx().start_daily_jobs().await?;
    super::data_exports::run_jobs(&world).await?;
    assert!(
        world
            .store
            .stopped_removals()
            .await
            .map_err(text)?
            .is_empty()
    );
    assert!(!removing(&world, bob).await?, "the removal is finished");
    assert!(pending_of(&world, bob).is_none());
    assert!(
        world
            .store
            .removal_words(bob)
            .await
            .map_err(text)?
            .is_none(),
        "the words are gone"
    );
    // Nothing keeps the dead job any more: the purge deletes it.
    assert_eq!(
        world
            .store
            .purge_dead_jobs(Timestamp::MAX, 100)
            .await
            .map_err(text)?,
        1
    );
    assert!(
        world
            .store
            .read(|state| state.jobs.iter().all(|job| job.id != dead))
    );
    Ok(())
}

#[tokio::test]
async fn priv_04_a_step_that_runs_twice_queues_no_second_chain() -> Outcome {
    let world = World::new().await?;
    let bob = world.person("bob@example.test").await?.person.id;
    switched_off(&world, bob).await?;
    removed(&world, bob, None).await?;
    // The first step is done and has queued the second; its worker lost the lease before it
    // reported, so the first step is taken again.
    let first = world
        .store
        .read(|state| {
            state
                .jobs
                .iter()
                .find(|job| job.kind.as_str() == JobKind::FinishRemoval.code())
                .cloned()
        })
        .ok_or("the first step waits")?;
    world.ctx().run_due_jobs().await.map_err(text)?;
    world.store.lock().state.jobs.push(first);
    for _ in 0..10 {
        assert!(finishing(&world).len() <= 2, "{:?}", finishing(&world));
        world.ctx().run_due_jobs().await.map_err(text)?;
        let next = pending_of(&world, bob).map(|pending| pending.job);
        let waiting: Vec<_> = world.store.read(|state| {
            state
                .jobs
                .iter()
                .filter(|job| job.kind.as_str() == JobKind::FinishRemoval.code())
                .map(|job| job.id)
                .collect()
        });
        // Only the job the row names waits: the copy did nothing and queued nothing.
        assert!(waiting.len() <= 1, "{waiting:?}");
        assert_eq!(waiting.first().copied(), next);
    }
    assert!(!removing(&world, bob).await?);
    Ok(())
}

#[tokio::test]
async fn priv_04_a_job_queued_before_the_steps_still_finishes() -> Outcome {
    let world = World::new().await?;
    let mut ids = SequentialIds::default();
    let comment = CommentId::generate(&mut ids).map_err(text)?;
    let issue = IssueId::generate(&mut ids).map_err(text)?;
    let job = JobId::generate(&mut ids).map_err(text)?;
    let earlier = serde_json::json!({ "comments": [comment], "issues": [issue], "exports": [] });
    world
        .store
        .queue_job(
            job,
            &JobKind::FinishRemoval.name()?,
            &earlier,
            world.ctx().now(),
            world.ctx().now(),
        )
        .await
        .map_err(text)?;
    super::data_exports::run_jobs(&world).await?;
    let (reindexed, compactions, dead) = world.store.read(|state| {
        (
            state.reindexed.clone(),
            state.compactions,
            state.jobs.iter().any(|job| job.state == JobState::Dead),
        )
    });
    assert_eq!(reindexed, vec![(vec![issue], vec![comment])]);
    assert_eq!(compactions, 1);
    assert!(!dead, "the job did not give up");
    assert!(finishing(&world).is_empty());
    Ok(())
}

#[tokio::test]
async fn priv_04_the_comments_that_go_are_hidden_from_the_change_of_the_removal_on() -> Outcome {
    let world = World::new().await?;
    let bob = world.person("bob@example.test").await?.person.id;
    switched_off(&world, bob).await?;
    let mut ids = SequentialIds::default();
    two_comments(&world, &mut ids, bob)?;
    removed(&world, bob, Some(RemovedTexts::Delete)).await?;
    // Before the job ran: Bob's comment is in the trash, Ada's is where it was.
    let (his, hers) = world.store.read(|state| {
        let of = |author| {
            state
                .comments
                .iter()
                .find(|comment| comment.author == author)
                .cloned()
        };
        (of(bob), of(world.admin.person.id))
    });
    let his = his.ok_or("his comment waits for the job")?;
    assert!(his.deleted_at.is_some());
    assert!(hers.ok_or("her comment stays")?.deleted_at.is_none());
    // Nobody takes it out of the trash while the removal goes on.
    let id = his.id;
    let restored = world
        .store
        .record_changes(&[], move |tx| {
            Box::pin(async move {
                <super::fake::FakeStore as crate::comments::CommentStore>::set_comment_deleted(
                    tx, id, None, None,
                )
                .await
            })
        })
        .await
        .map_err(text)?;
    assert!(!restored);
    super::data_exports::run_jobs(&world).await?;
    assert!(
        world
            .store
            .read(|state| state.comments.iter().all(|comment| comment.author != bob)),
        "the job deleted it"
    );
    Ok(())
}
