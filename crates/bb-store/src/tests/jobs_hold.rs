//! Holding the lease of a long job on both databases (ADR-0017): the worker that holds it keeps it
//! longer; another worker, and a job that is done, get `false`.

use bb_testing::{TestResult, each_database};
use jiff::SignedDuration;

use super::*;
use crate::test_support::{at, ids, open};

fn name(text: &str) -> TechnicalName {
    TechnicalName::new(text).unwrap()
}

#[tokio::test]
async fn adr_0017_only_the_worker_that_holds_a_job_keeps_it_longer() -> TestResult {
    each_database("adr_0017_hold", async |db| {
        let database = open(db).await?;
        let id = JobId::generate(&mut ids())?;
        let job = NewJob {
            kind: name("privacy.export"),
            payload: serde_json::json!({}),
            run_at: at(0),
            max_attempts: 3,
        };
        database.enqueue_job(id, &job, at(0)).await?;
        let (holder, other) = (name("worker-1"), name("worker-2"));
        let lease = SignedDuration::from_secs(60);
        assert_eq!(
            database.lease_jobs(&holder, at(1), lease, 5).await?.len(),
            1
        );
        // The holder renews: past the first lease, nobody else may take the job.
        assert!(database.hold_job(id, &holder, at(200)).await?);
        assert!(
            database
                .lease_jobs(&other, at(100), lease, 5)
                .await?
                .is_empty()
        );
        // Another worker cannot renew a lease it does not hold.
        assert!(!database.hold_job(id, &other, at(300)).await?);
        // A job that is done is held by nobody.
        assert!(database.complete_job(id, &holder).await?);
        assert!(!database.hold_job(id, &holder, at(300)).await?);
        Ok(())
    })
    .await
}
