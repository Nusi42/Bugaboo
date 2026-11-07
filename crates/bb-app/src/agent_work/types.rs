//! What the use cases of agent work take and give back. Inputs that change something are named
//! after what they do; inputs that only read end in `Query` (ARCH-EVENTS).

use bb_core::agent_records::{GrantState, ProposalState, ResultKind};
use bb_core::agent_work::{
    AgentWorkState, BriefPart, CheckResult, HandOverPower, ReviewAnswer, RuleField,
};
use bb_core::id::{
    AgentId, AgentNoteId, AgentWorkId, AttemptId, HandOverId, IssueId, ProjectId, SavedSearchId,
    StandingRuleId,
};
use bb_policy::PersonRef;
use jiff::Timestamp;

/// Hand issues over to one of the person's agents (AI-31).
pub struct HandOver {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The agent.
    pub agent: AgentId,
    /// The issues, by their IDs as people write them (`WEB-142`).
    pub issues: Vec<String>,
    /// The note of the brief.
    pub note: String,
    /// What the agent may do besides what it always may.
    pub powers: Vec<HandOverPower>,
    /// The fields the agent may change, when it may change fields.
    pub fields: Vec<String>,
    /// The most attempts on one issue; the agent's own limit when empty.
    pub budget_attempts: Option<i64>,
    /// The most minutes the agent reports.
    pub budget_minutes: Option<i64>,
    /// The most cost, in cents, the agent reports.
    pub budget_cost: Option<i64>,
    /// Whether each attempt waits for *Approve plan*.
    pub plan_needs_approval: bool,
    /// Whether an attempt may be accepted by itself when the project allows it.
    pub accept_by_itself: bool,
    /// After how many days the hand-over ends; 14 when empty.
    pub days: Option<i64>,
}

/// One issue of a hand-over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandedIssue {
    /// The work on it.
    pub work: AgentWorkId,
    /// The issue.
    pub issue: IssueId,
    /// What the brief lacks. It never stops the hand-over.
    pub missing: Vec<BriefPart>,
}

/// A hand-over as it was made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandOverView {
    /// The hand-over.
    pub id: HandOverId,
    /// The agent.
    pub agent: AgentId,
    /// The issues.
    pub issues: Vec<HandedIssue>,
    /// When it ends.
    pub ends_at: Timestamp,
    /// Whether it still gives work.
    pub state: GrantState,
    /// The version of the record.
    pub version: i64,
}

/// Read the agent work on an issue.
pub struct AgentWorkQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its ID as people write it.
    pub issue: String,
}

/// A result an agent attached.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResultView {
    /// A link or a check.
    pub kind: ResultKind,
    /// How the agent calls it.
    pub label: String,
    /// The link.
    pub link: Option<String>,
    /// The outcome of a check.
    pub outcome: Option<CheckResult>,
}

/// One attempt with what the agent reported. The numbers are the agent's own words, never a time
/// entry (M7-Q7).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttemptView {
    /// The attempt.
    pub id: AttemptId,
    /// Its number on the issue.
    pub number: i64,
    /// The plan.
    pub plan: Option<String>,
    /// Whether a person approved the plan.
    pub plan_approved: bool,
    /// The minutes the agent reports.
    pub reported_minutes: i64,
    /// The cost in cents the agent reports.
    pub reported_cost: i64,
    /// When it started.
    pub started_at: Timestamp,
    /// When it ended.
    pub ended_at: Option<Timestamp>,
    /// What it attached.
    pub results: Vec<ResultView>,
}

/// A question with its answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestionView {
    /// The question.
    pub id: AgentNoteId,
    /// What the agent asks.
    pub text: String,
    /// The answer.
    pub answer: Option<String>,
    /// When it was asked.
    pub asked_at: Timestamp,
}

/// A review.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewView {
    /// The answer.
    pub answer: ReviewAnswer,
    /// The note.
    pub note: Option<String>,
    /// Who reviewed, as the reader may see them; nobody when accepted by itself.
    pub by: Option<PersonRef>,
    /// When.
    pub at: Timestamp,
}

/// The agent work on one issue of one hand-over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkView {
    /// The work.
    pub id: AgentWorkId,
    /// The hand-over.
    pub hand_over: HandOverId,
    /// The agent.
    pub agent: AgentId,
    /// Where the work is.
    pub state: AgentWorkState,
    /// Whether the hand-over still gives work.
    pub open: bool,
    /// The note of the brief, then the answers to the questions.
    pub brief: Vec<String>,
    /// The attempts, oldest first.
    pub attempts: Vec<AttemptView>,
    /// The questions, oldest first.
    pub questions: Vec<QuestionView>,
    /// The reviews, oldest first.
    pub reviews: Vec<ReviewView>,
    /// Why the agent gave up.
    pub gave_up: Option<String>,
    /// Whether it was accepted by itself and nobody looked yet.
    pub accepted_by_itself: bool,
    /// The version of the work.
    pub version: i64,
}

/// An agent shown on an issue as *worked on by*.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkedOnBy {
    /// The agent.
    pub agent: AgentId,
    /// Its name.
    pub name: String,
    /// The person responsible, as the reader may see them.
    pub person: PersonRef,
}

/// The agent work on an issue, and who works on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IssueAgentWork {
    /// The agents with open work on the issue. The issue keeps its person; an agent is never
    /// *Assigned to*.
    pub worked_on_by: Vec<WorkedOnBy>,
    /// All the work, newest first.
    pub work: Vec<WorkView>,
    /// The proposals of standing rules that wait for a person.
    pub proposals: Vec<ProposalView>,
}

/// A change a standing rule proposes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposalView {
    /// The proposal.
    pub id: AgentNoteId,
    /// The rule.
    pub rule: StandingRuleId,
    /// The agent.
    pub agent: AgentId,
    /// The field.
    pub field: RuleField,
    /// The value now.
    pub before: serde_json::Value,
    /// The value proposed.
    pub after: serde_json::Value,
    /// What became of it.
    pub state: ProposalState,
    /// When.
    pub made_at: Timestamp,
}

/// Answer the question of an agent.
pub struct AnswerQuestion {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The work.
    pub work: AgentWorkId,
    /// The question.
    pub question: AgentNoteId,
    /// The answer, which joins the brief.
    pub answer: String,
    /// The version of the work the person saw.
    pub version: i64,
}

/// *Approve plan*, or a note instead.
pub struct ApprovePlan {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The work.
    pub work: AgentWorkId,
    /// The version of the work the person saw.
    pub version: i64,
    /// A note that joins the brief, when the person does not approve but corrects.
    pub note: Option<String>,
}

/// The review of a result.
pub struct ReviewWork {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The work.
    pub work: AgentWorkId,
    /// The answer.
    pub answer: ReviewAnswer,
    /// The note; it joins the brief with *Send back*.
    pub note: Option<String>,
    /// The version of the work the person saw.
    pub version: i64,
}

/// A standing rule to make.
pub struct CreateStandingRule {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The agent.
    pub agent: AgentId,
    /// The saved search.
    pub search: SavedSearchId,
    /// The fields the rule may change; empty means it only proposes.
    pub fields: Vec<RuleField>,
    /// The project in which it may create issues.
    pub creates_in: Option<ProjectId>,
    /// After how many days the rule ends; 30 when empty, at most 90.
    pub days: Option<i64>,
}

/// *Renew* a standing rule, or stop it.
pub struct ChangeStandingRule {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The rule.
    pub rule: StandingRuleId,
    /// Renew (`true`) or stop (`false`).
    pub renew: bool,
    /// After how many days the renewed rule ends.
    pub days: Option<i64>,
    /// The version the person saw.
    pub version: i64,
}

/// A standing rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StandingRuleView {
    /// The rule.
    pub id: StandingRuleId,
    /// The agent.
    pub agent: AgentId,
    /// The saved search.
    pub search: SavedSearchId,
    /// The project of the search.
    pub project: ProjectId,
    /// The fields it may change.
    pub fields: Vec<RuleField>,
    /// The project in which it may create issues.
    pub creates_in: Option<ProjectId>,
    /// Whether it still works.
    pub state: GrantState,
    /// When it ends.
    pub ends_at: Timestamp,
    /// The version of the record.
    pub version: i64,
}

/// Apply or discard a proposal of a standing rule.
pub struct DecideProposal {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The proposal.
    pub proposal: AgentNoteId,
    /// Apply (`true`) or discard (`false`).
    pub apply: bool,
}

/// What an agent reports on its attempt.
pub struct ReportAttempt {
    /// The secret of the key of the agent.
    pub session_token: String,
    /// The issue, by its ID as people write it.
    pub issue: String,
    /// The plan, when the agent has one.
    pub plan: Option<String>,
    /// The minutes it spent, as it reports them.
    pub minutes: i64,
    /// The cost in cents, as it reports it.
    pub cost: i64,
}

/// A question of an agent.
pub struct AskQuestion {
    /// The secret of the key of the agent.
    pub session_token: String,
    /// The issue.
    pub issue: String,
    /// What it asks.
    pub text: String,
}

/// A result an agent attaches.
pub struct AttachResult {
    /// The secret of the key of the agent.
    pub session_token: String,
    /// The issue.
    pub issue: String,
    /// A link or a check.
    pub kind: ResultKind,
    /// How the agent calls it.
    pub label: String,
    /// The link.
    pub link: Option<String>,
    /// The outcome of a check.
    pub outcome: Option<CheckResult>,
}

/// An agent says it is done with the attempt, or gives up.
pub struct EndAttempt {
    /// The secret of the key of the agent.
    pub session_token: String,
    /// The issue.
    pub issue: String,
    /// The reason, when the agent gives up; empty for a result that is ready.
    pub gave_up: Option<String>,
}
