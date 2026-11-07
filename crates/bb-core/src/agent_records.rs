//! The stored records of agent work (AI-31 to AI-35, ADR-0029). Every field carries its
//! personal-data mark (ADR-0015); briefs, notes, questions and answers are people's text.
//!
//! The rules over these records (ends, states, what a rule may change) are in
//! [`crate::agent_work`].

use jiff::Timestamp;

use crate::agent_work::{AgentWorkState, CheckResult, HandOverPower, ReviewAnswer, RuleField};
use crate::id::{
    AgentId, AgentNoteId, AgentWorkId, AttemptId, HandOverId, IssueId, ProjectId, SavedSearchId,
    StandingRuleId, UserId,
};
use crate::personal::Entity;

words! {
    /// Whether a hand-over or a standing rule still gives the agent work.
    pub enum GivingState {
        /// It gives work.
        Open => "open",
        /// It ended: accepted, stopped or ran out.
        Ended => "ended",
    }
}

crate::words_in_api!(
    GivingState,
    "Whether a hand-over or a standing rule still gives work."
);

words! {
    /// What became of a proposal of a standing rule.
    pub enum ProposalState {
        /// A person has not decided yet; nothing changed.
        Open => "open",
        /// A person applied it.
        Applied => "applied",
        /// A person discarded it.
        Discarded => "discarded",
    }
}

crate::words_in_api!(
    ProposalState,
    "What became of a proposal of a standing rule."
);

words! {
    /// What an attempt attached: a link to a result, or the result of a check.
    pub enum ResultKind {
        /// A link such as a branch, a pull request or a test report.
        Link => "link",
        /// A check with *passed* or *failed*.
        Check => "check",
    }
}

crate::words_in_api!(ResultKind, "What an agent attached to an attempt.");

/// One or several issues given to one agent (AI-31).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct HandOver {
    /// The ID of the hand-over.
    #[not_personal]
    pub id: HandOverId,
    /// The agent that gets the work.
    #[not_personal]
    pub agent: AgentId,
    /// The person who handed over.
    #[personal(category = "identity", basis = "contract")]
    pub person: UserId,
    /// The note of the brief.
    #[personal(category = "content", basis = "contract")]
    pub note: String,
    /// What the agent may do besides what it always may.
    #[not_personal]
    pub powers: Vec<HandOverPower>,
    /// The fields the agent may change, as their keys, when it may change fields.
    #[personal(category = "content", basis = "contract")]
    pub fields: Vec<String>,
    /// The most attempts on one issue.
    #[not_personal]
    pub budget_attempts: i64,
    /// The most minutes of work, as the agent reports them.
    #[not_personal]
    pub budget_minutes: Option<i64>,
    /// The most cost, in cents, as the agent reports it.
    #[not_personal]
    pub budget_cost: Option<i64>,
    /// Whether each attempt waits for *Approve plan*.
    #[not_personal]
    pub plan_needs_approval: bool,
    /// Whether an attempt may be accepted by itself (when the project allows it too).
    #[not_personal]
    pub accept_by_itself: bool,
    /// Whether it still gives work.
    #[not_personal]
    pub state: GivingState,
    /// When it ends.
    #[not_personal]
    pub ends_at: Timestamp,
    /// When it was made.
    #[not_personal]
    pub made_at: Timestamp,
    /// The version of the record.
    #[not_personal]
    pub version: i64,
}

/// A saved search an agent works on, with narrow powers (AI-32).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct StandingRule {
    /// The ID of the rule.
    #[not_personal]
    pub id: StandingRuleId,
    /// The agent that works on what the search finds.
    #[not_personal]
    pub agent: AgentId,
    /// The person who made the rule.
    #[personal(category = "identity", basis = "contract")]
    pub person: UserId,
    /// The saved search.
    #[not_personal]
    pub search: SavedSearchId,
    /// The project of the search; the rule works only there.
    #[not_personal]
    pub project: ProjectId,
    /// The fields the rule may change; empty means it only proposes.
    #[not_personal]
    pub fields: Vec<RuleField>,
    /// The project in which the rule may create issues.
    #[not_personal]
    pub creates_in: Option<ProjectId>,
    /// Whether it still works.
    #[not_personal]
    pub state: GivingState,
    /// When it ends.
    #[not_personal]
    pub ends_at: Timestamp,
    /// When it was made.
    #[not_personal]
    pub made_at: Timestamp,
    /// The version of the record.
    #[not_personal]
    pub version: i64,
}

/// The agent work on one issue of a hand-over (AI-33).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct IssueWork {
    /// The ID of the work.
    #[not_personal]
    pub id: AgentWorkId,
    /// The issue.
    #[not_personal]
    pub issue: IssueId,
    /// The hand-over that gave it.
    #[not_personal]
    pub hand_over: HandOverId,
    /// The agent.
    #[not_personal]
    pub agent: AgentId,
    /// The person responsible for the agent, when the work was given.
    #[personal(category = "identity", basis = "contract")]
    pub person: UserId,
    /// Where the work is. Not the issue's status.
    #[not_personal]
    pub state: AgentWorkState,
    /// Why the agent gave up, when it did.
    #[personal(category = "content", basis = "contract")]
    pub gave_up: Option<String>,
    /// Whether the work ended with a review: accepted, or thrown away. A hand-over of several
    /// issues goes on for the others.
    #[not_personal]
    pub closed: bool,
    /// Whether the work was accepted by itself and nobody has looked at it yet.
    #[not_personal]
    pub accepted_by_itself: bool,
    /// Whether a person still has to choose the next status of the issue: the work was accepted
    /// by itself and the workflow did not name one status to move to.
    #[not_personal]
    pub status_pending: bool,
    /// When it was given.
    #[not_personal]
    pub made_at: Timestamp,
    /// The version of the record.
    #[not_personal]
    pub version: i64,
}

impl IssueWork {
    /// Whether the work is open under its hand-over at `now`: no review closed it, it did not end
    /// as stopped or given up, and the hand-over still gives work and has not run out. The one
    /// definition the views use; the store's statements say the same (`open_work_of_agent`).
    #[must_use]
    pub fn is_open_under(&self, hand_over: &HandOver, now: Timestamp) -> bool {
        !self.closed
            && self.state.is_open()
            && hand_over.state == GivingState::Open
            && hand_over.ends_at > now
    }
}

/// One go of an agent at an issue (AI-33).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Attempt {
    /// The ID of the attempt.
    #[not_personal]
    pub id: AttemptId,
    /// The work it belongs to.
    #[not_personal]
    pub work: AgentWorkId,
    /// Its number on the issue, from 1.
    #[not_personal]
    pub number: i64,
    /// The plan the agent reported.
    #[personal(category = "content", basis = "contract")]
    pub plan: Option<String>,
    /// Whether a person approved the plan.
    #[not_personal]
    pub plan_approved: bool,
    /// The minutes the agent reports; never a time entry (M7-Q7).
    #[not_personal]
    pub reported_minutes: i64,
    /// The cost in cents the agent reports.
    #[not_personal]
    pub reported_cost: i64,
    /// When it started.
    #[not_personal]
    pub started_at: Timestamp,
    /// When it ended.
    #[not_personal]
    pub ended_at: Option<Timestamp>,
    /// The run of the attempt: everything it changed, undone in one step by *Throw away*.
    #[not_personal]
    pub run: Option<crate::id::RunId>,
}

/// A result an agent attached to an attempt: a link, or a check.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct AttemptResult {
    /// The ID of the result.
    #[not_personal]
    pub id: AgentNoteId,
    /// The attempt.
    #[not_personal]
    pub attempt: AttemptId,
    /// A link or a check.
    #[not_personal]
    pub kind: ResultKind,
    /// How the agent calls it.
    #[personal(category = "content", basis = "contract")]
    pub label: String,
    /// The link, for a link.
    #[personal(category = "content", basis = "contract")]
    pub link: Option<String>,
    /// The result, for a check.
    #[not_personal]
    pub outcome: Option<CheckResult>,
}

/// A question of an agent and its answer (AI-34).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Question {
    /// The ID of the question.
    #[not_personal]
    pub id: AgentNoteId,
    /// The work.
    #[not_personal]
    pub work: AgentWorkId,
    /// What the agent asks.
    #[personal(category = "content", basis = "contract")]
    pub text: String,
    /// The answer, which joins the brief.
    #[personal(category = "content", basis = "contract")]
    pub answer: Option<String>,
    /// Who answered.
    #[personal(category = "identity", basis = "contract")]
    pub answered_by: Option<UserId>,
    /// When it was asked.
    #[not_personal]
    pub asked_at: Timestamp,
}

/// The review that ends an attempt's work (AI-35).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Review {
    /// The ID of the review.
    #[not_personal]
    pub id: AgentNoteId,
    /// The work.
    #[not_personal]
    pub work: AgentWorkId,
    /// The answer.
    #[not_personal]
    pub answer: ReviewAnswer,
    /// The note; with *Send back* it joins the brief.
    #[personal(category = "content", basis = "contract")]
    pub note: Option<String>,
    /// Who reviewed, or nobody when the attempt was accepted by itself.
    #[personal(category = "identity", basis = "contract")]
    pub by: Option<UserId>,
    /// When.
    #[not_personal]
    pub at: Timestamp,
}

/// A change a standing rule proposes; nothing changes until a person applies it (AI-32).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Proposal {
    /// The ID of the proposal.
    #[not_personal]
    pub id: AgentNoteId,
    /// The rule.
    #[not_personal]
    pub rule: StandingRuleId,
    /// The agent that proposed it.
    #[not_personal]
    pub agent: AgentId,
    /// The issue.
    #[not_personal]
    pub issue: IssueId,
    /// The field.
    #[not_personal]
    pub field: RuleField,
    /// The value now, as the proposal saw it.
    #[personal(category = "content", basis = "contract")]
    pub before: serde_json::Value,
    /// The value proposed.
    #[personal(category = "content", basis = "contract")]
    pub after: serde_json::Value,
    /// What became of it.
    #[not_personal]
    pub state: ProposalState,
    /// Who decided.
    #[personal(category = "identity", basis = "contract")]
    pub decided_by: Option<UserId>,
    /// When it was made.
    #[not_personal]
    pub made_at: Timestamp,
}

/// An issue that newly came under a standing rule, written down with the moment (AI-32).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct RuleMatch {
    /// The rule.
    #[not_personal]
    pub rule: StandingRuleId,
    /// The issue.
    #[not_personal]
    pub issue: IssueId,
    /// When it was first seen under the rule.
    #[not_personal]
    pub at: Timestamp,
}
