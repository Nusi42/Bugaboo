//! What the use cases of agents take and give back. Inputs that change something are named after
//! what they do; inputs that only read end in `Query` (ARCH-EVENTS).

use bb_core::agent::{AgentLimits, AgentState, LimitKind};
use bb_core::agent_request::{AgentRequest, AskedAgent, RequestStands};
use bb_core::id::{AgentId, AgentRequestId, UserId};
use bb_policy::{Masked, PersonRef};
use jiff::Timestamp;

use crate::auth::{AccessKeyView, CreatedAccessKey};

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

/// Give an agent other limits.
pub struct SetAgentLimits {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The agent.
    pub id: AgentId,
    /// The new limits; at least one each, and at most what the organization allows.
    pub limits: AgentLimits,
    /// The version the person saw.
    pub version: i64,
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

/// Take back what was offered of an agent: the one who handed it over takes the offer back, the one it
/// was handed to declines it.
pub struct TakeBackOffer {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The agent.
    pub id: AgentId,
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

/// Where a page of the list of every agent starts: after the last agent of the page before, in the
/// order working, paused, stopped, then by name, then by ID. It holds what the last agent was
/// sorted by, so the next page is right even if that agent changed its state meanwhile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentPosition {
    /// Its state.
    pub state: AgentState,
    /// Its name.
    pub name: String,
    /// Its ID.
    pub id: AgentId,
}

impl AgentPosition {
    /// The position of an agent.
    #[must_use]
    pub fn of(agent: &bb_core::agent::Agent) -> Self {
        Self {
            state: agent.state,
            name: agent.name.clone(),
            id: agent.id,
        }
    }

    /// The position as the cursor of the next page says it: `state`, `ID`, `name`.
    #[must_use]
    pub fn cursor(&self) -> String {
        format!("{}:{}:{}", self.state.code(), self.id, self.name)
    }

    /// The position a cursor says; none for text that is not a cursor.
    #[must_use]
    pub fn from_cursor(cursor: &str) -> Option<Self> {
        let mut parts = cursor.splitn(3, ':');
        Some(Self {
            state: AgentState::from_code(parts.next()?)?,
            id: parts.next()?.parse().ok()?,
            name: parts.next()?.to_owned(),
        })
    }
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
    /// The limit it reached when it was paused; none otherwise.
    pub paused_for: Option<LimitKind>,
    /// The person responsible for it, as *Who sees people* says for the viewer.
    pub person: PersonRef,
    /// What it may do at most.
    pub limits: AgentLimits,
    /// The most any agent can be given, when the view is of one agent; none in a list.
    pub maximums: Option<AgentLimits>,
    /// Whether the viewer is the person responsible for it (an administrator who is not may read it
    /// and stop it, nothing more).
    pub yours: bool,
    /// Whether it is offered to another person who has not accepted yet.
    pub offered: bool,
    /// The version of the record.
    pub version: i64,
    /// When it was made.
    pub made_at: Timestamp,
    /// When it last signed in with a key, when it did. Read for the list of the organization and
    /// for one agent, not for the agents of a person.
    pub last_used_at: Option<Timestamp>,
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

/// One row of *Create several agents*: an agent and its first key.
#[derive(Clone, Debug)]
pub struct SeveralAgent {
    /// What the agent is called; unique among the person's agents. Its key is called the same.
    pub name: String,
    /// When its key stops working, as a date and time such as `2027-03-01T00:00:00Z`; left out
    /// for the organization's default number of days from now.
    pub expires_at: Option<String>,
}

/// *Create several agents*: every agent of the list with one key each, in one change, with the
/// password once (ADR-0029, amendment M9).
pub struct CreateSeveralAgents {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The password, asked again for the keys.
    pub password: String,
    /// The agents, in the order they are shown.
    pub agents: Vec<SeveralAgent>,
}

/// An agent that was made with its key. Not `Debug`: the secret of the key is in it.
#[derive(Clone)]
pub struct CreatedAgent {
    /// The agent.
    pub agent: AgentView,
    /// Its key, with the secret, shown once.
    pub key: CreatedAccessKey,
}

/// The agents a list made, in its order, each with its key.
#[derive(Clone)]
pub struct CreatedAgents {
    /// The agents and their keys.
    pub items: Vec<CreatedAgent>,
}

impl Masked for CreatedAgents {}

/// One agent a program asks for.
#[derive(Clone, Debug)]
pub struct AgentAsked {
    /// The name the agent would have.
    pub name: String,
    /// What it is for, in a few words.
    pub purpose: String,
    /// How many days its key should last; none for the organization's default.
    pub key_days: Option<i64>,
}

/// A program asks the person for agents (`ask_for_agents`): only a request is made.
pub struct AskForAgents {
    /// The access key or the token of the app that asks.
    pub session_token: String,
    /// Why it asks.
    pub reason: String,
    /// The agents it asks for, in order.
    pub agents: Vec<AgentAsked>,
}

/// The requests for agents asked of the person.
pub struct AgentRequestsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// The person approves requests with the password once.
pub struct ApproveAgentRequests {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The password, asked again.
    pub password: String,
    /// The requests, as the person ticked them.
    pub requests: Vec<AgentRequestId>,
}

/// The person declines a request.
pub struct DeclineAgentRequest {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The request.
    pub id: AgentRequestId,
}

/// The program that asked collects the keys of an approved request (`collect_agent_keys`).
pub struct CollectAgentKeys {
    /// The access key or the token of the app that asked.
    pub session_token: String,
    /// The request.
    pub id: AgentRequestId,
}

/// A request for agents as its person sees it.
#[derive(Clone, Debug, serde::Serialize)]
pub struct AgentRequestView {
    /// The ID of the request.
    pub id: AgentRequestId,
    /// The name of the app or the key that asked.
    pub asked_by: String,
    /// Whether an AI app asked (by its grant), not a key.
    pub by_app: bool,
    /// The web address the app comes back to; none for a key and for an app on the person's
    /// computer.
    pub web_address: Option<String>,
    /// Why it asked.
    pub reason: String,
    /// The agents it asks for, with the agents an approval made.
    pub agents: Vec<AskedAgent>,
    /// Where it stands now.
    pub stands: RequestStands,
    /// When it was asked.
    pub asked_at: Timestamp,
    /// Until when it waits for an answer, or for its keys once approved.
    pub waits_until: Timestamp,
}

impl Masked for AgentRequestView {}

impl AgentRequestView {
    /// The request as its person sees it at `now`.
    pub(crate) fn of(request: &AgentRequest, now: Timestamp) -> Self {
        Self {
            id: request.id,
            asked_by: request.asked_by_name.clone(),
            by_app: request.asked_by_grant.is_some(),
            web_address: request.asked_from.clone(),
            reason: request.reason.clone(),
            agents: request.agents.clone(),
            stands: request.stands_at(now),
            asked_at: request.asked_at,
            waits_until: request.waits_until(),
        }
    }
}

/// The requests for agents of a person, newest first.
#[derive(Clone, Debug, serde::Serialize)]
pub struct AgentRequestPage {
    /// The requests.
    pub items: Vec<AgentRequestView>,
    /// Whether there were more than the list shows.
    pub truncated: bool,
}

impl Masked for AgentRequestPage {}

/// One key a pickup made. Not `Debug`: the secret is in it.
#[derive(Clone, serde::Serialize)]
pub struct CollectedKey {
    /// The agent the key belongs to.
    pub agent: Option<AgentId>,
    /// The name of the key, which is the name of its agent.
    pub name: String,
    /// The whole key, shown once.
    pub secret: String,
}

/// What a pickup answers: where the request stands, and the keys when it made them. Not `Debug`.
#[derive(Clone, serde::Serialize)]
pub struct CollectedKeys {
    /// Where the request stands now.
    pub stands: RequestStands,
    /// The keys, once, when this pickup made them.
    pub keys: Vec<CollectedKey>,
    /// For a preview of an approved request: the names of the keys a pickup would make.
    pub names: Vec<String>,
}

impl Masked for CollectedKeys {}
