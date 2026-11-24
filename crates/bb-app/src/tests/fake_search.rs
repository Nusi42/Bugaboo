//! The search part of the in-memory store: it does not search. It keeps the requests the use case
//! made, so a test sees exactly what the person's rights turned into, and answers what the test
//! put there. The compiler itself is tested on both databases in `bb-store`.

use bb_core::id::IssueId;

use super::fake::FakeStore;
use super::fake_offers::reads;
use crate::search::{
    GroupCount, LengthTotals, MapStep, SearchFailure, SearchHit, SearchRequest, SearchStore,
};

impl SearchStore for FakeStore {
    /// The step walks the relations and parents in memory, through the issues the sight of the
    /// request reads, as the statement of the store does on both databases.
    async fn map_step(
        &self,
        request: &SearchRequest,
        step: &MapStep,
    ) -> Result<Vec<IssueId>, SearchFailure> {
        let shared = self.lock();
        let state = &shared.state;
        let far = |one: IssueId, other: IssueId| {
            if step.near.contains(&one) {
                Some(other)
            } else if step.near.contains(&other) {
                Some(one)
            } else {
                None
            }
        };
        let relations = state
            .relations
            .iter()
            .filter(|relation| step.kinds.contains(&relation.kind))
            .filter_map(|relation| far(relation.from, relation.to));
        let parents = state
            .parents
            .iter()
            .filter(|_| step.part_of)
            .filter_map(|row| far(row.child, row.parent));
        let mut found: Vec<IssueId> = relations
            .chain(parents)
            .filter(|issue| !step.known.contains(issue))
            .filter(|issue| {
                state
                    .issues
                    .iter()
                    .any(|each| each.id == *issue && reads(&request.sight, each))
            })
            .collect();
        found.sort();
        found.dedup();
        found.truncate(usize::try_from(step.most).unwrap_or(usize::MAX));
        Ok(found)
    }

    async fn search_page(&self, request: &SearchRequest) -> Result<Vec<SearchHit>, SearchFailure> {
        let mut shared = self.lock();
        shared.searches.push(request.clone());
        if let Some(failure) = shared.search_fails.clone() {
            return Err(failure);
        }
        let limit = usize::try_from(request.limit).unwrap_or(usize::MAX);
        Ok(shared.search_hits.iter().take(limit).cloned().collect())
    }

    async fn search_count(&self, request: &SearchRequest) -> Result<u64, SearchFailure> {
        let mut shared = self.lock();
        shared.searches.push(request.clone());
        Ok(u64::try_from(shared.search_hits.len()).unwrap_or(u64::MAX))
    }

    async fn search_groups(
        &self,
        request: &SearchRequest,
        _by: &str,
    ) -> Result<Vec<GroupCount>, SearchFailure> {
        let mut shared = self.lock();
        shared.searches.push(request.clone());
        Ok(shared.search_groups.clone())
    }

    async fn search_lengths(&self, request: &SearchRequest) -> Result<LengthTotals, SearchFailure> {
        let mut shared = self.lock();
        shared.searches.push(request.clone());
        Ok(shared.search_lengths)
    }
}
