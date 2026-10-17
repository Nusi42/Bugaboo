//! The adapter of the jobs port (`bb_app::jobs::JobStore`) and of the port of the runs of changes
//! to many issues (`bb_app::many_changes::ManyChangeStore`): the queue of background jobs (ADR-0017) and the
//! record of how far a change to many issues is (ISS-09).
//!
//! The queue itself is `jobs.rs`; the run is a row that the change which queues it writes, and its
//! progress is saved outside a change, an issue at a time, with the number of issues looked at as
//! the check that two workers never count the same issue twice.

use bb_app::many_changes::ManyChangeStore;
use bb_core::id::{ManyChangeRunId, UserId};
use bb_core::many_changes::{
    ManyChangeItem, ManyChangeRun, ManyChangeSkip, ManyChangeStatus, RunsToEnd,
};
use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, Query, SelectStatement};

use crate::auth::AuthStorage;
use crate::db::{Row, StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::schema::ManyChangeRunRow;

/// The statuses of a run that is not over.
const NOT_OVER: [ManyChangeStatus; 2] = [ManyChangeStatus::Queued, ManyChangeStatus::Running];

fn run_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            ManyChangeRunRow::Id,
            ManyChangeRunRow::Person,
            ManyChangeRunRow::Action,
            ManyChangeRunRow::Items,
            ManyChangeRunRow::Status,
            ManyChangeRunRow::Done,
            ManyChangeRunRow::Changed,
            ManyChangeRunRow::Skipped,
            ManyChangeRunRow::CreatedAt,
            ManyChangeRunRow::FinishedAt,
        ])
        .from(ManyChangeRunRow::Table);
    select
}

fn read_run(row: &Row) -> Result<ManyChangeRun, StoreError> {
    let items: Vec<ManyChangeItem> =
        serde_json::from_str(&row.get::<String>(ManyChangeRunRow::Items)?)?;
    let skipped: Vec<ManyChangeSkip> =
        serde_json::from_str(&row.get::<String>(ManyChangeRunRow::Skipped)?)?;
    Ok(ManyChangeRun {
        id: row.id::<ManyChangeRunId>(ManyChangeRunRow::Id)?,
        person: row.id::<UserId>(ManyChangeRunRow::Person)?,
        action: serde_json::from_str(&row.get::<String>(ManyChangeRunRow::Action)?)?,
        items,
        status: ManyChangeStatus::from_code(&row.get::<String>(ManyChangeRunRow::Status)?)
            .ok_or(StoreError::Corrupt("the status of a run"))?,
        done: row.get(ManyChangeRunRow::Done)?,
        changed: row.get(ManyChangeRunRow::Changed)?,
        skipped,
        created_at: row.time(ManyChangeRunRow::CreatedAt)?,
        finished_at: row.optional_time(ManyChangeRunRow::FinishedAt)?,
    })
}

impl ManyChangeStore for AuthStorage {
    async fn event_with_reason(&self, reason: &str, since: Timestamp) -> Result<bool, StoreError> {
        self.database().event_with_reason_since(reason, since).await
    }

    async fn many_changes_run_in(
        transaction: &mut Transaction,
        id: ManyChangeRunId,
    ) -> Result<Option<ManyChangeRun>, StoreError> {
        let mut select = run_select();
        select.and_where(Expr::col(ManyChangeRunRow::Id).eq(id.stored()));
        transaction
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_run)
            .transpose()
    }

    async fn save_many_changes_progress(
        &self,
        run: &ManyChangeRun,
        expected_done: i64,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(ManyChangeRunRow::Table)
            .value(ManyChangeRunRow::Status, run.status.code())
            .value(ManyChangeRunRow::Done, run.done)
            .value(ManyChangeRunRow::Changed, run.changed)
            .value(
                ManyChangeRunRow::Skipped,
                serde_json::to_string(&run.skipped)?,
            )
            .value(ManyChangeRunRow::FinishedAt, run.finished_at.map(to_micros))
            .and_where(Expr::col(ManyChangeRunRow::Id).eq(run.id.stored()))
            .and_where(Expr::col(ManyChangeRunRow::Done).eq(expected_done))
            .and_where(
                Expr::col(ManyChangeRunRow::Status).is_in(NOT_OVER.map(ManyChangeStatus::code)),
            );
        Ok(self.database().execute(&update).await? > 0)
    }

    async fn many_changes_run_is_over(&self, id: ManyChangeRunId) -> Result<bool, StoreError> {
        let mut select = Query::select();
        select
            .column(ManyChangeRunRow::Id)
            .from(ManyChangeRunRow::Table)
            .and_where(Expr::col(ManyChangeRunRow::Id).eq(id.stored()))
            .and_where(
                Expr::col(ManyChangeRunRow::Status).is_in(NOT_OVER.map(ManyChangeStatus::code)),
            );
        // A run that is not there is as good as over.
        Ok(self.database().fetch_optional(&select).await?.is_none())
    }

    async fn end_many_changes_runs(
        transaction: &mut Transaction,
        which: &RunsToEnd,
        status: ManyChangeStatus,
        at: Timestamp,
    ) -> Result<u64, StoreError> {
        let mut update = Query::update();
        update
            .table(ManyChangeRunRow::Table)
            .value(ManyChangeRunRow::Status, status.code())
            .value(ManyChangeRunRow::FinishedAt, to_micros(at))
            .and_where(
                Expr::col(ManyChangeRunRow::Status).is_in(NOT_OVER.map(ManyChangeStatus::code)),
            );
        match which {
            RunsToEnd::One(id) => update.and_where(Expr::col(ManyChangeRunRow::Id).eq(id.stored())),
            RunsToEnd::OfPerson(person) => {
                update.and_where(Expr::col(ManyChangeRunRow::Person).eq(person.stored()))
            }
        };
        transaction.execute(&update).await
    }

    async fn insert_many_changes_run(
        transaction: &mut Transaction,
        run: &ManyChangeRun,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(ManyChangeRunRow::Table)
            .columns([
                ManyChangeRunRow::Id,
                ManyChangeRunRow::Person,
                ManyChangeRunRow::Action,
                ManyChangeRunRow::Items,
                ManyChangeRunRow::Status,
                ManyChangeRunRow::Done,
                ManyChangeRunRow::Changed,
                ManyChangeRunRow::Skipped,
                ManyChangeRunRow::CreatedAt,
                ManyChangeRunRow::FinishedAt,
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
#[path = "tests/many_changes.rs"]
mod tests;
