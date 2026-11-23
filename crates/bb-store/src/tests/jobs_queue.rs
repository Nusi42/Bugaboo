//! The queue of jobs: leases, retries and the dead-letter list (ADR-0017).

use std::collections::HashSet;

use bb_testing::{TestResult, each_database};

use super::*;
use crate::test_support::{at, ids, open};

fn name(text: &str) -> TechnicalName {
    TechnicalName::new(text).unwrap()
}

fn mail(run_at: Timestamp, max_attempts: u32) -> NewJob {
    NewJob {
        kind: name("mail.send"),
        payload: serde_json::json!({"to": "someone"}),
        run_at,
        max_attempts,
    }
}

const LEASE: SignedDuration = SignedDuration::from_secs(60);

/// Every kind: the tests of the queue itself look at no lane.
fn any() -> LaneKinds {
    LaneKinds::AllBut(Vec::new())
}

#[test]
fn retries_wait_longer_each_time_up_to_an_hour() {
    let waits: Vec<i64> = (1..=9).map(|attempt| backoff(attempt).as_secs()).collect();
    assert_eq!(waits, [30, 60, 120, 240, 480, 960, 1920, 3600, 3600]);
    assert_eq!(backoff(0).as_secs(), 30);
    assert_eq!(backoff(u32::MAX).as_secs(), 3600);
}

#[tokio::test]
async fn adr_0017_a_job_is_leased_once_and_comes_back_when_the_lease_runs_out() -> TestResult {
    each_database("adr_0017_lease", async |db| {
        let database = open(db).await?;
        let mut source = ids();
        let id = JobId::generate(&mut source)?;
        database.enqueue_job(id, &mail(at(0), 3), at(0)).await?;
        let worker = name("worker-1");

        let leased = database
            .lease_jobs(&worker, at(1), LEASE, 5, &any())
            .await?;
        assert_eq!(leased.len(), 1);
        assert_eq!(leased[0].id, id);
        assert_eq!(leased[0].attempts, 1);
        assert_eq!(leased[0].lease_owner.as_ref(), Some(&worker));
        assert_eq!(leased[0].payload, serde_json::json!({"to": "someone"}));
        assert!(
            database
                .lease_jobs(&name("worker-2"), at(30), LEASE, 5, &any())
                .await?
                .is_empty()
        );

        let again = database
            .lease_jobs(&name("worker-2"), at(70), LEASE, 5, &any())
            .await?;
        assert_eq!(again.len(), 1);
        assert_eq!(again[0].attempts, 2);
        assert!(
            !database.complete_job(id, &worker).await?,
            "worker-1 lost the lease"
        );
        assert!(database.complete_job(id, &name("worker-2")).await?);
        assert_eq!(database.queued_job_count().await?, 0);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn adr_0017_a_long_job_keeps_its_lease_while_its_worker_holds_it() -> TestResult {
    each_database("adr_0017_hold", async |db| {
        let database = open(db).await?;
        let id = JobId::generate(&mut ids())?;
        database.enqueue_job(id, &mail(at(0), 3), at(0)).await?;
        let (first, second) = (name("worker-1"), name("worker-2"));
        assert_eq!(
            database
                .lease_jobs(&first, at(1), LEASE, 5, &any())
                .await?
                .len(),
            1
        );
        assert!(database.hold_job(id, &first, at(500)).await?);
        assert!(
            database
                .lease_jobs(&second, at(400), LEASE, 5, &any())
                .await?
                .is_empty()
        );
        assert!(!database.hold_job(id, &second, at(900)).await?);
        assert_eq!(
            database
                .lease_jobs(&second, at(501), LEASE, 5, &any())
                .await?
                .len(),
            1
        );
        assert!(
            !database.hold_job(id, &first, at(900)).await?,
            "worker-1 lost it"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn a_job_waits_for_its_time() -> TestResult {
    each_database("adr_0017_run_at", async |db| {
        let database = open(db).await?;
        let id = JobId::generate(&mut ids())?;
        database.enqueue_job(id, &mail(at(100), 3), at(0)).await?;
        let worker = name("worker-1");
        assert!(
            database
                .lease_jobs(&worker, at(99), LEASE, 5, &any())
                .await?
                .is_empty()
        );
        assert_eq!(
            database
                .lease_jobs(&worker, at(100), LEASE, 5, &any())
                .await?
                .len(),
            1
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn failed_jobs_back_off_and_end_on_the_dead_letter_list() -> TestResult {
    each_database("adr_0017_dead_letter", async |db| {
        let database = open(db).await?;
        let id = JobId::generate(&mut ids())?;
        database.enqueue_job(id, &mail(at(0), 2), at(0)).await?;
        let worker = name("worker-1");
        let failure = name("mail.refused");

        database
            .lease_jobs(&worker, at(0), LEASE, 1, &any())
            .await?;
        let first = database
            .fail_job(id, &worker, &failure, FailureKind::Temporary, at(1))
            .await?;
        assert_eq!(first, Some(FailureOutcome::Retry(at(31))));
        assert!(
            database
                .lease_jobs(&worker, at(30), LEASE, 1, &any())
                .await?
                .is_empty()
        );

        assert_eq!(
            database
                .lease_jobs(&worker, at(31), LEASE, 1, &any())
                .await?
                .len(),
            1
        );
        let second = database
            .fail_job(id, &worker, &failure, FailureKind::Temporary, at(32))
            .await?;
        assert_eq!(second, Some(FailureOutcome::Dead));
        assert_eq!(database.queued_job_count().await?, 0);
        let dead = database.dead_jobs(10).await?;
        assert_eq!(dead.len(), 1);
        assert_eq!(dead[0].last_failure.as_ref(), Some(&failure));
        assert_eq!(dead[0].state, JobState::Dead);
        assert!(
            database
                .lease_jobs(&worker, at(10_000), LEASE, 1, &any())
                .await?
                .is_empty()
        );

        assert!(database.requeue_dead_job(id, at(100)).await?);
        assert!(!database.requeue_dead_job(id, at(100)).await?);
        let revived = database
            .lease_jobs(&worker, at(100), LEASE, 1, &any())
            .await?;
        assert_eq!(revived[0].attempts, 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn a_failure_that_is_permanent_goes_to_the_dead_letter_list_at_once() -> TestResult {
    each_database("adr_0017_permanent", async |db| {
        let database = open(db).await?;
        let id = JobId::generate(&mut ids())?;
        database.enqueue_job(id, &mail(at(0), 5), at(0)).await?;
        let worker = name("worker-1");
        database
            .lease_jobs(&worker, at(0), LEASE, 1, &any())
            .await?;
        let outcome = database
            .fail_job(
                id,
                &worker,
                &name("job.failed"),
                FailureKind::Permanent,
                at(1),
            )
            .await?;
        assert_eq!(
            outcome,
            Some(FailureOutcome::Dead),
            "after one start of five"
        );
        assert_eq!(database.dead_jobs(10).await?.len(), 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn bad_example_only_the_lease_holder_can_report_a_job() -> TestResult {
    each_database("adr_0017_wrong_worker", async |db| {
        let database = open(db).await?;
        let id = JobId::generate(&mut ids())?;
        database.enqueue_job(id, &mail(at(0), 3), at(0)).await?;
        database
            .lease_jobs(&name("worker-1"), at(0), LEASE, 1, &any())
            .await?;
        let other = name("worker-2");
        assert!(!database.complete_job(id, &other).await?);
        let outcome = database
            .fail_job(id, &other, &name("x.y"), FailureKind::Temporary, at(1))
            .await?;
        assert_eq!(outcome, None);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn a_job_that_dies_with_its_last_attempt_goes_to_the_dead_letter_list() -> TestResult {
    each_database("adr_0017_crashed_last_attempt", async |db| {
        let database = open(db).await?;
        let id = JobId::generate(&mut ids())?;
        database.enqueue_job(id, &mail(at(0), 1), at(0)).await?;
        let worker = name("worker-1");
        assert_eq!(
            database
                .lease_jobs(&worker, at(0), LEASE, 1, &any())
                .await?
                .len(),
            1
        );
        // The worker never answers.
        assert!(
            database
                .lease_jobs(&worker, at(61), LEASE, 1, &any())
                .await?
                .is_empty()
        );
        let dead = database.dead_jobs(10).await?;
        assert_eq!(dead.len(), 1);
        assert_eq!(dead[0].last_failure, Some(name(LEASE_EXPIRED)));
        Ok(())
    })
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn workers_at_the_same_time_never_get_the_same_job() -> TestResult {
    each_database("adr_0017_parallel_workers", async |db| {
        let database = open(db).await?;
        let mut source = ids();
        for _ in 0..20 {
            let id = JobId::generate(&mut source)?;
            database.enqueue_job(id, &mail(at(0), 3), at(0)).await?;
        }
        let mut tasks = Vec::new();
        for number in 0..4 {
            let database = database.clone();
            tasks.push(tokio::spawn(async move {
                let worker = TechnicalName::new(&format!("worker-{number}")).unwrap();
                let mut mine = Vec::new();
                loop {
                    let batch = database
                        .lease_jobs(&worker, at(1), LEASE, 3, &any())
                        .await?;
                    if batch.is_empty() {
                        break;
                    }
                    mine.extend(batch.into_iter().map(|job| job.id));
                }
                Ok::<_, StoreError>(mine)
            }));
        }
        let mut all = Vec::new();
        for task in tasks {
            all.extend(task.await??);
        }
        let distinct: HashSet<JobId> = all.iter().copied().collect();
        assert_eq!(all.len(), 20);
        assert_eq!(distinct.len(), 20, "no job was handed out twice");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn bad_example_a_job_that_may_run_zero_times_is_refused() -> TestResult {
    each_database("adr_0017_zero_attempts", async |db| {
        let database = open(db).await?;
        let id = JobId::generate(&mut ids())?;
        let refused = database.enqueue_job(id, &mail(at(0), 0), at(0)).await;
        assert!(
            matches!(refused, Err(StoreError::InvalidInput(_))),
            "{refused:?}"
        );
        assert_eq!(database.queued_job_count().await?, 0);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn adr_0017_each_lane_takes_only_its_kinds() -> TestResult {
    each_database("adr_0017_lanes", async |db| {
        let database = open(db).await?;
        let mut source = ids();
        let preview = JobId::generate(&mut source)?;
        let preview_job = NewJob {
            kind: name("files.preview"),
            ..mail(at(0), 3)
        };
        database.enqueue_job(preview, &preview_job, at(0)).await?;
        let mail_id = JobId::generate(&mut source)?;
        database
            .enqueue_job(mail_id, &mail(at(1), 3), at(1))
            .await?;
        let long = LaneKinds::Only(vec![name("files.preview")]);
        let short = LaneKinds::AllBut(vec![name("files.preview")]);
        // The e-mail is younger, but the short lane takes it and leaves the preview.
        let taken = database
            .lease_jobs(&name("worker-short"), at(2), LEASE, 5, &short)
            .await?;
        assert_eq!(
            taken.iter().map(|job| job.id).collect::<Vec<_>>(),
            [mail_id]
        );
        let taken = database
            .lease_jobs(&name("worker-long"), at(2), LEASE, 5, &long)
            .await?;
        assert_eq!(
            taken.iter().map(|job| job.id).collect::<Vec<_>>(),
            [preview]
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ops_07_the_newest_job_of_a_kind_is_found_waiting_or_dead() -> TestResult {
    // The list of backups says from it whether a backup asked for runs or failed.
    each_database("ops_07_newest_job", async |db| {
        let database = open(db).await?;
        let kind = name("mail.send");
        assert_eq!(database.newest_job(&kind).await?, None);
        let mut source = ids();
        let (older, newer) = (JobId::generate(&mut source)?, JobId::generate(&mut source)?);
        database.enqueue_job(older, &mail(at(0), 5), at(0)).await?;
        database.enqueue_job(newer, &mail(at(0), 5), at(5)).await?;
        let found = database.newest_job(&kind).await?.ok_or("no job")?;
        assert_eq!((found.id, found.state), (newer, JobState::Queued));
        let worker = name("worker-1");
        database
            .lease_jobs(&worker, at(10), LEASE, 2, &any())
            .await?;
        database
            .fail_job(
                newer,
                &worker,
                &name("job.failed"),
                FailureKind::Permanent,
                at(11),
            )
            .await?;
        let found = database.newest_job(&kind).await?.ok_or("no job")?;
        assert_eq!((found.id, found.state), (newer, JobState::Dead));
        assert_eq!(database.newest_job(&name("files.preview")).await?, None);
        Ok(())
    })
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ops_07_a_second_ask_for_a_backup_waits_for_the_first_and_sees_its_job() -> TestResult {
    use bb_app::backups::BackupStore as _;

    use crate::auth::AuthStorage;

    /// How long the second ask is watched while the first holds the asks.
    const WATCHED: std::time::Duration = std::time::Duration::from_millis(300);
    each_database("ops_07_backup_asks", async |db| {
        let database = open(db).await?;
        let kind = name(JobKind::BackupNow.code());
        let mut first = database.begin().await?;
        AuthStorage::lock_backup_asks(&mut first).await?;
        assert_eq!(newest_job_in(&mut first, kind.as_str()).await?, None);
        let other = database.clone();
        let second = tokio::spawn(async move {
            let mut second = other.begin().await?;
            AuthStorage::lock_backup_asks(&mut second).await?;
            let seen = newest_job_in(&mut second, JobKind::BackupNow.code()).await?;
            second.commit().await?;
            Ok::<_, StoreError>(seen)
        });
        tokio::time::sleep(WATCHED).await;
        assert!(!second.is_finished(), "the second ask did not wait");
        let id = JobId::generate(&mut ids())?;
        let job = NewJob {
            kind,
            payload: serde_json::json!({}),
            run_at: at(0),
            max_attempts: 1,
        };
        enqueue(&mut first, id, &job, at(0)).await?;
        first.commit().await?;
        let seen = second.await??.ok_or("the second ask saw no job")?;
        assert_eq!(seen.id, id);
        Ok(())
    })
    .await
}
