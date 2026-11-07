//! The agent part of the in-memory store (AI-30).

use bb_core::agent::{Agent, AgentState};
use bb_core::id::{AccessKeyId, AgentId, UserId};

use super::fake::{FakeError, FakeStore, Tx};
use crate::agents::{AgentStore, AgentWork, AgentWorkStore};
use crate::auth::AccessKeyRecord;

impl AgentWorkStore for FakeStore {
    async fn agent_work(&self, agent: AgentId) -> Result<AgentWork, FakeError> {
        Ok(self.read(|state| {
            state
                .agent_work
                .iter()
                .find(|(id, _)| *id == agent)
                .map(|(_, work)| work.clone())
                .unwrap_or_default()
        }))
    }
}

/// Counts the version of the agent up if it is at `version`.
fn change(tx: &mut Tx, id: AgentId, version: i64, set: impl FnOnce(&mut Agent)) -> bool {
    match tx
        .state
        .agents
        .iter_mut()
        .find(|agent| agent.id == id && agent.version == version)
    {
        Some(agent) => {
            set(agent);
            agent.version += 1;
            true
        }
        None => false,
    }
}

impl AgentStore for FakeStore {
    async fn agent_in(tx: &mut Tx, id: AgentId) -> Result<Option<Agent>, FakeError> {
        Ok(tx.state.agents.iter().find(|agent| agent.id == id).cloned())
    }

    async fn agent_count_of_in(tx: &mut Tx, person: UserId) -> Result<i64, FakeError> {
        let held = tx
            .state
            .agents
            .iter()
            .filter(|a| a.person == person)
            .count();
        Ok(i64::try_from(held).unwrap_or(i64::MAX))
    }

    async fn agents_of(
        &self,
        person: UserId,
        after: Option<AgentId>,
        limit: u64,
    ) -> Result<Vec<Agent>, FakeError> {
        Ok(self.read(|state| page(&state.agents, |a| a.person == person, after, limit)))
    }

    async fn all_agents(
        &self,
        after: Option<AgentId>,
        limit: u64,
    ) -> Result<Vec<Agent>, FakeError> {
        Ok(self.read(|state| page(&state.agents, |_| true, after, limit)))
    }

    async fn keys_of_agent(&self, agent: AgentId) -> Result<Vec<AccessKeyRecord>, FakeError> {
        Ok(self.read(|state| {
            state
                .access_keys
                .iter()
                .filter(|record| record.key.agent == Some(agent))
                .cloned()
                .collect()
        }))
    }

    async fn agents_offered_to(&self, person: UserId) -> Result<Vec<Agent>, FakeError> {
        Ok(self.read(|state| page(&state.agents, |a| a.offered_to == Some(person), None, 100)))
    }

    async fn lock_agents_of(_tx: &mut Tx, _person: UserId) -> Result<(), FakeError> {
        Ok(())
    }

    async fn insert_agent(tx: &mut Tx, agent: &Agent) -> Result<(), FakeError> {
        let taken = tx
            .state
            .agents
            .iter()
            .any(|held| held.person == agent.person && held.name == agent.name);
        if taken {
            return Err(FakeError::Duplicate);
        }
        tx.state.agents.push(agent.clone());
        Ok(())
    }

    async fn rename_agent(
        tx: &mut Tx,
        id: AgentId,
        name: &str,
        version: i64,
    ) -> Result<bool, FakeError> {
        Ok(change(tx, id, version, |agent| {
            name.clone_into(&mut agent.name);
        }))
    }

    async fn set_agent_state(
        tx: &mut Tx,
        id: AgentId,
        state: AgentState,
        version: i64,
    ) -> Result<bool, FakeError> {
        Ok(change(tx, id, version, |agent| agent.state = state))
    }

    async fn offer_agent(
        tx: &mut Tx,
        id: AgentId,
        to: Option<UserId>,
        version: i64,
    ) -> Result<bool, FakeError> {
        Ok(change(tx, id, version, |agent| agent.offered_to = to))
    }

    async fn hand_agent_to(
        tx: &mut Tx,
        id: AgentId,
        person: UserId,
        version: i64,
    ) -> Result<bool, FakeError> {
        Ok(change(tx, id, version, |agent| {
            agent.person = person;
            agent.offered_to = None;
        }))
    }

    async fn stop_agents_of(tx: &mut Tx, person: UserId) -> Result<Vec<AgentId>, FakeError> {
        let mut stopped = Vec::new();
        for agent in &mut tx.state.agents {
            if agent.person == person && agent.state != AgentState::Stopped {
                agent.state = AgentState::Stopped;
                agent.version += 1;
                stopped.push(agent.id);
            }
        }
        stopped.sort();
        Ok(stopped)
    }

    async fn delete_keys_of_agent(
        tx: &mut Tx,
        agent: AgentId,
    ) -> Result<Vec<AccessKeyId>, FakeError> {
        let gone: Vec<AccessKeyId> = tx
            .state
            .access_keys
            .iter()
            .filter(|record| record.key.agent == Some(agent))
            .map(|record| record.key.id)
            .collect();
        tx.state
            .access_keys
            .retain(|record| record.key.agent != Some(agent));
        Ok(gone)
    }
}

fn page(
    agents: &[Agent],
    wanted: impl Fn(&Agent) -> bool,
    after: Option<AgentId>,
    limit: u64,
) -> Vec<Agent> {
    let mut found: Vec<Agent> = agents.iter().filter(|a| wanted(a)).cloned().collect();
    found.sort_by_key(|agent| agent.id);
    found
        .into_iter()
        .filter(|agent| after.is_none_or(|after| agent.id > after))
        .take(usize::try_from(limit).unwrap_or(usize::MAX))
        .collect()
}
