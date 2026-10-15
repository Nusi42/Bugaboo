//! Runs of changes to many issues and the jobs that carry them, on both databases (ISS-09,
//! ADR-0017): a run is saved as it is read, its progress is counted once for each issue even when
//! two workers took the same step, and a job queued in the change that makes the run is leased
//! once.

use bb_app::bulk::BulkStore;
use bb_app::jobs::{JOB_ATTEMPTS_MAX, JobStore};
use bb_app::ports::{BoxFuture, ChangeStore};
use bb_core::bulk::{BulkItem, BulkRun, BulkSkip, BulkStatus, SkipReason};
use bb_core::id::{BulkRunId, IdSource, IssueId, JobId, UserId};
use bb_core::job::{FailureKind, FailureOutcome};
use bb_core::technical::TechnicalName;
use bb_testing::{TestResult, each_database};
use jiff::SignedDuration;
use serde_json::json;

use super::*;
use crate::test_support::{at, event, fixture, ids};

async fn changed<T: Send>(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    write: impl for<'t> FnOnce(&'t mut Transaction) -> BoxFuture<'t, Result<T, StoreError>> + Send,
) -> Result<T, StoreError> {
    ChangeStore::record_change(storage, &event(source, 5), write).await
}

fn a_run(source: &mut impl IdSource, issues: usize) -> BulkRun {
    BulkRun {
        id: BulkRunId::generate(source).unwrap(),
        person: UserId::generate(source),
        action: json!({"kind": "delete"}),
        items: (0..issues)
            .map(|_| BulkItem {
                issue: IssueId::generate(source).unwrap(),
                version: 1,
            })
            .collect(),
        status: BulkStatus::Queued,
        done: 0,
        changed: 0,
        skipped: Vec::new(),
        created_at: at(1),
        finished_at: None,
    }
}

#[tokio::test]
async fn iss_09_a_run_and_its_job_are_saved_together_and_the_progress_is_counted_once() -> TestResult
{
    each_database("iss_09_run_and_job", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let run = a_run(&mut source, 3);
        let job = JobId::generate(&mut source).unwrap();
        let kind = TechnicalName::new("bulk.step")?;
        let (stored, payload) = (run.clone(), json!({"run": run.id.to_string()}));
        let (queued_kind, queued_payload) = (kind.clone(), payload.clone());
        changed(storage, &mut source, move |tx| {
            Box::pin(async move {
                AuthStorage::insert_bulk_run(tx, &stored).await?;
                AuthStorage::enqueue_job(tx, job, &queued_kind, &queued_payload, at(1), at(1)).await
            })
        })
        .await?;
        assert_eq!(storage.bulk_run(run.id).await?, Some(run.clone()));
        // The job is leased once, with the payload that names the run.
        let worker = TechnicalName::new("worker.one")?;
        let leased = storage
            .lease_jobs(&worker, at(2), SignedDuration::from_secs(60), 10)
            .await?;
        assert_eq!(leased.len(), 1);
        assert_eq!(
            (leased[0].kind.clone(), leased[0].payload.clone()),
            (kind, payload)
        );
        assert_eq!(leased[0].max_attempts, JOB_ATTEMPTS_MAX);
        let other = TechnicalName::new("worker.two")?;
        assert!(
            storage
                .lease_jobs(&other, at(2), SignedDuration::from_secs(60), 10)
                .await?
                .is_empty()
        );
        // Counting an issue: only the worker that is at that issue counts it.
        let mut progress = run.clone();
        progress.status = BulkStatus::Running;
        progress.done = 1;
        progress.changed = 1;
        assert!(storage.save_bulk_progress(&progress, 0).await?);
        let late = storage.save_bulk_progress(&progress, 0).await?;
        assert!(!late, "the same issue is not counted twice");
        progress.done = 3;
        progress.changed = 2;
        progress.status = BulkStatus::Done;
        progress.skipped = vec![BulkSkip {
            issue: run.items[2].issue,
            reason: SkipReason::ChangedSince,
        }];
        progress.finished_at = Some(at(9));
        assert!(storage.save_bulk_progress(&progress, 1).await?);
        assert_eq!(storage.bulk_run(run.id).await?, Some(progress));
        // The worker reports the job done.
        assert!(storage.complete_job(leased[0].id, &worker).await?);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_09_a_job_queued_on_its_own_waits_for_its_time() -> TestResult {
    each_database("iss_09_queue_job", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let kind = TechnicalName::new("bulk.step")?;
        storage
            .queue_job(
                JobId::generate(&mut source).unwrap(),
                &kind,
                &json!({"run": "x"}),
                at(100),
                at(1),
            )
            .await?;
        let worker = TechnicalName::new("worker.one")?;
        let early = storage
            .lease_jobs(&worker, at(10), SignedDuration::from_secs(60), 10)
            .await?;
        assert!(early.is_empty());
        let due = storage
            .lease_jobs(&worker, at(101), SignedDuration::from_secs(60), 10)
            .await?;
        assert_eq!(due.len(), 1);
        let failure = TechnicalName::new("job.failed")?;
        let outcome = storage
            .fail_job(
                due[0].id,
                &worker,
                &failure,
                FailureKind::Temporary,
                at(101),
            )
            .await?;
        assert!(
            matches!(outcome, Some(FailureOutcome::Retry(_))),
            "{outcome:?}"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_09_a_job_that_waits_for_later_is_told_by_its_kind() -> TestResult {
    each_database("iss_09_later_job_waits", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let kind = TechnicalName::new(bb_app::jobs::kind::PURGE_DRAFTS)?;
        let other = TechnicalName::new("history.catch-up")?;
        assert!(!storage.later_job_waits(&kind, at(10)).await?);
        storage
            .queue_job(
                JobId::generate(&mut source).unwrap(),
                &kind,
                &json!({}),
                at(100),
                at(1),
            )
            .await?;
        assert!(storage.later_job_waits(&kind, at(10)).await?);
        assert!(!storage.later_job_waits(&kind, at(100)).await?);
        assert!(!storage.later_job_waits(&other, at(10)).await?);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn adr_0017_an_event_is_found_by_its_reason_since_a_moment() -> TestResult {
    each_database("adr_0017_event_by_reason", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let mut marked = event(&mut source, 5);
        marked.reason = Some("bulk-run:one:7".to_owned());
        ChangeStore::record_change(storage, &marked, |_| Box::pin(async { Ok(()) })).await?;
        let early = Timestamp::UNIX_EPOCH;
        let late = Timestamp::from_second(4_000_000_000)?;
        assert!(storage.event_with_reason("bulk-run:one:7", early).await?);
        assert!(!storage.event_with_reason("bulk-run:one:8", early).await?);
        assert!(!storage.event_with_reason("bulk-run:one:7", late).await?);
        Ok(())
    })
    .await
}
