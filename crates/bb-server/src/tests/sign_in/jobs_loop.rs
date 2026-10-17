//! The loop of the job runner (`bb-jobs`, ADR-0017, M3-F155): it takes the jobs that are due, goes
//! on after a job that dies, and stops when it is stopped. `run_due_jobs` has its tests in
//! `bb-app`; this is the loop around it, on both databases.

use std::sync::Arc;
use std::time::Duration;

use bb_app::context::SystemIds;
use bb_app::jobs::JobStore as _;
use bb_core::id::{JobId, UserId};
use bb_core::technical::TechnicalName;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::json;

use super::rig::Rig;

/// How often the loop of a test looks for jobs: much shorter than the real `POLL_EVERY`, so a test
/// does not wait for it.
const TEST_POLL: Duration = Duration::from_millis(10);

/// The longest a test waits for the loop to do what it should. A loop that works needs a few
/// polls; this is only the limit that stops a loop that does not.
const WAIT_AT_MOST: Duration = Duration::from_secs(5);

/// How long a test lets a stopped loop show that it does nothing more.
const STOPPED_FOR: Duration = Duration::from_millis(200);

/// Waits until the queue holds no job that is not dead.
async fn until_the_queue_is_empty(rig: &Rig) -> Result<(), Failure> {
    let waited = tokio::time::timeout(WAIT_AT_MOST, async {
        loop {
            if rig.database.queued_job_count().await? == 0 {
                return Ok::<(), Failure>(());
            }
            tokio::time::sleep(TEST_POLL).await;
        }
    })
    .await;
    waited.map_err(|_| "the loop did not take the jobs in time")?
}

/// Puts an e-mail job in the queue that has nothing to send: it is done at once and leaves the
/// queue.
async fn queue_a_job_that_is_done_at_once(rig: &Rig, number: u8) -> Result<(), Failure> {
    let id = JobId::generate(&mut SystemIds)?;
    // A different person for each job, so the payloads differ.
    let nobody = UserId::from_random([number; 16]);
    let now = rig.clock_now();
    rig.storage
        .queue_job(
            id,
            &TechnicalName::new("mail.send")?,
            &json!({"what": "changes", "recipient": nobody.to_string(), "issue": null}),
            now,
            now,
        )
        .await?;
    Ok(())
}

#[tokio::test]
async fn adr_0017_the_loop_takes_the_jobs_that_are_due_and_stops_when_it_is_stopped() -> TestResult
{
    each_database("adr_0017_loop", async |db| {
        let rig = Rig::start(db).await?;
        queue_a_job_that_is_done_at_once(&rig, 1).await?;
        assert_eq!(rig.database.queued_job_count().await?, 1);
        let running = bb_jobs::spawn(Arc::clone(&rig.services), TEST_POLL);
        until_the_queue_is_empty(&rig).await?;

        // A stopped loop does nothing more.
        running.abort();
        let _ = running.await;
        queue_a_job_that_is_done_at_once(&rig, 2).await?;
        tokio::time::sleep(STOPPED_FOR).await;
        assert_eq!(rig.database.queued_job_count().await?, 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn adr_0017_a_job_that_dies_does_not_stop_the_loop() -> TestResult {
    each_database("adr_0017_loop_dead_job", async |db| {
        let rig = Rig::start(db).await?;
        // A job of a kind nobody does dies at once; the one after it is still done.
        let now = rig.clock_now();
        rig.storage
            .queue_job(
                JobId::generate(&mut SystemIds)?,
                &TechnicalName::new("nobody.does-this")?,
                &json!({}),
                now,
                now,
            )
            .await?;
        queue_a_job_that_is_done_at_once(&rig, 3).await?;
        let running = bb_jobs::spawn(Arc::clone(&rig.services), TEST_POLL);
        until_the_queue_is_empty(&rig).await?;
        running.abort();
        assert_eq!(rig.database.dead_jobs(10).await?.len(), 1);
        Ok(())
    })
    .await
}

#[test]
fn adr_0017_the_real_loop_looks_for_jobs_every_few_seconds() {
    assert_eq!(bb_jobs::POLL_EVERY, Duration::from_secs(2));
}
