//! What the check after a load run reads of agent work (TEST-CONCURRENT).
use super::*;

impl crate::db::Database {
    /// The work of every agent on an issue with its attempts, oldest first: what the check after a
    /// load run reads (TEST-CONCURRENT).
    ///
    /// # Errors
    /// Fails when the database fails or a row is unreadable.
    #[cfg(any(test, feature = "testing"))]
    pub async fn stored_agent_work_on(
        &self,
        issue: IssueId,
    ) -> Result<Vec<(IssueWork, Vec<Attempt>)>, StoreError> {
        let mut select = work_select();
        select
            .and_where(Expr::col(IssueWorkRow::Issue).eq(issue.stored()))
            .order_by(IssueWorkRow::MadeAt, Order::Asc)
            .order_by(IssueWorkRow::Id, Order::Asc);
        let mut found = Vec::new();
        for row in self.fetch_all(&select).await? {
            let work = read_work(&row)?;
            let mut attempts = attempt_select();
            attempts
                .and_where(Expr::col(AttemptRow::Work).eq(work.id.stored()))
                .order_by(AttemptRow::Number, Order::Asc);
            let attempts = self
                .fetch_all(&attempts)
                .await?
                .iter()
                .map(read_attempt)
                .collect::<Result<Vec<_>, _>>()?;
            found.push((work, attempts));
        }
        Ok(found)
    }
}
