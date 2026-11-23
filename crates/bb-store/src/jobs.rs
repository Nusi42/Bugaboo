//! The job queue (ADR-0017): jobs live in the `job` table, workers poll and take a lease.
//!
//! A worker takes jobs with [`Database::lease_jobs`]. The lease is one `UPDATE … RETURNING`
//! statement, so on SQLite (one writer at a time) and on PostgreSQL (with `FOR UPDATE SKIP
//! LOCKED`, so workers do not wait for each other) two workers never get the same job. A worker
//! that stops without telling anyone loses its lease when it runs out, and another worker takes
//! the job again. Every start counts as an attempt; failed jobs wait longer each time
//! ([`backoff`]) and, after the last attempt, go to the dead-letter list.

use bb_app::jobs::{JOB_ATTEMPTS_MAX, JobKind, JobStore, LaneKinds};
use bb_core::id::JobId;
use bb_core::job::{FailureKind, FailureOutcome, Job, JobState};
use bb_core::technical::TechnicalName;
use jiff::{SignedDuration, Timestamp};
use sea_query::{Expr, ExprTrait as _, LockBehavior, LockType, Order, Query};
use serde_json::Value;

use crate::auth::AuthStorage;
use crate::db::{Database, Dialect, Row, StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::keeping::delete_limited;
use crate::keeping::past::past_dead_jobs;
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
    pub payload: Value,
    /// The earliest moment it may run.
    pub run_at: Timestamp,
    /// After how many starts it goes to the dead-letter list.
    pub max_attempts: u32,
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

    /// Takes up to `limit` jobs of `kinds` that are due and not leased, for `worker`, until
    /// `now + lease`. Jobs whose last attempt ended without an answer go to the dead-letter list
    /// instead.
    ///
    /// # Errors
    /// Fails when the database fails.
    pub async fn lease_jobs(
        &self,
        worker: &TechnicalName,
        now: Timestamp,
        lease: SignedDuration,
        limit: u64,
        kinds: &LaneKinds,
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
            .and_where(Expr::col(Columns::Attempts).lt(Expr::col(Columns::MaxAttempts)));
        match kinds {
            LaneKinds::Only(names) => {
                due.and_where(
                    Expr::col(Columns::Kind).is_in(names.iter().map(TechnicalName::as_str)),
                );
            }
            LaneKinds::AllBut(names) if !names.is_empty() => {
                due.and_where(
                    Expr::col(Columns::Kind).is_not_in(names.iter().map(TechnicalName::as_str)),
                );
            }
            LaneKinds::AllBut(_) => {}
        }
        due.order_by(Columns::RunAt, Order::Asc)
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
            .returning(Query::returning().columns(JOB_COLUMNS));
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

    /// Keeps a job leased to `worker` until `until`, for a job that works longer than its lease.
    /// Returns `false` when `worker` does not hold the lease any more.
    ///
    /// # Errors
    /// Fails when the database fails.
    pub async fn hold_job(
        &self,
        id: JobId,
        worker: &TechnicalName,
        until: Timestamp,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(Columns::Table)
            .value(Columns::LeaseUntil, to_micros(until))
            .and_where(Expr::col(Columns::Id).eq(id.stored()))
            .and_where(Expr::col(Columns::LeaseOwner).eq(worker.as_str()));
        Ok(self.execute(&update).await? > 0)
    }

    /// Reports a job as failed, with a technical code (never an error message). It waits
    /// [`backoff`] and runs again, or goes to the dead-letter list after its last attempt, or at
    /// once when the failure is [`FailureKind::Permanent`]. Returns `None` when `worker` does not
    /// hold the lease.
    ///
    /// # Errors
    /// Fails when the database fails.
    pub async fn fail_job(
        &self,
        id: JobId,
        worker: &TechnicalName,
        failure: &TechnicalName,
        kind: FailureKind,
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
        let outcome =
            if kind == FailureKind::Permanent || attempts >= count(&row, Columns::MaxAttempts)? {
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

    /// Deletes at most `limit` of the jobs on the dead-letter list whose last start was before
    /// `before`, with their payload. Returns how many went.
    ///
    /// # Errors
    /// Fails when the database fails.
    pub async fn purge_dead_jobs(&self, before: Timestamp, limit: u64) -> Result<u64, StoreError> {
        let delete = delete_limited(
            Columns::Table,
            &[Columns::Id],
            past_dead_jobs(before),
            limit,
        );
        self.execute(&delete).await
    }

    /// The job of this kind that was queued last, waiting, worked on or dead.
    ///
    /// # Errors
    /// Fails when the database fails.
    pub async fn newest_job(&self, kind: &TechnicalName) -> Result<Option<Job>, StoreError> {
        let mut transaction = self.begin_read().await?;
        let found = newest_job_in(&mut transaction, kind.as_str()).await?;
        transaction.commit().await?;
        Ok(found)
    }

    /// Whether a job of this kind waits in the queue (not dead) for a time after `after`.
    ///
    /// # Errors
    /// Fails when the database fails.
    pub async fn later_job_waits(
        &self,
        kind: &TechnicalName,
        after: Timestamp,
    ) -> Result<bool, StoreError> {
        let mut select = Query::select();
        select
            .expr_as(Expr::col(Columns::Id).count(), Computed::Total)
            .from(Columns::Table)
            .and_where(Expr::col(Columns::Kind).eq(kind.as_str()))
            .and_where(Expr::col(Columns::State).eq(JobState::Queued.code()))
            .and_where(Expr::col(Columns::RunAt).gt(to_micros(after)));
        match self.fetch_optional(&select).await? {
            Some(row) => Ok(row.get::<i64>(Computed::Total)? > 0),
            None => Ok(false),
        }
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

/// The job of this kind that was queued last, inside `transaction`.
async fn newest_job_in(
    transaction: &mut Transaction,
    kind: &str,
) -> Result<Option<Job>, StoreError> {
    let mut select = Query::select();
    select
        .columns(JOB_COLUMNS)
        .from(Columns::Table)
        .and_where(Expr::col(Columns::Kind).eq(kind))
        .order_by(Columns::CreatedAt, Order::Desc)
        .order_by(Columns::Id, Order::Desc)
        .limit(1);
    transaction
        .fetch_optional(&select)
        .await?
        .as_ref()
        .map(read_job)
        .transpose()
}

/// The columns [`read_job`] reads.
const JOB_COLUMNS: [Columns; 11] = [
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
];

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

fn new_job(kind: &TechnicalName, payload: &Value, run_at: Timestamp) -> NewJob {
    NewJob {
        kind: kind.clone(),
        payload: payload.clone(),
        run_at,
        max_attempts: JOB_ATTEMPTS_MAX,
    }
}

impl JobStore for AuthStorage {
    async fn lease_jobs(
        &self,
        worker: &TechnicalName,
        now: Timestamp,
        lease: SignedDuration,
        limit: u64,
        kinds: &LaneKinds,
    ) -> Result<Vec<Job>, StoreError> {
        self.database()
            .lease_jobs(worker, now, lease, limit, kinds)
            .await
    }

    async fn complete_job(&self, id: JobId, worker: &TechnicalName) -> Result<bool, StoreError> {
        self.database().complete_job(id, worker).await
    }

    async fn hold_job(
        &self,
        id: JobId,
        worker: &TechnicalName,
        until: Timestamp,
    ) -> Result<bool, StoreError> {
        self.database().hold_job(id, worker, until).await
    }

    async fn fail_job(
        &self,
        id: JobId,
        worker: &TechnicalName,
        failure: &TechnicalName,
        kind: FailureKind,
        now: Timestamp,
    ) -> Result<Option<FailureOutcome>, StoreError> {
        self.database()
            .fail_job(id, worker, failure, kind, now)
            .await
    }

    async fn later_job_waits(
        &self,
        kind: &TechnicalName,
        after: Timestamp,
    ) -> Result<bool, StoreError> {
        self.database().later_job_waits(kind, after).await
    }

    async fn newest_job_in(
        transaction: &mut Transaction,
        kind: JobKind,
    ) -> Result<Option<Job>, StoreError> {
        newest_job_in(transaction, kind.code()).await
    }

    async fn purge_dead_jobs(&self, before: Timestamp, limit: u64) -> Result<u64, StoreError> {
        self.database().purge_dead_jobs(before, limit).await
    }

    async fn queue_job(
        &self,
        id: JobId,
        kind: &TechnicalName,
        payload: &Value,
        run_at: Timestamp,
        now: Timestamp,
    ) -> Result<(), StoreError> {
        self.database()
            .enqueue_job(id, &new_job(kind, payload, run_at), now)
            .await
    }

    async fn enqueue_job(
        transaction: &mut Transaction,
        id: JobId,
        kind: &TechnicalName,
        payload: &Value,
        run_at: Timestamp,
        now: Timestamp,
    ) -> Result<(), StoreError> {
        enqueue(transaction, id, &new_job(kind, payload, run_at), now).await
    }
}

#[cfg(test)]
#[path = "tests/jobs_queue.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/jobs_hold.rs"]
mod hold_tests;
