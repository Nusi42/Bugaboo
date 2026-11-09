//! The ports of agents: the records of agents, and the work an agent has been given.
//!
//! Like the other ports, reads take `&self` and **writes** are associated functions that run inside
//! `record_change`, so a change and its domain event are committed together (ARCH-EVENTS).

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;

use bb_core::agent::{Agent, AgentLimits, AgentState, LimitKind};
use bb_core::agent_work::Allowance;
use bb_core::id::{AgentId, IssueId, ProjectId, UserId};
use jiff::Timestamp;

use super::types::AgentPosition;
use crate::auth::AccessKeyRecord;
use crate::ports::ChangeStore;
use crate::store_macros::{queries, reads, writes};

/// Where an agent has been given work (ADR-0029, decision 2): the projects its hand-overs and
/// standing rules are in, and the projects its rules may create issues in. Until hand-overs exist
/// an agent has none, and may do nothing but read its own record. It never lists the issues: an
/// agent holds thousands, and a request asks about the few it touches ([`Hold`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AgentReach {
    /// The projects the work is in.
    pub projects: BTreeSet<ProjectId>,
    /// The projects in which the standing rules of the agent may create issues.
    pub creates_in: BTreeSet<ProjectId>,
}

/// What an agent holds on one issue it reaches: handed over to it, or only found by a standing
/// rule, and what it may change there.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Hold {
    /// The issue is handed over to it, as against found by a standing rule: it works on a handed
    /// issue (comments, reports, attempts); on the others it reads and proposes.
    pub handed: bool,
    /// What the agent may change: the ticks of the hand-overs and the fields of the rules. Reading,
    /// commenting and reporting need nothing.
    pub allowance: Allowance,
}

/// The work of agents, read as the policy needs it. The store of hand-overs and standing rules
/// fills it (agent work); until then every agent has no work.
pub trait AgentWorkStore: ChangeStore + 'static {
    reads! {
        /// Where one agent has work at `now`: hand-overs and rules that ran out do not count.
        fn agent_reach(agent: AgentId, now: Timestamp) -> AgentReach;
        /// What one agent holds on each of these issues at `now`, read together: an issue it does
        /// not reach has no entry. The one definition of an issue an agent reaches (its open work
        /// and what its working rules found).
        fn agent_holds(agent: AgentId, now: Timestamp, issues: Vec<IssueId>) -> BTreeMap<IssueId, Hold>;
    }
}

/// What the use cases of agents need from storage.
pub trait AgentStore: ChangeStore + AgentWorkStore + 'static {
    queries! {
        /// The agent with this ID.
        fn agent(id: AgentId) -> Option<Agent> = agent_in;
        /// The agents with these IDs, read together.
        fn agents_by_ids(ids: Vec<AgentId>) -> Vec<Agent> = agents_by_ids_in;
        /// How many agents a person is responsible for.
        fn agent_count_of(person: UserId) -> i64 = agent_count_of_in;
        /// How many keys an agent has.
        fn agent_key_count(agent: AgentId) -> i64 = agent_key_count_in;
    }

    reads! {
        /// The agents a person is responsible for, by ID, after `after`, at most `limit`.
        fn agents_of(person: UserId, after: Option<AgentId>, limit: u64) -> Vec<Agent>;
        /// Every agent of the organization, working ones first, then paused, then stopped, by name
        /// and ID inside each, after `after`, at most `limit`.
        fn all_agents(after: Option<AgentPosition>, limit: u64) -> Vec<Agent>;
        /// When each of these agents last signed in with a key, for those that did.
        fn last_use_of_agents(agents: Vec<AgentId>) -> Vec<(AgentId, Timestamp)>;
        /// The keys of one agent, newest first.
        fn keys_of_agent(agent: AgentId) -> Vec<AccessKeyRecord>;
        /// The agents handed to a person that they have not accepted yet and that are not stopped,
        /// by ID, at most `limit`.
        fn agents_offered_to(person: UserId, limit: u64) -> Vec<Agent>;
    }

    writes! {
        /// Takes the lock of one person's agents, so that two changes at the same moment cannot
        /// both pass the most agents; does nothing where writes cannot overlap.
        fn lock_agents_of(person: UserId) -> ();
        /// Adds an agent.
        fn insert_agent(agent: &Agent) -> ();
        /// Writes the name, counts the version up and says whether the agent was at `version`.
        fn rename_agent(id: AgentId, name: &str, version: i64) -> bool;
        /// Writes the state and the limit that caused it (only a pause by a limit has one), counts
        /// the version up and says whether the agent was at `version`.
        fn set_agent_state(id: AgentId, state: AgentState, paused_for: Option<LimitKind>, version: i64) -> bool;
        /// Writes the limits, counts the version up and says whether the agent was at `version`.
        fn set_agent_limits(id: AgentId, limits: AgentLimits, version: i64) -> bool;
        /// Ends the offer of the agent without counting its version up: it runs inside the change that
        /// stops the agent, which counts the version once (a stopped agent is offered to nobody).
        fn end_offer_of(id: AgentId) -> ();
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
