//! The ports of agents: the records of agents, and the work an agent has been given.
//!
//! Like the other ports, reads take `&self` and **writes** are associated functions that run inside
//! `record_change`, so a change and its domain event are committed together (ARCH-EVENTS).

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;

use bb_core::agent::{Agent, AgentState};
use bb_core::agent_work::Allowance;
use bb_core::id::{AgentId, IssueId, ProjectId, UserId};

use crate::auth::AccessKeyRecord;
use crate::ports::ChangeStore;
use crate::store_macros::{queries, reads, writes};

/// What an agent has been given to work on (ADR-0029, decision 2): the projects its hand-overs and
/// standing rules are in, and the issues of its hand-overs. Until hand-overs exist an agent has
/// none, and may do nothing but read its own record.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AgentWork {
    /// The projects the work is in.
    pub projects: BTreeSet<ProjectId>,
    /// The issues handed over, or found by a standing rule.
    pub issues: BTreeSet<IssueId>,
    /// What the agent may change on each of them: the ticks of the hand-overs and the fields of
    /// the rules. Reading, commenting and reporting need nothing.
    pub allowances: BTreeMap<IssueId, Allowance>,
}

/// The work of agents, read as the policy needs it. The store of hand-overs and standing rules
/// fills it (agent work); until then every agent has no work.
pub trait AgentWorkStore: ChangeStore + 'static {
    reads! {
        /// The work of one agent.
        fn agent_work(agent: AgentId) -> AgentWork;
    }
}

/// What the use cases of agents need from storage.
pub trait AgentStore: ChangeStore + AgentWorkStore + 'static {
    queries! {
        /// The agent with this ID.
        fn agent(id: AgentId) -> Option<Agent> = agent_in;
        /// How many agents a person is responsible for.
        fn agent_count_of(person: UserId) -> i64 = agent_count_of_in;
        /// How many keys an agent has.
        fn agent_key_count(agent: AgentId) -> i64 = agent_key_count_in;
    }

    reads! {
        /// The agents a person is responsible for, by ID, after `after`, at most `limit`.
        fn agents_of(person: UserId, after: Option<AgentId>, limit: u64) -> Vec<Agent>;
        /// Every agent of the organization, by ID, after `after`, at most `limit`.
        fn all_agents(after: Option<AgentId>, limit: u64) -> Vec<Agent>;
        /// The keys of one agent, newest first.
        fn keys_of_agent(agent: AgentId) -> Vec<AccessKeyRecord>;
        /// The agents handed to a person that they have not accepted yet.
        fn agents_offered_to(person: UserId) -> Vec<Agent>;
    }

    writes! {
        /// Takes the lock of one person's agents, so that two changes at the same moment cannot
        /// both pass the most agents; does nothing where writes cannot overlap.
        fn lock_agents_of(person: UserId) -> ();
        /// Adds an agent.
        fn insert_agent(agent: &Agent) -> ();
        /// Writes the name, counts the version up and says whether the agent was at `version`.
        fn rename_agent(id: AgentId, name: &str, version: i64) -> bool;
        /// Writes the state, counts the version up and says whether the agent was at `version`.
        fn set_agent_state(id: AgentId, state: AgentState, version: i64) -> bool;
        /// Offers the agent to a person (`Some`) or takes the offer back (`None`).
        fn offer_agent(id: AgentId, to: Option<UserId>, version: i64) -> bool;
        /// Hands the agent to the person who accepted it, counting the version up.
        fn hand_agent_to(id: AgentId, person: UserId, version: i64) -> bool;
        /// Stops every active or paused agent of a person; the IDs of those that were stopped.
        fn stop_agents_of(person: UserId) -> Vec<AgentId>;
        /// Removes every key of an agent; the IDs of the keys that went.
        fn delete_keys_of_agent(agent: AgentId) -> Vec<bb_core::id::AccessKeyId>;
    }
}
