//! The bodies of the routes of hand-overs, agent work and standing rules (AI-31 to AI-35).

use bb_app::agent_work::{
    AttemptView, HandOverView, HandedIssue, IssueAgentWork, ProposalView, QuestionView, ResultView,
    ReviewView, StandingRuleView, WorkBudget, WorkView, WorkedOnBy,
};
use bb_core::agent_records::{GivingState, ProposalState, ResultKind};
use bb_core::agent_work::{
    AgentWorkState, BriefPart, CheckResult, HandOverPower, ReviewAnswer, RuleField,
};
use bb_policy::{Masked, PersonRef};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Hand issues over to one of the person's agents.
#[derive(Deserialize, ToSchema)]
pub(crate) struct HandOverBody {
    /// The ID of the agent.
    pub(super) agent: String,
    /// The issues, by their IDs as people write them (`WEB-142`).
    pub(super) issues: Vec<String>,
    /// The note of the brief.
    #[serde(default)]
    pub(super) note: String,
    /// What the agent may do besides reading, commenting, reporting and attaching.
    #[serde(default)]
    pub(super) powers: Vec<HandOverPower>,
    /// The members of the issue the agent may change, when it may change fields.
    #[serde(default)]
    pub(super) fields: Vec<String>,
    /// The most attempts on one issue; the agent's own limit when left out.
    #[serde(default)]
    pub(super) budget_attempts: Option<i64>,
    /// The most minutes the agent reports.
    #[serde(default)]
    pub(super) budget_minutes: Option<i64>,
    /// The most cost, in cents, the agent reports.
    #[serde(default)]
    pub(super) budget_cost: Option<i64>,
    /// Whether each attempt waits for *Approve plan*.
    #[serde(default)]
    pub(super) plan_needs_approval: bool,
    /// Whether an attempt may be accepted by itself when the project allows it.
    #[serde(default)]
    pub(super) accept_by_itself: bool,
    /// After how many days the hand-over ends; the setting of the organization when left out.
    #[serde(default)]
    pub(super) days: Option<i64>,
}

/// One issue of a hand-over.
#[derive(Serialize, ToSchema)]
pub(crate) struct HandedIssueBody {
    /// The ID of the work on the issue.
    pub(super) work: String,
    /// The issue.
    pub(super) issue: String,
    /// The parts of the brief the issue lacks. It does not stop the hand-over.
    pub(super) missing: Vec<BriefPart>,
}

impl From<&HandedIssue> for HandedIssueBody {
    fn from(handed: &HandedIssue) -> Self {
        Self {
            work: handed.work.to_string(),
            issue: handed.issue.to_string(),
            missing: handed.missing.clone(),
        }
    }
}

/// A hand-over as it was made.
#[derive(Serialize, ToSchema)]
pub(crate) struct HandOverAnswer {
    /// The ID of the hand-over.
    pub(super) id: String,
    /// The agent.
    pub(super) agent: String,
    /// The issues.
    pub(super) issues: Vec<HandedIssueBody>,
    /// When it ends.
    pub(super) ends_at: String,
    /// Whether it still gives work.
    pub(super) state: GivingState,
    /// The version of the hand-over.
    pub(super) version: i64,
}

impl Masked for HandOverAnswer {}

impl From<&HandOverView> for HandOverAnswer {
    fn from(view: &HandOverView) -> Self {
        Self {
            id: view.id.to_string(),
            agent: view.agent.to_string(),
            issues: view.issues.iter().map(HandedIssueBody::from).collect(),
            ends_at: view.ends_at.to_string(),
            state: view.state,
            version: view.version,
        }
    }
}

/// What an attempt attached.
#[derive(Serialize, ToSchema)]
pub(crate) struct ResultBody {
    /// A link or a check.
    pub(super) kind: ResultKind,
    /// How the agent calls it.
    pub(super) label: String,
    /// The link.
    pub(super) link: Option<String>,
    /// The outcome of a check. Bugaboo does not run or verify checks.
    pub(super) outcome: Option<CheckResult>,
}

/// One attempt. The minutes and the cost are what the agent reports, never a time entry.
#[derive(Serialize, ToSchema)]
pub(crate) struct AttemptBody {
    /// The ID of the attempt.
    pub(super) id: String,
    /// Its number on the issue.
    pub(super) number: i64,
    /// The plan the agent reported.
    pub(super) plan: Option<String>,
    /// Whether a person approved the plan.
    pub(super) plan_approved: bool,
    /// The minutes the agent reports.
    pub(super) reported_minutes: i64,
    /// The cost in cents the agent reports.
    pub(super) reported_cost: i64,
    /// When it started.
    pub(super) started_at: String,
    /// When it ended.
    pub(super) ended_at: Option<String>,
    /// What it attached.
    pub(super) results: Vec<ResultBody>,
    /// The run of its changes; a person who may open runs reads and undoes it there.
    pub(super) run: Option<String>,
}

impl From<&AttemptView> for AttemptBody {
    fn from(attempt: &AttemptView) -> Self {
        Self {
            id: attempt.id.to_string(),
            number: attempt.number,
            plan: attempt.plan.clone(),
            plan_approved: attempt.plan_approved,
            reported_minutes: attempt.reported_minutes,
            reported_cost: attempt.reported_cost,
            started_at: attempt.started_at.to_string(),
            ended_at: attempt.ended_at.map(|at| at.to_string()),
            results: attempt.results.iter().map(ResultBody::from).collect(),
            run: attempt.run.map(|run| run.to_string()),
        }
    }
}

impl From<&ResultView> for ResultBody {
    fn from(result: &ResultView) -> Self {
        Self {
            kind: result.kind,
            label: result.label.clone(),
            link: result.link.clone(),
            outcome: result.outcome,
        }
    }
}

/// A question of an agent with its answer.
#[derive(Serialize, ToSchema)]
pub(crate) struct QuestionBody {
    /// The ID of the question.
    pub(super) id: String,
    /// What the agent asks.
    pub(super) text: String,
    /// The answer, which joins the brief.
    #[serde(rename = "answer")]
    pub(super) reply: Option<String>,
    /// When it was asked.
    pub(super) asked_at: String,
}

impl From<&QuestionView> for QuestionBody {
    fn from(question: &QuestionView) -> Self {
        Self {
            id: question.id.to_string(),
            text: question.text.clone(),
            reply: question.answer.clone(),
            asked_at: question.asked_at.to_string(),
        }
    }
}

/// A review.
#[derive(Serialize, ToSchema)]
pub(crate) struct ReviewAnswerBody {
    /// The answer.
    #[serde(rename = "answer")]
    pub(super) verdict: ReviewAnswer,
    /// The note.
    pub(super) note: Option<String>,
    /// Who reviewed, as *Who sees people* says for the reader; nobody when it was accepted by
    /// itself.
    pub(super) by: Option<PersonRef>,
    /// When.
    pub(super) at: String,
}

impl From<&ReviewView> for ReviewAnswerBody {
    fn from(review: &ReviewView) -> Self {
        Self {
            verdict: review.answer,
            note: review.note.clone(),
            by: review.by.clone(),
            at: review.at.to_string(),
        }
    }
}

/// What a hand-over lets the agent spend, and whether its plan waits for a person.
#[derive(Serialize, ToSchema)]
pub(crate) struct WorkBudgetBody {
    /// The most attempts on the issue.
    pub(super) attempts: i64,
    /// The most minutes the agent may report, when limited.
    pub(super) minutes: Option<i64>,
    /// The most cost in cents the agent may report, when limited.
    pub(super) cost: Option<i64>,
    /// Whether each attempt waits for *Approve plan*.
    pub(super) plan_needs_approval: bool,
}

impl From<&WorkBudget> for WorkBudgetBody {
    fn from(budget: &WorkBudget) -> Self {
        Self {
            attempts: budget.attempts,
            minutes: budget.minutes,
            cost: budget.cost,
            plan_needs_approval: budget.plan_needs_approval,
        }
    }
}

/// The agent work on one issue of one hand-over.
#[derive(Serialize, ToSchema)]
pub(crate) struct WorkBody {
    /// The ID of the work.
    pub(super) id: String,
    /// The hand-over.
    pub(super) hand_over: String,
    /// The agent.
    pub(super) agent: String,
    /// The name of the agent.
    pub(super) agent_name: String,
    /// The person responsible for the agent, as *Who sees people* says for the reader.
    pub(super) agent_person: PersonRef,
    /// Who handed the issue over, as *Who sees people* says for the reader.
    pub(super) handed_over_by: PersonRef,
    /// When the issue was handed over.
    pub(super) handed_over_at: String,
    /// When the hand-over ends.
    pub(super) ends_at: String,
    /// What the hand-over lets the agent spend.
    pub(super) budget: WorkBudgetBody,
    /// Where the work is. It is not the status of the issue.
    pub(super) state: AgentWorkState,
    /// Whether the hand-over still gives work.
    pub(super) open: bool,
    /// The note of the brief, then the answers to the questions.
    pub(super) brief: Vec<String>,
    /// The attempts, oldest first.
    pub(super) attempts: Vec<AttemptBody>,
    /// The questions, oldest first.
    pub(super) questions: Vec<QuestionBody>,
    /// The reviews, oldest first.
    pub(super) reviews: Vec<ReviewAnswerBody>,
    /// Why the agent gave up.
    pub(super) gave_up: Option<String>,
    /// Whether it was accepted by itself and nobody looked yet.
    pub(super) accepted_by_itself: bool,
    /// Whether the issue keeps its status after an accept by itself and a person chooses the next
    /// one.
    pub(super) choose_next_status: bool,
    /// The version of the work; send it back with an answer.
    pub(super) version: i64,
}

impl Masked for WorkBody {}

impl From<&WorkView> for WorkBody {
    fn from(work: &WorkView) -> Self {
        Self {
            id: work.id.to_string(),
            hand_over: work.hand_over.to_string(),
            agent: work.agent.to_string(),
            agent_name: work.agent_name.clone(),
            agent_person: work.agent_person.clone(),
            handed_over_by: work.handed_over_by.clone(),
            handed_over_at: work.handed_over_at.to_string(),
            ends_at: work.ends_at.to_string(),
            budget: WorkBudgetBody::from(&work.budget),
            state: work.state,
            open: work.open,
            brief: work.brief.clone(),
            attempts: work.attempts.iter().map(AttemptBody::from).collect(),
            questions: work.questions.iter().map(QuestionBody::from).collect(),
            reviews: work.reviews.iter().map(ReviewAnswerBody::from).collect(),
            gave_up: work.gave_up.clone(),
            accepted_by_itself: work.accepted_by_itself,
            choose_next_status: work.choose_next_status,
            version: work.version,
        }
    }
}

/// An agent shown on an issue as *worked on by*.
#[derive(Serialize, ToSchema)]
pub(crate) struct WorkedOnByBody {
    /// The agent.
    pub(super) agent: String,
    /// Its name.
    pub(super) name: String,
    /// The person responsible for it, as *Who sees people* says for the reader.
    pub(super) person: PersonRef,
}

impl From<&WorkedOnBy> for WorkedOnByBody {
    fn from(worked: &WorkedOnBy) -> Self {
        Self {
            agent: worked.agent.to_string(),
            name: worked.name.clone(),
            person: worked.person.clone(),
        }
    }
}

/// A change a standing rule proposes.
#[derive(Serialize, ToSchema)]
pub(crate) struct ProposalBody {
    /// The ID of the proposal.
    pub(super) id: String,
    /// The rule.
    pub(super) rule: String,
    /// The agent.
    pub(super) agent: String,
    /// The field.
    pub(super) field: RuleField,
    /// The value now.
    #[schema(value_type = Object)]
    pub(super) before: serde_json::Value,
    /// The value proposed.
    #[schema(value_type = Object)]
    pub(super) after: serde_json::Value,
    /// What became of it.
    pub(super) state: ProposalState,
    /// When it was proposed.
    pub(super) proposed_at: String,
}

impl Masked for ProposalBody {}

impl From<&ProposalView> for ProposalBody {
    fn from(proposal: &ProposalView) -> Self {
        Self {
            id: proposal.id.to_string(),
            rule: proposal.rule.to_string(),
            agent: proposal.agent.to_string(),
            field: proposal.field,
            before: proposal.before.clone(),
            after: proposal.after.clone(),
            state: proposal.state,
            proposed_at: proposal.made_at.to_string(),
        }
    }
}

/// The agent work on an issue.
#[derive(Serialize, ToSchema)]
pub(crate) struct IssueAgentWorkBody {
    /// The agents with open work on the issue. The issue keeps its person: an agent is never
    /// *Assigned to*.
    pub(super) worked_on_by: Vec<WorkedOnByBody>,
    /// All the work, newest first.
    pub(super) work: Vec<WorkBody>,
    /// The proposals of standing rules that wait for a person.
    pub(super) proposals: Vec<ProposalBody>,
    /// Whether the lists were cut at the most an issue shows.
    pub(super) truncated: bool,
    /// The parts of a brief the issue lacks now, so a hand-over can say it before it is made.
    pub(super) brief_missing: Vec<BriefPart>,
}

impl Masked for IssueAgentWorkBody {}

impl From<&IssueAgentWork> for IssueAgentWorkBody {
    fn from(all: &IssueAgentWork) -> Self {
        Self {
            worked_on_by: all.worked_on_by.iter().map(WorkedOnByBody::from).collect(),
            work: all.work.iter().map(WorkBody::from).collect(),
            proposals: all.proposals.iter().map(ProposalBody::from).collect(),
            truncated: all.truncated,
            brief_missing: all.brief_missing.clone(),
        }
    }
}

/// Answer a question of an agent.
#[derive(Deserialize, ToSchema)]
pub(crate) struct AnswerBody {
    /// The ID of the question.
    pub(super) question: String,
    /// The answer, which joins the brief.
    #[serde(rename = "answer")]
    pub(super) reply: String,
    /// The version of the work the person saw.
    pub(super) version: i64,
}

/// *Approve plan*, or a note instead.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ApprovePlanBody {
    /// A note that answers the plan instead of approving it.
    #[serde(default)]
    pub(super) note: Option<String>,
    /// The version of the work the person saw.
    pub(super) version: i64,
}

/// The review of a result.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ReviewBody {
    /// `accept` or `send-back`.
    #[serde(rename = "answer")]
    pub(super) verdict: ReviewAnswer,
    /// The note; with `send-back` it joins the brief and is needed.
    #[serde(default)]
    pub(super) note: Option<String>,
    /// With `accept`: the status the issue moves to, by its key.
    #[serde(default)]
    pub(super) status: Option<String>,
    /// The version of the work the person saw.
    pub(super) version: i64,
}

/// Make a standing rule.
#[derive(Deserialize, ToSchema)]
pub(crate) struct StandingRuleBody {
    /// The ID of the agent.
    pub(super) agent: String,
    /// The ID of the saved search.
    pub(super) search: String,
    /// The ID of the project the rule works in.
    pub(super) project: String,
    /// The fields the rule may change; none means it only proposes.
    #[serde(default)]
    pub(super) fields: Vec<RuleField>,
    /// The project in which the rule may create issues.
    #[serde(default)]
    pub(super) creates_in: Option<String>,
    /// After how many days the rule ends (at most 90); the setting of the organization when left
    /// out.
    #[serde(default)]
    pub(super) days: Option<i64>,
}

/// A standing rule.
#[derive(Serialize, ToSchema)]
pub(crate) struct StandingRuleItem {
    /// The ID of the rule.
    pub(super) id: String,
    /// The agent.
    pub(super) agent: String,
    /// The saved search.
    pub(super) search: String,
    /// The project of the rule.
    pub(super) project: String,
    /// The fields it may change.
    pub(super) fields: Vec<RuleField>,
    /// The project in which it may create issues.
    pub(super) creates_in: Option<String>,
    /// Whether it still works.
    pub(super) state: GivingState,
    /// When it ends.
    pub(super) ends_at: String,
    /// The version of the rule; send it back with a change.
    pub(super) version: i64,
}

impl Masked for StandingRuleItem {}

impl From<&StandingRuleView> for StandingRuleItem {
    fn from(rule: &StandingRuleView) -> Self {
        Self {
            id: rule.id.to_string(),
            agent: rule.agent.to_string(),
            search: rule.search.to_string(),
            project: rule.project.to_string(),
            fields: rule.fields.clone(),
            creates_in: rule.creates_in.map(|project| project.to_string()),
            state: rule.state,
            ends_at: rule.ends_at.to_string(),
            version: rule.version,
        }
    }
}

/// *Renew* a standing rule, or stop it.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ChangeRuleBody {
    /// `renew` or `stop`.
    pub(super) action: RuleAction,
    /// With `renew`: after how many days the rule ends (at most 90).
    #[serde(default)]
    pub(super) days: Option<i64>,
    /// The version the person saw.
    pub(super) version: i64,
}

/// What to do to a standing rule.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum RuleAction {
    /// A new end, counted from now.
    Renew,
    /// End it now.
    Stop,
}

/// Apply or discard a proposal.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ProposalDecisionBody {
    /// `true` applies the proposal; `false` discards it.
    pub(super) apply: bool,
}
