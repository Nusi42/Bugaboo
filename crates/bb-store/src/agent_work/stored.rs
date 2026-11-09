//! What the check after a load run reads of agent work (TEST-CONCURRENT).
use super::{Attempt, AuthStorage, IssueId, IssueWork, StoreError};

/// The most pieces of work on one issue the check reads: agents hold one each, a handful at most.
const MOST_WORK_ON_AN_ISSUE: u64 = 100;

impl crate::db::Database {
    /// The work of every agent on an issue with its attempts, oldest first: what the check after a
    /// load run reads (TEST-CONCURRENT), by the reads the use cases make.
    ///
    /// # Errors
    /// Fails when the database fails or a row is unreadable.
    pub async fn stored_agent_work_on(
        &self,
        issue: IssueId,
    ) -> Result<Vec<(IssueWork, Vec<Attempt>)>, StoreError> {
        use bb_app::agent_work::AgentWorkRecords as _;
        let mut transaction = self.begin_read().await?;
        let mut found = Vec::new();
        // Newest first by the moment of the hand-over, so the oldest first is the reverse.
        for work in
            AuthStorage::work_on_issue_in(&mut transaction, issue, None, MOST_WORK_ON_AN_ISSUE)
                .await?
                .into_iter()
                .rev()
        {
            let attempts = AuthStorage::attempts_of_in(&mut transaction, work.id).await?;
            found.push((work, attempts));
        }
        Ok(found)
    }
}
