//! What the use cases of agents take and give back. Inputs that change something are named after
//! what they do; inputs that only read end in `Query` (ARCH-EVENTS).

use bb_core::agent::{AgentLimits, AgentState};
use bb_core::id::{AgentId, UserId};
use bb_policy::{Masked, PersonRef};
use jiff::Timestamp;

use crate::auth::AccessKeyView;

/// Make an agent for the person.
pub struct CreateAgent {
    /// The secret of the session of the request.
    pub session_token: String,
    /// What the agent is called; unique among the person's agents.
    pub name: String,
}

/// Give an agent another name.
pub struct RenameAgent {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The agent.
    pub id: AgentId,
    /// The new name.
    pub name: String,
    /// The version the person saw.
    pub version: i64,
}

/// What a person does to the state of an agent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentAction {
    /// Let it rest until the person resumes it.
    Pause,
    /// Let a paused agent work again.
    Resume,
    /// Stop it for good.
    Stop,
}

/// Pause, resume or stop an agent.
pub struct ChangeAgent {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The agent.
    pub id: AgentId,
    /// What to do.
    pub action: AgentAction,
    /// The version the person saw.
    pub version: i64,
}

/// *Stop all my agents*.
pub struct StopAllAgents {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// Hand an agent to another person, who has to accept it.
pub struct HandAgent {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The agent.
    pub id: AgentId,
    /// The person it is offered to.
    pub to: UserId,
    /// The version the person saw.
    pub version: i64,
}

/// Accept an agent that was handed to the person.
pub struct AcceptAgent {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The agent.
    pub id: AgentId,
}

/// One agent.
pub struct AgentQuery {
    /// The secret of the session or the agent key of the request.
    pub session_token: String,
    /// The agent.
    pub id: AgentId,
}

/// A list of agents, one page at a time.
pub struct AgentsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Where the page after the last one starts, if any.
    pub cursor: Option<String>,
    /// How many agents a page holds at most.
    pub limit: u32,
}

/// An agent as the viewer sees it.
#[derive(Clone, Debug)]
pub struct AgentView {
    /// The ID of the agent.
    pub id: AgentId,
    /// Its name.
    pub name: String,
    /// Whether it may act.
    pub state: AgentState,
    /// The person responsible for it, as *Who sees people* says for the viewer.
    pub person: PersonRef,
    /// What it may do at most.
    pub limits: AgentLimits,
    /// Whether it is offered to another person who has not accepted yet.
    pub offered: bool,
    /// The version of the record.
    pub version: i64,
    /// When it was made.
    pub made_at: Timestamp,
    /// Its keys, newest first.
    pub keys: Vec<AccessKeyView>,
}

impl Masked for AgentView {}

/// A page of agents.
#[derive(Clone, Debug)]
pub struct AgentPage {
    /// The agents, by ID.
    pub items: Vec<AgentView>,
    /// The cursor of the next page, if there is one.
    pub next_cursor: Option<String>,
}

impl Masked for AgentPage {}
