//! The requests for agents, in the in-memory store.

use bb_core::agent_request::{AgentRequest, AskedAgent, Asker, RequestState};
use bb_core::id::{AgentRequestId, UserId};
use jiff::Timestamp;

use super::fake::{FakeError, FakeStore, Tx, remove_at_most};
use crate::agents::AgentRequestStore;

impl AgentRequestStore for FakeStore {
    async fn agent_request_in(
        tx: &mut Tx,
        id: AgentRequestId,
    ) -> Result<Option<AgentRequest>, FakeError> {
        Ok(tx
            .state
            .agent_requests
            .iter()
            .find(|request| request.id == id)
            .cloned())
    }

    async fn waiting_requests_of_in(
        tx: &mut Tx,
        asker: Asker,
        since: Timestamp,
    ) -> Result<i64, FakeError> {
        let count = tx
            .state
            .agent_requests
            .iter()
            .filter(|request| {
                request.state == RequestState::Waiting
                    && request.asked_at > since
                    && request.asker() == Some(asker)
            })
            .count();
        Ok(i64::try_from(count).unwrap_or(i64::MAX))
    }

    async fn agent_requests_of(
        &self,
        person: UserId,
        since: Timestamp,
        limit: u64,
    ) -> Result<Vec<AgentRequest>, FakeError> {
        Ok(self.read(|state| {
            let mut found: Vec<AgentRequest> = state
                .agent_requests
                .iter()
                .filter(|request| request.person == person && request.asked_at > since)
                .cloned()
                .collect();
            found.sort_by(|a, b| b.asked_at.cmp(&a.asked_at).then(b.id.cmp(&a.id)));
            found.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
            found
        }))
    }

    async fn insert_agent_request(tx: &mut Tx, request: &AgentRequest) -> Result<(), FakeError> {
        tx.state.agent_requests.push(request.clone());
        Ok(())
    }

    async fn answer_agent_request(
        tx: &mut Tx,
        id: AgentRequestId,
        from: RequestState,
        to: RequestState,
        at: Timestamp,
        agents: &[AskedAgent],
    ) -> Result<bool, FakeError> {
        let Some(found) = tx
            .state
            .agent_requests
            .iter_mut()
            .find(|request| request.id == id && request.state == from)
        else {
            return Ok(false);
        };
        found.state = to;
        if found.answered_at.is_none() {
            found.answered_at = Some(at);
        }
        agents.clone_into(&mut found.agents);
        Ok(true)
    }

    async fn delete_old_agent_requests(
        tx: &mut Tx,
        before: Timestamp,
        limit: u64,
    ) -> Result<u64, FakeError> {
        Ok(remove_at_most(
            &mut tx.state.agent_requests,
            limit,
            |request| request.asked_at < before,
        ))
    }
}
