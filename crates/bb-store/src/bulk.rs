//! The adapter of the jobs port (`bb_app::jobs::JobStore`) and of the port of the runs of changes
//! to many issues (`bb_app::bulk::BulkStore`): the queue of background jobs (ADR-0017) and the
//! record of how far a change to many issues is (ISS-09).
//!
//! The queue itself is `jobs.rs`; the run is a row that the change which queues it writes, and its
//! progress is saved outside a change, an issue at a time, with the number of issues looked at as
//! the check that two workers never count the same issue twice.

use bb_app::bulk::BulkStore;
use bb_core::bulk::{BulkItem, BulkRun, BulkSkip, BulkStatus};
use bb_core::id::{BulkRunId, UserId};
use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, Query, SelectStatement};

use crate::auth::AuthStorage;
use crate::db::{Row, StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::schema::BulkRunRow;

fn run_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            BulkRunRow::Id,
            BulkRunRow::Person,
            BulkRunRow::Action,
            BulkRunRow::Items,
            BulkRunRow::Status,
            BulkRunRow::Done,
            BulkRunRow::Changed,
            BulkRunRow::Skipped,
            BulkRunRow::CreatedAt,
            BulkRunRow::FinishedAt,
        ])
        .from(BulkRunRow::Table);
    select
}

fn read_run(row: &Row) -> Result<BulkRun, StoreError> {
    let items: Vec<BulkItem> = serde_json::from_str(&row.get::<String>(BulkRunRow::Items)?)?;
    let skipped: Vec<BulkSkip> = serde_json::from_str(&row.get::<String>(BulkRunRow::Skipped)?)?;
    Ok(BulkRun {
        id: row.id::<BulkRunId>(BulkRunRow::Id)?,
        person: row.id::<UserId>(BulkRunRow::Person)?,
        action: serde_json::from_str(&row.get::<String>(BulkRunRow::Action)?)?,
        items,
        status: BulkStatus::from_code(&row.get::<String>(BulkRunRow::Status)?)
            .ok_or(StoreError::Corrupt("the status of a run"))?,
        done: row.get(BulkRunRow::Done)?,
        changed: row.get(BulkRunRow::Changed)?,
        skipped,
        created_at: row.time(BulkRunRow::CreatedAt)?,
        finished_at: row.optional_time(BulkRunRow::FinishedAt)?,
    })
}

impl BulkStore for AuthStorage {
    async fn event_with_reason(&self, reason: &str, since: Timestamp) -> Result<bool, StoreError> {
        self.database().event_with_reason_since(reason, since).await
    }

    async fn bulk_run_in(
        transaction: &mut Transaction,
        id: BulkRunId,
    ) -> Result<Option<BulkRun>, StoreError> {
        let mut select = run_select();
        select.and_where(Expr::col(BulkRunRow::Id).eq(id.stored()));
        transaction
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_run)
            .transpose()
    }

    async fn save_bulk_progress(
        &self,
        run: &BulkRun,
        expected_done: i64,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(BulkRunRow::Table)
            .value(BulkRunRow::Status, run.status.code())
            .value(BulkRunRow::Done, run.done)
            .value(BulkRunRow::Changed, run.changed)
            .value(BulkRunRow::Skipped, serde_json::to_string(&run.skipped)?)
            .value(BulkRunRow::FinishedAt, run.finished_at.map(to_micros))
            .and_where(Expr::col(BulkRunRow::Id).eq(run.id.stored()))
            .and_where(Expr::col(BulkRunRow::Done).eq(expected_done));
        Ok(self.database().execute(&update).await? > 0)
    }

    async fn insert_bulk_run(
        transaction: &mut Transaction,
        run: &BulkRun,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(BulkRunRow::Table)
            .columns([
                BulkRunRow::Id,
                BulkRunRow::Person,
                BulkRunRow::Action,
                BulkRunRow::Items,
                BulkRunRow::Status,
                BulkRunRow::Done,
                BulkRunRow::Changed,
                BulkRunRow::Skipped,
                BulkRunRow::CreatedAt,
                BulkRunRow::FinishedAt,
            ])
            .values([
                Expr::val(run.id.stored()),
                Expr::val(run.person.stored()),
                Expr::val(serde_json::to_string(&run.action)?),
                Expr::val(serde_json::to_string(&run.items)?),
                Expr::val(run.status.code()),
                Expr::val(run.done),
                Expr::val(run.changed),
                Expr::val(serde_json::to_string(&run.skipped)?),
                Expr::val(to_micros(run.created_at)),
                Expr::val(run.finished_at.map(to_micros)),
            ])
            .map_err(|_| StoreError::Corrupt("a run"))?;
        transaction.execute(&insert).await.map(|_| ())
    }
}

#[cfg(test)]
#[path = "tests/bulk.rs"]
mod tests;
