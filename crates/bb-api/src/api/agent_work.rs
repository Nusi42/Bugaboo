//! Hand-overs, agent work and standing rules (AI-31 to AI-35, ADR-0029): the routes people use.
//!
//! The routes an agent itself calls (take, report, ask, attach, give up) are use cases in
//! `bb-app` and tools of the MCP server, not routes here. Every route starts at a session: an
//! agent, a key or an app never hands work over or reviews it.

use axum::extract::State;
use axum::response::Response;
use bb_app::agent_work::{
    AgentWorkQuery, AnswerQuestion, ApprovePlan, ChangeStandingRule, CreateStandingRule,
    DecideProposal, HandOver, LookAtWork, ReviewWork, StandingRulePage, StandingRulesQuery,
};
use bb_core::ErrorCode;
use bb_core::id::{AgentId, AgentNoteId, AgentWorkId, ProjectId, SavedSearchId, StandingRuleId};
use bb_core::workflow::StatusKey;
use bb_policy::Authorized;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::access_bodies::cursor_error;
use super::agent_work_bodies::{
    AnswerBody, ApprovePlanBody, ChangeRuleBody, HandOverAnswer, HandOverBody, IssueAgentWorkBody,
    ProposalBody, ProposalDecisionBody, ReviewBody, RuleAction, StandingRuleBody, StandingRuleItem,
    WorkBody,
};
use super::auth::{
    AuthHandle, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer, NotSignedInAnswer, Session,
    answer_authorized,
};
use super::{body_id, path_id};
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};
use crate::lean::{FieldSpec, ListParams, Listed, Page};

/// The fields of a standing rule that can be asked for.
const RULE_FIELDS: [FieldSpec; 9] = [
    FieldSpec::by_default("id"),
    FieldSpec::by_default("agent"),
    FieldSpec::by_default("search"),
    FieldSpec::by_default("project"),
    FieldSpec::by_default("fields"),
    FieldSpec::by_default("creates_in"),
    FieldSpec::by_default("state"),
    FieldSpec::by_default("ends_at"),
    FieldSpec::by_default("version"),
];

/// The name of the lists of standing rules, so their cursors are their own.
const RULES_LIST: &str = "standing-rules";

/// Hands one or several issues over to one of the person's agents. The person needs *Change
/// issues* (or *Change the issues they reported*) on each issue and *Create agents that work for
/// you* in its project.
#[utoipa::path(
    post,
    path = "/api/v1/hand-overs",
    operation_id = "handOver",
    tag = "agent-work",
    request_body = HandOverBody,
    responses(
        (status = 200, description = "The hand-over.", body = HandOverAnswer),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn hand_over(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    body: Result<ApiJson<HandOverBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = body.and_then(|ApiJson(body)| {
        let agent: AgentId = body_id(&body.agent)?;
        Ok((body, agent))
    });
    let (body, agent) = auth.after_who_asks(&info, &session_token, read).await?;
    let made = auth
        .api()?
        .hand_over(
            info,
            HandOver {
                session_token,
                agent,
                issues: body.issues,
                note: body.note,
                powers: body.powers,
                fields: body.fields,
                budget_attempts: body.budget_attempts,
                budget_minutes: body.budget_minutes,
                budget_cost: body.budget_cost,
                plan_needs_approval: body.plan_needs_approval,
                accept_by_itself: body.accept_by_itself,
                days: body.days,
            },
        )
        .await?;
    answer_authorized(&made.map(|view| HandOverAnswer::from(&view)))
}

/// The agent work on an issue: who works on it, the brief, the attempts, the questions, the
/// reviews and the proposals that wait for a person.
#[utoipa::path(
    get,
    path = "/api/v1/issues/{issue}/agent-work",
    operation_id = "getIssueAgentWork",
    tag = "agent-work",
    params(("issue" = String, Path, description = "The readable or internal ID of the issue.")),
    responses((status = 200, description = "The agent work.", body = IssueAgentWorkBody), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn get_issue_agent_work(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue): ApiPath<String>,
) -> Result<Response, ApiError> {
    let found = auth
        .api()?
        .issue_agent_work(
            info,
            AgentWorkQuery {
                session_token,
                issue,
            },
        )
        .await?;
    answer_authorized(&found.map(|all| IssueAgentWorkBody::from(&all)))
}

/// Answers a question of an agent. The answer joins the brief.
#[utoipa::path(
    post,
    path = "/api/v1/agent-work/{work}/answer",
    operation_id = "answerAgentQuestion",
    tag = "agent-work",
    params(("work" = String, Path, description = "The ID of the work.")),
    request_body = AnswerBody,
    responses(
        (status = 200, description = "The work.", body = WorkBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn reply_to_agent_question(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(work_id): ApiPath<String>,
    body: Result<ApiJson<AnswerBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = body.and_then(|ApiJson(body)| {
        let question: AgentNoteId = body_id(&body.question)?;
        Ok((body, path_id::<AgentWorkId>(&work_id)?, question))
    });
    let (body, work, question) = auth.after_who_asks(&info, &session_token, read).await?;
    let answered = auth
        .api()?
        .answer_question(
            info,
            AnswerQuestion {
                session_token,
                work,
                question,
                reply: body.reply,
                version: body.version,
            },
        )
        .await?;
    answer_authorized(&answered.map(|view| WorkBody::from(&view)))
}

/// *Approve plan*: the attempt may start. A note instead answers the plan.
#[utoipa::path(
    post,
    path = "/api/v1/agent-work/{work}/approve-plan",
    operation_id = "approveAgentPlan",
    tag = "agent-work",
    params(("work" = String, Path, description = "The ID of the work.")),
    request_body = ApprovePlanBody,
    responses(
        (status = 200, description = "The work.", body = WorkBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn approve_agent_plan(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(work_id): ApiPath<String>,
    body: Result<ApiJson<ApprovePlanBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = body.and_then(|ApiJson(body)| Ok((body, path_id::<AgentWorkId>(&work_id)?)));
    let (body, work) = auth.after_who_asks(&info, &session_token, read).await?;
    let approved = auth
        .api()?
        .approve_plan(
            info,
            ApprovePlan {
                session_token,
                work,
                version: body.version,
                note: body.note,
            },
        )
        .await?;
    answer_authorized(&approved.map(|view| WorkBody::from(&view)))
}

/// The review of a result: *Accept* ends the hand-over, *Send back* starts another attempt with
/// the note in the brief.
#[utoipa::path(
    post,
    path = "/api/v1/agent-work/{work}/review",
    operation_id = "reviewAgentWork",
    tag = "agent-work",
    params(("work" = String, Path, description = "The ID of the work.")),
    request_body = ReviewBody,
    responses(
        (status = 200, description = "The work.", body = WorkBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn review_agent_work(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(work_id): ApiPath<String>,
    body: Result<ApiJson<ReviewBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = body.and_then(|ApiJson(body)| {
        let status = body
            .status
            .as_deref()
            .map(|key| StatusKey::new(key).map_err(|_| ApiError::new(ErrorCode::InvalidRequest)))
            .transpose()?;
        Ok((body, path_id::<AgentWorkId>(&work_id)?, status))
    });
    let (body, work, status) = auth.after_who_asks(&info, &session_token, read).await?;
    let reviewed = auth
        .api()?
        .review_work(
            info,
            ReviewWork {
                session_token,
                work,
                verdict: body.verdict,
                note: body.note,
                status,
                version: body.version,
            },
        )
        .await?;
    answer_authorized(&reviewed.map(|view| WorkBody::from(&view)))
}

/// Marks work that was accepted by itself as looked at.
#[utoipa::path(
    post,
    path = "/api/v1/agent-work/{work}/looked-at",
    operation_id = "lookAtAgentWork",
    tag = "agent-work",
    params(("work" = String, Path, description = "The ID of the work.")),
    responses((status = 200, description = "The work.", body = WorkBody), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn look_at_agent_work(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(work_id): ApiPath<String>,
) -> Result<Response, ApiError> {
    let work: AgentWorkId = auth
        .after_who_asks(&info, &session_token, path_id(&work_id))
        .await?;
    let seen = auth
        .api()?
        .look_at_work(
            info,
            LookAtWork {
                session_token,
                work,
            },
        )
        .await?;
    answer_authorized(&seen.map(|view| WorkBody::from(&view)))
}

/// Makes a standing rule for one of the person's agents over a saved search.
#[utoipa::path(
    post,
    path = "/api/v1/standing-rules",
    operation_id = "createStandingRule",
    tag = "agent-work",
    request_body = StandingRuleBody,
    responses(
        (status = 200, description = "The rule.", body = StandingRuleItem),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn create_standing_rule(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    body: Result<ApiJson<StandingRuleBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = body.and_then(|ApiJson(body)| {
        let agent: AgentId = body_id(&body.agent)?;
        let search: SavedSearchId = body_id(&body.search)?;
        let project: ProjectId = body_id(&body.project)?;
        let creates_in = body
            .creates_in
            .as_deref()
            .map(body_id::<ProjectId>)
            .transpose()?;
        Ok((body, agent, search, project, creates_in))
    });
    let (body, agent, search, project, creates_in) =
        auth.after_who_asks(&info, &session_token, read).await?;
    let made = auth
        .api()?
        .create_standing_rule(
            info,
            CreateStandingRule {
                session_token,
                agent,
                search,
                project,
                way: body.way,
                powers: body.powers,
                fields: body.fields,
                accept_by_itself: body.accept_by_itself,
                creates_in,
                days: body.days,
                never_ends: body.never_ends,
            },
        )
        .await?;
    answer_authorized(&made.map(|rule| StandingRuleItem::from(&rule)))
}

/// Renews or stops a standing rule.
#[utoipa::path(
    patch,
    path = "/api/v1/standing-rules/{rule}",
    operation_id = "changeStandingRule",
    tag = "agent-work",
    params(("rule" = String, Path, description = "The ID of the rule.")),
    request_body = ChangeRuleBody,
    responses(
        (status = 200, description = "The rule.", body = StandingRuleItem),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn change_standing_rule(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(rule_id): ApiPath<String>,
    body: Result<ApiJson<ChangeRuleBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = body.and_then(|ApiJson(body)| Ok((body, path_id::<StandingRuleId>(&rule_id)?)));
    let (body, rule) = auth.after_who_asks(&info, &session_token, read).await?;
    let changed = auth
        .api()?
        .change_standing_rule(
            info,
            ChangeStandingRule {
                session_token,
                rule,
                renew: matches!(body.action, RuleAction::Renew),
                days: body.days,
                never_ends: body.never_ends,
                version: body.version,
            },
        )
        .await?;
    answer_authorized(&changed.map(|rule| StandingRuleItem::from(&rule)))
}

/// The standing rules of one of the person's agents, a page at a time.
#[utoipa::path(
    get,
    path = "/api/v1/agents/{agent}/standing-rules",
    operation_id = "listStandingRules",
    tag = "agent-work",
    params(("agent" = String, Path, description = "The ID of the agent."), ListParams),
    responses((status = 200, description = "The rules, by ID.", body = Page<StandingRuleItem>), NotSignedInAnswer, ForbiddenAnswer, InvalidAnswer, NotFoundAnswer)
)]
async fn list_standing_rules(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(agent_id): ApiPath<String>,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    let agent: AgentId = auth
        .after_who_asks(&info, &session_token, path_id(&agent_id))
        .await?;
    let (cursor, bad_cursor) = params.start(RULES_LIST);
    let page = auth
        .api()?
        .standing_rules(
            info,
            StandingRulesQuery {
                session_token,
                agent,
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(cursor_error)?;
    bad_cursor?;
    answer_rules(&params, page)
}

/// The answer of a list of standing rules: the page, with the fields that were asked for.
fn answer_rules(
    params: &ListParams,
    page: Authorized<StandingRulePage>,
) -> Result<Response, ApiError> {
    let list = page.map(|page| Listed {
        items: page.items.iter().map(StandingRuleItem::from).collect(),
        next: page.next_cursor,
    });
    let fields = params.fields_readable(&RULE_FIELDS, &list.hidden_properties())?;
    Ok(super::auth::answer(Page::of_authorized(
        RULES_LIST, &list, &fields,
    )?))
}

/// Applies or discards a proposal of a standing rule. Nothing changes before a person applies it.
#[utoipa::path(
    post,
    path = "/api/v1/proposals/{proposal}/decision",
    operation_id = "decideProposal",
    tag = "agent-work",
    params(("proposal" = String, Path, description = "The ID of the proposal.")),
    request_body = ProposalDecisionBody,
    responses(
        (status = 200, description = "The proposal.", body = ProposalBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn decide_proposal(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(proposal_id): ApiPath<String>,
    body: Result<ApiJson<ProposalDecisionBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = body.and_then(|ApiJson(body)| Ok((body, path_id::<AgentNoteId>(&proposal_id)?)));
    let (body, proposal) = auth.after_who_asks(&info, &session_token, read).await?;
    let decided = auth
        .api()?
        .decide_proposal(
            info,
            DecideProposal {
                session_token,
                proposal,
                apply: body.apply,
            },
        )
        .await?;
    answer_authorized(&decided.map(|view| ProposalBody::from(&view)))
}

/// The routes of hand-overs, agent work and standing rules.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(hand_over))
        .routes(routes!(get_issue_agent_work))
        .routes(routes!(reply_to_agent_question))
        .routes(routes!(approve_agent_plan))
        .routes(routes!(review_agent_work))
        .routes(routes!(look_at_agent_work))
        .routes(routes!(create_standing_rule))
        .routes(routes!(change_standing_rule))
        .routes(routes!(list_standing_rules))
        .routes(routes!(decide_proposal))
}
