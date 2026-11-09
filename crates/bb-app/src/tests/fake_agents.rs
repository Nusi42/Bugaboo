//! The agent part of the in-memory store (AI-30).

use std::collections::{BTreeMap, BTreeSet};

use bb_core::agent::{Agent, AgentLimits, AgentState};
use bb_core::agent_work::Allowance;
use bb_core::id::{AccessKeyId, AgentId, IssueId, ProjectId, UserId};

use super::fake::{FakeError, FakeStore, Tx};
use crate::agents::{AgentPosition, AgentReach, AgentStore, AgentWorkStore, Hold};
use crate::auth::AccessKeyRecord;

/// Everything an agent has been given, the whole of it: what the fake keeps for the tests that
/// give work by hand (`give_work_on`), joined with the records of hand-overs and rules. The ports
/// answer from it a part at a time, as the real store does.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct AgentWork {
    pub(crate) projects: BTreeSet<ProjectId>,
    pub(crate) issues: BTreeSet<IssueId>,
    pub(crate) handed: BTreeSet<IssueId>,
    pub(crate) allowances: BTreeMap<IssueId, Allowance>,
    pub(crate) creates_in: BTreeSet<ProjectId>,
}

impl FakeStore {
    fn whole_work_of(&self, agent: AgentId, now: jiff::Timestamp) -> AgentWork {
        self.read(|state| {
            let mut work = state
                .agent_work
                .iter()
                .find(|(id, _)| *id == agent)
                .map(|(_, work)| work.clone())
                .unwrap_or_default();
            let records = &state.work;
            for held in records
                .works
                .iter()
                .filter(|w| w.agent == agent && records.is_open(w))
                .filter(|w| {
                    records
                        .hand_overs
                        .iter()
                        .any(|h| h.id == w.hand_over && h.ends_at > now)
                })
            {
                work.issues.insert(held.issue);
                work.handed.insert(held.issue);
                if let Some(hand_over) = records.hand_overs.iter().find(|h| h.id == held.hand_over)
                {
                    work.allowances
                        .entry(held.issue)
                        .or_default()
                        .add(Allowance::of_hand_over(
                            &hand_over.powers,
                            &hand_over.fields,
                        ));
                }
                work.projects.extend(
                    state
                        .issues
                        .iter()
                        .find(|issue| issue.id == held.issue)
                        .map(|issue| issue.project),
                );
            }
            for rule in records.rules.iter().filter(|r| {
                r.agent == agent
                    && r.state == bb_core::agent_records::GivingState::Open
                    && r.ends_at > now
            }) {
                work.projects.insert(rule.project);
                work.creates_in.extend(rule.creates_in);
                for found in records.matches.iter().filter(|m| m.rule == rule.id) {
                    work.issues.insert(found.issue);
                    work.allowances
                        .entry(found.issue)
                        .or_default()
                        .add(Allowance::of_rule(&rule.fields));
                }
            }
            work
        })
    }
}

impl AgentWorkStore for FakeStore {
    async fn agent_reach(
        &self,
        agent: AgentId,
        now: jiff::Timestamp,
    ) -> Result<AgentReach, FakeError> {
        let work = self.whole_work_of(agent, now);
        Ok(AgentReach {
            projects: work.projects,
            creates_in: work.creates_in,
        })
    }

    async fn agent_holds(
        &self,
        agent: AgentId,
        now: jiff::Timestamp,
        issues: Vec<IssueId>,
    ) -> Result<BTreeMap<IssueId, Hold>, FakeError> {
        let work = self.whole_work_of(agent, now);
        Ok(issues
            .into_iter()
            .filter(|issue| work.issues.contains(issue))
            .map(|issue| {
                let hold = Hold {
                    handed: work.handed.contains(&issue),
                    allowance: work.allowances.get(&issue).cloned().unwrap_or_default(),
                };
                (issue, hold)
            })
            .collect())
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

    async fn agents_by_ids_in(tx: &mut Tx, ids: Vec<AgentId>) -> Result<Vec<Agent>, FakeError> {
        Ok(tx
            .state
            .agents
            .iter()
            .filter(|agent| ids.contains(&agent.id))
            .cloned()
            .collect())
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

    async fn agent_key_count_in(tx: &mut Tx, agent: AgentId) -> Result<i64, FakeError> {
        let held = tx
            .state
            .access_keys
            .iter()
            .filter(|record| record.key.agent == Some(agent))
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
        after: Option<AgentPosition>,
        limit: u64,
    ) -> Result<Vec<Agent>, FakeError> {
        let place = |agent: &Agent| (agent.state.rank(), agent.name.to_lowercase(), agent.id);
        Ok(self.read(|state| {
            let mut found: Vec<Agent> = state.agents.clone();
            found.sort_by_key(place);
            found
                .into_iter()
                .filter(|agent| {
                    after
                        .as_ref()
                        .is_none_or(|after| place(agent) > place_of(after))
                })
                .take(usize::try_from(limit).unwrap_or(usize::MAX))
                .collect()
        }))
    }

    async fn last_use_of_agents(
        &self,
        agents: Vec<AgentId>,
    ) -> Result<Vec<(AgentId, jiff::Timestamp)>, FakeError> {
        Ok(self.read(|state| {
            let mut latest: BTreeMap<AgentId, jiff::Timestamp> = BTreeMap::new();
            for record in &state.access_keys {
                let (Some(agent), Some(at)) = (record.key.agent, record.key.last_used_at) else {
                    continue;
                };
                if agents.contains(&agent) {
                    let slot = latest.entry(agent).or_insert(at);
                    *slot = (*slot).max(at);
                }
            }
            latest.into_iter().collect()
        }))
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

    async fn agents_offered_to(&self, person: UserId, limit: u64) -> Result<Vec<Agent>, FakeError> {
        Ok(self.read(|state| {
            page(
                &state.agents,
                |a| a.offered_to == Some(person) && a.state != AgentState::Stopped,
                None,
                limit,
            )
        }))
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
        paused_for: Option<bb_core::agent::LimitKind>,
        version: i64,
    ) -> Result<bool, FakeError> {
        Ok(change(tx, id, version, |agent| {
            agent.state = state;
            agent.paused_for = paused_for;
        }))
    }

    async fn end_offer_of(tx: &mut Tx, id: AgentId) -> Result<(), FakeError> {
        if let Some(agent) = tx
            .state
            .agents
            .iter_mut()
            .find(|agent| agent.id == id && agent.offered_to.is_some())
        {
            agent.offered_to = None;
        }
        Ok(())
    }

    async fn set_agent_limits(
        tx: &mut Tx,
        id: AgentId,
        limits: AgentLimits,
        version: i64,
    ) -> Result<bool, FakeError> {
        Ok(change(tx, id, version, |agent| {
            agent.changes_a_minute = limits.changes_a_minute;
            agent.issues_at_once = limits.issues_at_once;
            agent.attempts_on_one_issue = limits.attempts_on_one_issue;
        }))
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

/// The place of a position, as the order of the list of every agent says it.
fn place_of(position: &AgentPosition) -> (i64, String, AgentId) {
    (
        position.state.rank(),
        position.name.to_lowercase(),
        position.id,
    )
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
