//! Agents (AI-30, ADR-0029): an account of its own that is not a person. It has one responsible
//! person and never does more than that person may, narrowed by its work and its limits.
//!
//! The record holds who is responsible, the state and the limits. What an agent may do is never
//! stored with it: at every request it is its person's evaluation, intersected with its work
//! (`bb_policy::Narrowing::agent`).

use jiff::Timestamp;

use crate::id::{AgentId, UserId};
use crate::personal::Entity;

/// The most agents one person can have (also the most the list of their agents shows).
pub const MOST_AGENTS: usize = 50;

/// The most keys one agent can have at the same time.
pub const MOST_AGENT_KEYS: usize = 20;

/// The most changes an agent may make in a minute when nobody chose otherwise (ADR-0029, decision 9).
pub const DEFAULT_CHANGES_A_MINUTE: i64 = 60;
/// The most issues an agent may work on at once when nobody chose otherwise.
pub const DEFAULT_ISSUES_AT_ONCE: i64 = 5;
/// The most attempts on one issue when nobody chose otherwise.
pub const DEFAULT_ATTEMPTS_ON_ONE_ISSUE: i64 = 5;
/// The most issues one request of an agent key can start a change to (M3-F121).
pub const MOST_ISSUES_OF_ONE_CHANGE: usize = 1_000;

words! {
    /// Whether an agent may act.
    pub enum AgentState {
        /// It may work.
        Active => "active",
        /// Its person paused it; its next request is refused until they resume it.
        Paused => "paused",
        /// Stopped by its person or an administrator; it does not work again.
        Stopped => "stopped",
    }
}

crate::words_in_api!(AgentState, "Whether an agent may act.");

words! {
    /// The limits of an agent (ADR-0029, decision 9). Reaching one pauses the agent with a
    /// sentence that names it.
    pub enum LimitKind {
        /// Changes in one minute.
        ChangesAMinute => "changes-a-minute",
        /// Issues it works on at once.
        IssuesAtOnce => "issues-at-once",
        /// Attempts on one issue.
        AttemptsOnOneIssue => "attempts-on-one-issue",
    }
}

crate::words_in_api!(LimitKind, "Which limit of an agent it reached.");

impl LimitKind {
    /// The sentence that says an agent was paused by this limit: *"Fixer paused: 60 changes in a
    /// minute"*.
    #[must_use]
    pub fn sentence(self, agent: &str, limit: i64) -> crate::message::Message {
        use crate::message;
        match self {
            Self::ChangesAMinute => message::agent_paused_changes_a_minute(agent, limit),
            Self::IssuesAtOnce => message::agent_paused_issues_at_once(agent, limit),
            Self::AttemptsOnOneIssue => message::agent_paused_attempts_on_one_issue(agent, limit),
        }
    }
}

impl AgentLimits {
    /// The number the agent may not pass for `kind`.
    #[must_use]
    pub const fn of(&self, kind: LimitKind) -> i64 {
        match kind {
            LimitKind::ChangesAMinute => self.changes_a_minute,
            LimitKind::IssuesAtOnce => self.issues_at_once,
            LimitKind::AttemptsOnOneIssue => self.attempts_on_one_issue,
        }
    }
}

/// How much an agent may do (ADR-0029, decision 9). Each is at most the organization's maximum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AgentLimits {
    /// Changes in one minute.
    pub changes_a_minute: i64,
    /// Issues it works on at once.
    pub issues_at_once: i64,
    /// Attempts on one issue.
    pub attempts_on_one_issue: i64,
}

impl Default for AgentLimits {
    fn default() -> Self {
        Self {
            changes_a_minute: DEFAULT_CHANGES_A_MINUTE,
            issues_at_once: DEFAULT_ISSUES_AT_ONCE,
            attempts_on_one_issue: DEFAULT_ATTEMPTS_ON_ONE_ISSUE,
        }
    }
}

/// An agent. Every field carries its personal-data mark (ADR-0015).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Agent {
    /// The ID of the agent.
    #[not_personal]
    pub id: AgentId,
    /// The person responsible for it.
    #[personal(category = "identity", basis = "contract")]
    pub person: UserId,
    /// Its name, unique among the agents of its person.
    #[personal(category = "content", basis = "contract")]
    pub name: String,
    /// Whether it may act.
    #[not_personal]
    pub state: AgentState,
    /// The limit it reached when it was paused; none when its person paused it, or it is not
    /// paused.
    #[not_personal]
    pub paused_for: Option<LimitKind>,
    /// What it may do at most: changes in a minute.
    #[not_personal]
    pub changes_a_minute: i64,
    /// Issues at once.
    #[not_personal]
    pub issues_at_once: i64,
    /// Attempts on one issue.
    #[not_personal]
    pub attempts_on_one_issue: i64,
    /// The person it is handed to, who has not accepted yet (the hand-over of an agent needs the
    /// acceptance of the other person).
    #[personal(category = "identity", basis = "contract")]
    pub offered_to: Option<UserId>,
    /// Who made it.
    #[personal(category = "identity", basis = "contract")]
    pub made_by: UserId,
    /// When it was made.
    #[personal(category = "activity", basis = "contract")]
    pub made_at: Timestamp,
    /// The version of the record, counted up by every change.
    #[not_personal]
    pub version: i64,
}

impl Agent {
    /// Its limits.
    #[must_use]
    pub const fn limits(&self) -> AgentLimits {
        AgentLimits {
            changes_a_minute: self.changes_a_minute,
            issues_at_once: self.issues_at_once,
            attempts_on_one_issue: self.attempts_on_one_issue,
        }
    }

    /// Whether it may act now.
    #[must_use]
    pub fn may_act(&self) -> bool {
        self.state == AgentState::Active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_30_the_limits_start_at_the_numbers_of_the_decision_record() {
        assert_eq!(
            AgentLimits::default(),
            AgentLimits {
                changes_a_minute: 60,
                issues_at_once: 5,
                attempts_on_one_issue: 5
            }
        );
    }
}
