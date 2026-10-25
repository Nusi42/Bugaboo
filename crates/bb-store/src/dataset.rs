//! What only the large dataset needs (architecture §11, *Large dataset*): writes that the use
//! cases never make. The dataset writes its rows through the same store functions as the use
//! cases, in big transactions; the one thing those functions cannot do is to say where a counter
//! stands after a million issues were written at once. The server does not build this module
//! (`dataset` is switched on by `bb-sample` only).

use bb_core::id::ProjectId;
use sea_query::{Expr, ExprTrait as _, OnConflict, Query};

use crate::db::{StoredId, Transaction};
use crate::error::StoreError;
use crate::schema::IssueCounterRow;

impl Transaction {
    /// Says that the project has given out the counting numbers up to `last`.
    ///
    /// # Errors
    /// The error of the database.
    pub async fn set_issue_counter(
        &mut self,
        project: ProjectId,
        last: i64,
    ) -> Result<(), StoreError> {
        let mut upsert = Query::insert();
        upsert
            .into_table(IssueCounterRow::Table)
            .columns([IssueCounterRow::Project, IssueCounterRow::Last])
            .values([Expr::val(project.stored()), Expr::val(last)])
            .map_err(|_| StoreError::Corrupt("a counter"))?
            .on_conflict(
                OnConflict::column(IssueCounterRow::Project)
                    .update_column(IssueCounterRow::Last)
                    .to_owned(),
            );
        self.execute(&upsert).await.map(|_| ())
    }
}

impl crate::db::Database {
    /// The number a project has given out up to, if it has given out any.
    ///
    /// # Errors
    /// The error of the database.
    pub async fn issue_counter(&self, project: ProjectId) -> Result<Option<i64>, StoreError> {
        let mut read = Query::select();
        read.column(IssueCounterRow::Last)
            .from(IssueCounterRow::Table)
            .and_where(Expr::col(IssueCounterRow::Project).eq(project.stored()));
        self.fetch_optional(&read)
            .await?
            .map(|row| row.get(IssueCounterRow::Last))
            .transpose()
    }
}
