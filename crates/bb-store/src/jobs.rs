//! The job queue (ADR-0017): jobs live in the `job` table, workers poll and take a lease.
//!
//! A worker takes jobs with [`Database::lease_jobs`]. The lease is one `UPDATE … RETURNING`
//! statement, so on SQLite (one writer at a time) and on PostgreSQL (with `FOR UPDATE SKIP
//! LOCKED`, so workers do not wait for each other) two workers never get the same job. A worker
//! that stops without telling anyone loses its lease when it runs out, and another worker takes
//! the job again. Every start counts as an attempt; failed jobs wait longer each time
//! ([`backoff`]) and, after the last attempt, go to the dead-letter list.

use bb_core::id::JobId;
use bb_core::job::{Job, JobState};
use bb_core::technical::TechnicalName;
use jiff::{SignedDuration, Timestamp};
use sea_query::{Expr, ExprTrait as _, LockBehavior, LockType, Order, Query};

use crate::db::{Database, Dialect, Row, StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::schema::{Computed, Job as Columns};

/// The failure code of a job whose last lease ran out without an answer.
const LEASE_EXPIRED: &str = "lease.expired";
/// How long the first retry waits; every further one waits twice as long.
const FIRST_RETRY: SignedDuration = SignedDuration::from_secs(30);
/// The longest wait between two attempts.
const LONGEST_RETRY: SignedDuration = SignedDuration::from_hours(1);

/// A job to put in the queue.
#[derive(Clone, Debug)]
pub struct NewJob {
    /// What kind of work it is.
    pub kind: TechnicalName,
    /// What the work needs to know.
    pub payload: serde_json::Value,
    /// The earliest moment it may run.
    pub run_at: Timestamp,
    /// After how many starts it goes to the dead-letter list.
    pub max_attempts: u32,
}

/// What happened to a job that a worker reported as failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureOutcome {
    /// It will run again at this moment.
    Retry(Timestamp),
    /// It failed too often and is on the dead-letter list.
    Dead,
}

/// How long a job waits after its `attempts`-th start failed: 30 seconds, doubled each time,
/// never more than an hour.
#[must_use]
pub fn backoff(attempts: u32) -> SignedDuration {
    let doublings = std::cmp::min(attempts.saturating_sub(1), 16);
    let wait = FIRST_RETRY
        .checked_mul(1_i32 << doublings)
        .unwrap_or(LONGEST_RETRY);
    std::cmp::min(wait, LONGEST_RETRY)
}

impl Database {
    /// Puts a job in the queue.
    ///
    /// # Errors
    /// Fails when the database fails.
    pub async fn enqueue_job(
        &self,
        id: JobId,
        job: &NewJob,
        now: Timestamp,
    ) -> Result<(), StoreError> {
        let mut transaction = self.begin().await?;
        enqueue(&mut transaction, id, job, now).await?;
        transaction.commit().await
    }

    /// Takes up to `limit` jobs that are due and not leased, for `worker`, until `now + lease`.
    /// Jobs whose last attempt ended without an answer go to the dead-letter list instead.
    ///
    /// # Errors
    /// Fails when the database fails.
    pub async fn lease_jobs(
        &self,
        worker: &TechnicalName,
        now: Timestamp,
        lease: SignedDuration,
        limit: u64,
    ) -> Result<Vec<Job>, StoreError> {
        let now_micros = to_micros(now);
        let lease_end = now
            .checked_add(lease)
            .map_err(|_| StoreError::Corrupt("a lease"))?;

        // A job that used all its attempts and whose lease ran out has no worker any more.
        let mut reap = Query::update();
        reap.table(Columns::Table)
            .value(Columns::State, JobState::Dead.code())
            .value(Columns::LeaseOwner, Option::<String>::None)
            .value(Columns::LeaseUntil, Option::<i64>::None)
            .value(Columns::LastFailure, LEASE_EXPIRED)
            .and_where(Expr::col(Columns::State).eq(JobState::Queued.code()))
            .and_where(Expr::col(Columns::LeaseUntil).lte(now_micros))
            .and_where(Expr::col(Columns::Attempts).gte(Expr::col(Columns::MaxAttempts)));
        self.execute(&reap).await?;

        let mut due = Query::select();
        due.column(Columns::Id)
            .from(Columns::Table)
            .and_where(Expr::col(Columns::State).eq(JobState::Queued.code()))
            .and_where(Expr::col(Columns::RunAt).lte(now_micros))
            .and_where(
                Expr::col(Columns::LeaseUntil)
                    .is_null()
                    .or(Expr::col(Columns::LeaseUntil).lte(now_micros)),
            )
            .and_where(Expr::col(Columns::Attempts).lt(Expr::col(Columns::MaxAttempts)))
            .order_by(Columns::RunAt, Order::Asc)
            .order_by(Columns::Id, Order::Asc)
            .limit(limit);
        if self.dialect() == Dialect::Postgres {
            // Dialect-specific on purpose (ADR-0017): PostgreSQL has many writers at once, and
            // `SKIP LOCKED` lets each worker take jobs the others have not locked. SQLite has one
            // writer at a time, so the single statement below is already exclusive.
            due.lock_with_behavior(LockType::Update, LockBehavior::SkipLocked);
        }
        let mut take = Query::update();
        take.table(Columns::Table)
            .value(Columns::LeaseOwner, worker.as_str())
            .value(Columns::LeaseUntil, to_micros(lease_end))
            .value(Columns::Attempts, Expr::col(Columns::Attempts).add(1_i64))
            .and_where(Expr::col(Columns::Id).in_subquery(due))
            .returning(Query::returning().columns([
                Columns::Id,
                Columns::Kind,
                Columns::Payload,
                Columns::RunAt,
                Columns::Attempts,
                Columns::MaxAttempts,
                Columns::State,
                Columns::LeaseOwner,
                Columns::LeaseUntil,
                Columns::LastFailure,
                Columns::CreatedAt,
            ]));
        let mut transaction = self.begin().await?;
        let rows = transaction.fetch_all(&take).await?;
        transaction.commit().await?;
        let mut jobs = rows.iter().map(read_job).collect::<Result<Vec<_>, _>>()?;
        jobs.sort_by_key(|job| (job.run_at, job.id));
        Ok(jobs)
    }

    /// Reports a job as done; it leaves the queue. Returns `false` when `worker` does not hold
    /// the lease any more (it ran out and the job went to someone else).
    ///
    /// # Errors
    /// Fails when the database fails.
    pub async fn complete_job(
        &self,
        id: JobId,
        worker: &TechnicalName,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(Columns::Table)
            .and_where(Expr::col(Columns::Id).eq(id.stored()))
            .and_where(Expr::col(Columns::LeaseOwner).eq(worker.as_str()));
        Ok(self.execute(&delete).await? > 0)
    }

    /// Reports a job as failed, with a technical code (never an error message). It waits
    /// [`backoff`] and runs again, or goes to the dead-letter list after its last attempt.
    /// Returns `None` when `worker` does not hold the lease.
    ///
    /// # Errors
    /// Fails when the database fails.
    pub async fn fail_job(
        &self,
        id: JobId,
        worker: &TechnicalName,
        failure: &TechnicalName,
        now: Timestamp,
    ) -> Result<Option<FailureOutcome>, StoreError> {
        let mut transaction = self.begin().await?;
        let mut select = Query::select();
        select
            .columns([Columns::Attempts, Columns::MaxAttempts])
            .from(Columns::Table)
            .and_where(Expr::col(Columns::Id).eq(id.stored()))
            .and_where(Expr::col(Columns::LeaseOwner).eq(worker.as_str()));
        let Some(row) = transaction.fetch_optional(&select).await? else {
            return Ok(None);
        };
        let attempts = count(&row, Columns::Attempts)?;
        let outcome = if attempts >= count(&row, Columns::MaxAttempts)? {
            FailureOutcome::Dead
        } else {
            let at = now
                .checked_add(backoff(attempts))
                .map_err(|_| StoreError::Corrupt("a retry time"))?;
            FailureOutcome::Retry(at)
        };
        let mut update = Query::update();
        update
            .table(Columns::Table)
            .value(Columns::LeaseOwner, Option::<String>::None)
            .value(Columns::LeaseUntil, Option::<i64>::None)
            .value(Columns::LastFailure, failure.as_str())
            .and_where(Expr::col(Columns::Id).eq(id.stored()));
        match outcome {
            FailureOutcome::Dead => {
                update.value(Columns::State, JobState::Dead.code());
            }
            FailureOutcome::Retry(at) => {
                update.value(Columns::RunAt, to_micros(at));
            }
        }
        transaction.execute(&update).await?;
        transaction.commit().await?;
        Ok(Some(outcome))
    }

    /// The jobs on the dead-letter list, oldest first.
    ///
    /// # Errors
    /// Fails when the database fails.
    pub async fn dead_jobs(&self, limit: u64) -> Result<Vec<Job>, StoreError> {
        let mut select = Query::select();
        select
            .columns(all_columns())
            .from(Columns::Table)
            .and_where(Expr::col(Columns::State).eq(JobState::Dead.code()))
            .order_by(Columns::CreatedAt, Order::Asc)
            .order_by(Columns::Id, Order::Asc)
            .limit(limit);
        self.fetch_all(&select)
            .await?
            .iter()
            .map(read_job)
            .collect()
    }

    /// Gives a dead job a fresh start (an administrator's decision): it runs again at `now`.
    /// Returns `false` when the job is not on the dead-letter list.
    ///
    /// # Errors
    /// Fails when the database fails.
    pub async fn requeue_dead_job(&self, id: JobId, now: Timestamp) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(Columns::Table)
            .value(Columns::State, JobState::Queued.code())
            .value(Columns::Attempts, 0_i64)
            .value(Columns::RunAt, to_micros(now))
            .value(Columns::LastFailure, Option::<String>::None)
            .and_where(Expr::col(Columns::Id).eq(id.stored()))
            .and_where(Expr::col(Columns::State).eq(JobState::Dead.code()));
        Ok(self.execute(&update).await? > 0)
    }

    /// How many jobs are in the queue, waiting or leased (not dead).
    ///
    /// # Errors
    /// Fails when the database fails.
    pub async fn queued_job_count(&self) -> Result<i64, StoreError> {
        let mut select = Query::select();
        select
            .expr_as(Expr::col(Columns::Id).count(), Computed::Total)
            .from(Columns::Table)
            .and_where(Expr::col(Columns::State).eq(JobState::Queued.code()));
        match self.fetch_optional(&select).await? {
            Some(row) => row.get(Computed::Total),
            None => Ok(0),
        }
    }
}

/// Puts a job in the queue inside an open transaction, so a use case can queue work in the same
/// transaction as its change.
pub(crate) async fn enqueue(
    transaction: &mut Transaction,
    id: JobId,
    job: &NewJob,
    now: Timestamp,
) -> Result<(), StoreError> {
    if job.max_attempts < 1 {
        return Err(StoreError::InvalidInput(
            "a job must be allowed to run at least once",
        ));
    }
    let mut insert = Query::insert();
    insert
        .into_table(Columns::Table)
        .columns(all_columns())
        .values([
            Expr::val(id.stored()),
            Expr::val(job.kind.as_str()),
            Expr::val(serde_json::to_string(&job.payload)?),
            Expr::val(to_micros(job.run_at)),
            Expr::val(0_i64),
            Expr::val(i64::from(job.max_attempts)),
            Expr::val(JobState::Queued.code()),
            Expr::val(Option::<String>::None),
            Expr::val(Option::<i64>::None),
            Expr::val(Option::<String>::None),
            Expr::val(to_micros(now)),
        ])
        .map_err(|_| StoreError::Corrupt("a job"))?;
    transaction.execute(&insert).await?;
    Ok(())
}

fn all_columns() -> [Columns; 11] {
    [
        Columns::Id,
        Columns::Kind,
        Columns::Payload,
        Columns::RunAt,
        Columns::Attempts,
        Columns::MaxAttempts,
        Columns::State,
        Columns::LeaseOwner,
        Columns::LeaseUntil,
        Columns::LastFailure,
        Columns::CreatedAt,
    ]
}

fn count(row: &Row, column: Columns) -> Result<u32, StoreError> {
    u32::try_from(row.get::<i64>(column)?).map_err(|_| StoreError::Corrupt("a count"))
}

fn technical(row: &Row, column: Columns) -> Result<Option<TechnicalName>, StoreError> {
    row.get::<Option<String>>(column)?
        .map(|name| TechnicalName::new(&name).map_err(|_| StoreError::Corrupt("a name")))
        .transpose()
}

fn read_job(row: &Row) -> Result<Job, StoreError> {
    Ok(Job {
        id: row.id(Columns::Id)?,
        kind: TechnicalName::new(&row.get::<String>(Columns::Kind)?)
            .map_err(|_| StoreError::Corrupt("a job kind"))?,
        payload: serde_json::from_str(&row.get::<String>(Columns::Payload)?)?,
        run_at: row.time(Columns::RunAt)?,
        attempts: count(row, Columns::Attempts)?,
        max_attempts: count(row, Columns::MaxAttempts)?,
        state: JobState::from_code(&row.get::<String>(Columns::State)?)
            .ok_or(StoreError::Corrupt("a job state"))?,
        lease_owner: technical(row, Columns::LeaseOwner)?,
        lease_until: row.optional_time(Columns::LeaseUntil)?,
        last_failure: technical(row, Columns::LastFailure)?,
        created_at: row.time(Columns::CreatedAt)?,
    })
}

#[cfg(test)]
mod tests {
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

            let leased = database.lease_jobs(&worker, at(1), LEASE, 5).await?;
            assert_eq!(leased.len(), 1);
            assert_eq!(leased[0].id, id);
            assert_eq!(leased[0].attempts, 1);
            assert_eq!(leased[0].lease_owner.as_ref(), Some(&worker));
            assert_eq!(leased[0].payload, serde_json::json!({"to": "someone"}));
            assert!(
                database
                    .lease_jobs(&name("worker-2"), at(30), LEASE, 5)
                    .await?
                    .is_empty()
            );

            let again = database
                .lease_jobs(&name("worker-2"), at(70), LEASE, 5)
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
    async fn a_job_waits_for_its_time() -> TestResult {
        each_database("adr_0017_run_at", async |db| {
            let database = open(db).await?;
            let id = JobId::generate(&mut ids())?;
            database.enqueue_job(id, &mail(at(100), 3), at(0)).await?;
            let worker = name("worker-1");
            assert!(
                database
                    .lease_jobs(&worker, at(99), LEASE, 5)
                    .await?
                    .is_empty()
            );
            assert_eq!(
                database.lease_jobs(&worker, at(100), LEASE, 5).await?.len(),
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

            database.lease_jobs(&worker, at(0), LEASE, 1).await?;
            let first = database.fail_job(id, &worker, &failure, at(1)).await?;
            assert_eq!(first, Some(FailureOutcome::Retry(at(31))));
            assert!(
                database
                    .lease_jobs(&worker, at(30), LEASE, 1)
                    .await?
                    .is_empty()
            );

            assert_eq!(
                database.lease_jobs(&worker, at(31), LEASE, 1).await?.len(),
                1
            );
            let second = database.fail_job(id, &worker, &failure, at(32)).await?;
            assert_eq!(second, Some(FailureOutcome::Dead));
            assert_eq!(database.queued_job_count().await?, 0);
            let dead = database.dead_jobs(10).await?;
            assert_eq!(dead.len(), 1);
            assert_eq!(dead[0].last_failure.as_ref(), Some(&failure));
            assert_eq!(dead[0].state, JobState::Dead);
            assert!(
                database
                    .lease_jobs(&worker, at(10_000), LEASE, 1)
                    .await?
                    .is_empty()
            );

            assert!(database.requeue_dead_job(id, at(100)).await?);
            assert!(!database.requeue_dead_job(id, at(100)).await?);
            let revived = database.lease_jobs(&worker, at(100), LEASE, 1).await?;
            assert_eq!(revived[0].attempts, 1);
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
                .lease_jobs(&name("worker-1"), at(0), LEASE, 1)
                .await?;
            let other = name("worker-2");
            assert!(!database.complete_job(id, &other).await?);
            let outcome = database.fail_job(id, &other, &name("x.y"), at(1)).await?;
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
                database.lease_jobs(&worker, at(0), LEASE, 1).await?.len(),
                1
            );
            // The worker never answers.
            assert!(
                database
                    .lease_jobs(&worker, at(61), LEASE, 1)
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
                        let batch = database.lease_jobs(&worker, at(1), LEASE, 3).await?;
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
}
