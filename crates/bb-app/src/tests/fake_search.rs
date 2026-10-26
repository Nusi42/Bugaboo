//! The search part of the in-memory store: it does not search. It keeps the requests the use case
//! made, so a test sees exactly what the person's rights turned into, and answers what the test
//! put there. The compiler itself is tested on both databases in `bb-store`.

use super::fake::FakeStore;
use crate::search::{GroupCount, SearchFailure, SearchHit, SearchRequest, SearchStore};

impl SearchStore for FakeStore {
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
        self.lock().searches.push(request.clone());
        Ok(Vec::new())
    }
}
