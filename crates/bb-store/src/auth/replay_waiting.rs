//! The lines of the replay list that wait for its file (ADR-0044, decision 8): the adapter of
//! `bb_app::privacy::replay::ReplayStore`.

use bb_app::privacy::replay::ReplayStore;
use bb_core::backup::replay::ReplayLine;
use sea_query::{Expr, ExprTrait as _, OnConflict, Order, Query};

use super::AuthStorage;
use crate::db::{Database, STATEMENT_ITEMS, Transaction};
use crate::error::StoreError;
use crate::schema::ReplayWaitingRow;

/// Every line that waits in `database` for the file, oldest first: for the server, and for a
/// restore, which reads them before it replaces the database.
pub(crate) async fn waiting_lines(database: &Database) -> Result<Vec<String>, StoreError> {
    let mut select = Query::select();
    select
        .column(ReplayWaitingRow::Line)
        .from(ReplayWaitingRow::Table)
        .order_by(ReplayWaitingRow::Line, Order::Asc);
    database
        .fetch_all(&select)
        .await?
        .iter()
        .map(|row| row.get(ReplayWaitingRow::Line))
        .collect()
}

impl ReplayStore for AuthStorage {
    async fn wait_to_replay(
        transaction: &mut Transaction,
        lines: &[ReplayLine],
    ) -> Result<(), StoreError> {
        for line in lines {
            let mut insert = Query::insert();
            insert
                .into_table(ReplayWaitingRow::Table)
                .columns([ReplayWaitingRow::Line])
                .values([Expr::val(line.to_string())])
                .map_err(|_| StoreError::Corrupt("a line of the replay list"))?
                .on_conflict(
                    OnConflict::column(ReplayWaitingRow::Line)
                        .do_nothing()
                        .to_owned(),
                );
            transaction.execute(&insert).await?;
        }
        Ok(())
    }

    async fn waiting_replay_lines(&self) -> Result<Vec<String>, StoreError> {
        waiting_lines(self.database()).await
    }

    async fn replay_lines_written(&self, lines: &[String]) -> Result<(), StoreError> {
        let mut transaction = self.database().begin().await?;
        for chunk in lines.chunks(STATEMENT_ITEMS) {
            let mut delete = Query::delete();
            delete
                .from_table(ReplayWaitingRow::Table)
                .and_where(Expr::col(ReplayWaitingRow::Line).is_in(chunk.to_vec()));
            transaction.execute(&delete).await?;
        }
        transaction.commit().await
    }
}

#[cfg(test)]
#[path = "../tests/replay_waiting.rs"]
mod tests;
