//! Agents (AI-30, ADR-0029): accounts of their own that are not people, each with one responsible
//! person. What an agent may do is never stored: it is its person's evaluation narrowed by its
//! work (`bb_policy::Narrowing::agent`), made again at every request.

mod actor;
mod manage;
mod store;
mod types;

pub use manage::{
    accept_agent, agent, change_agent, create_agent, hand_agent, my_agents, organization_agents,
    rename_agent, stop_all_agents,
};
pub use store::{AgentStore, AgentWork, AgentWorkStore};
pub use types::{
    AcceptAgent, AgentAction, AgentPage, AgentQuery, AgentView, AgentsQuery, ChangeAgent,
    CreateAgent, HandAgent, RenameAgent, StopAllAgents,
};
