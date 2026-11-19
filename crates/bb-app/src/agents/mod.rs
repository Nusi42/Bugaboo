//! Agents (AI-30, ADR-0029): accounts of their own that are not people, each with one responsible
//! person. What an agent may do is never stored: it is its person's evaluation narrowed by its
//! work (`bb_policy::Narrowing::agent`), made again at every request.

mod actor;
mod limits;
mod manage;
mod names;
mod plan;
mod request_store;
mod several;
mod store;
mod types;
mod values;

pub(crate) use limits::{AgentRate, input_changes_something};
pub use manage::{
    accept_agent, agent, change_agent, create_agent, hand_agent, my_agents, organization_agents,
    rename_agent, stop_all_agents, take_back_offer,
};
pub(crate) use manage::{agents_stopped, stop_agents_in};
pub(crate) use names::AgentNames;
pub use request_store::AgentRequestStore;
pub use several::create_several_agents;
pub use several::requests::{
    agent_requests, approve_agent_requests, ask_for_agents, collect_agent_keys,
    decline_agent_request,
};
pub use store::{AgentReach, AgentStore, AgentWorkStore, Hold};
pub use types::{
    AcceptAgent, AgentAction, AgentAsked, AgentPage, AgentPosition, AgentQuery, AgentRequestPage,
    AgentRequestView, AgentRequestsQuery, AgentView, AgentsQuery, ApproveAgentRequests,
    AskForAgents, ChangeAgent, CollectAgentKeys, CollectedKey, CollectedKeys, CreateAgent,
    CreateSeveralAgents, CreatedAgent, CreatedAgents, DeclineAgentRequest, HandAgent, RenameAgent,
    SetAgentLimits, SetAgentSwitches, SeveralAgent, StopAllAgents, TakeBackOffer,
};
pub use values::{set_agent_limits, set_agent_switches};
