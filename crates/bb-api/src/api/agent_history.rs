//! The history of an agent (AI-37, ADR-0029, amendment M9): `/api/v1/agents/{agent}/history`, what
//! the agent handed over to its person's other agents and which of its runs it undid, newest first.
//!
//! It starts at a session, and only the agent's person reads it.

use axum::extract::State;
use axum::response::Response;
use bb_app::agents::{AgentDeed, AgentHistoryItem, AgentHistoryPage, AgentHistoryQuery};
use bb_core::id::AgentId;
use bb_policy::{Authorized, Masked};
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::access_bodies::{NamedRef, cursor_error};
use super::auth::{
    AuthHandle, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer, NotSignedInAnswer, Session,
};
use super::path_id;
use super::relations::IssueRef;
use crate::errors::{ApiError, ApiPath, ApiQuery};
use crate::lean::{FieldSpec, ListParams, Listed, Page};

/// What an agent did.
#[derive(Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum AgentDeedKind {
    /// It handed issues of its work over to another agent of its person.
    HandOver,
    /// It undid the run of one of its attempts.
    Undo,
}

/// One entry of the history of an agent.
#[derive(Serialize, ToSchema)]
pub(crate) struct AgentHistoryItemBody {
    /// What the entry is known by: the ID of the hand-over, or of the event of the undo.
    id: String,
    /// What the agent did.
    kind: AgentDeedKind,
    /// When.
    at: String,
    /// For a hand-over, the agent it handed the issues to; left out when the reader may not be told
    /// that agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    to: Option<NamedRef>,
    /// The issues: those a hand-over handed on, or the issue of the run that was undone; only
    /// those the reader may see.
    issues: Vec<IssueRef>,
    /// For an undo, how many changes went back.
    #[serde(skip_serializing_if = "Option::is_none")]
    went_back: Option<u64>,
}

// The issues are lines, each dropped by its own project; the agent is named only to a reader
// who is told its person.
impl Masked for AgentHistoryItemBody {}

impl From<&AgentHistoryItem> for AgentHistoryItemBody {
    fn from(item: &AgentHistoryItem) -> Self {
        let (at, id) = (item.at.to_string(), item.id.to_string());
        match &item.deed {
            AgentDeed::HandedOver { to, issues } => Self {
                id,
                kind: AgentDeedKind::HandOver,
                at,
                to: to.as_ref().map(|to| NamedRef {
                    id: to.id.to_string(),
                    name: to.name.clone(),
                }),
                issues: issues.iter().map(IssueRef::from).collect(),
                went_back: None,
            },
            AgentDeed::Undid { issue, went_back } => Self {
                id,
                kind: AgentDeedKind::Undo,
                at,
                to: None,
                issues: issue.iter().map(IssueRef::from).collect(),
                went_back: Some(*went_back),
            },
        }
    }
}

/// The fields of an entry of the history that can be asked for.
const HISTORY_FIELDS: [FieldSpec; 6] = [
    FieldSpec::by_default("id"),
    FieldSpec::by_default("kind"),
    FieldSpec::by_default("at"),
    FieldSpec::by_default("to"),
    FieldSpec::by_default("issues"),
    FieldSpec::by_default("went_back"),
];

/// The name of the lists of the history of agents, so their cursors are their own.
const HISTORY_LIST: &str = "agent-history";

/// The history of an agent: its hand-overs to its person's other agents and its undos, newest
/// first, a page at a time.
#[utoipa::path(
    get,
    path = "/api/v1/agents/{agent}/history",
    operation_id = "listAgentHistory",
    tag = "agents",
    params(("agent" = String, Path, description = "The ID of the agent."), ListParams),
    responses((status = 200, description = "What the agent did, newest first.", body = Page<AgentHistoryItemBody>), NotSignedInAnswer, ForbiddenAnswer, InvalidAnswer, NotFoundAnswer)
)]
async fn list_agent_history(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(agent_id): ApiPath<String>,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    let agent: AgentId = auth
        .after_who_asks(&info, &session_token, path_id(&agent_id))
        .await?;
    let (cursor, bad_cursor) = params.start(HISTORY_LIST);
    let page = auth
        .api()?
        .agent_history(
            info,
            AgentHistoryQuery {
                session_token,
                agent,
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(cursor_error)?;
    bad_cursor?;
    answer_history(&params, page)
}

/// The answer of the history: the page, with the fields that were asked for.
fn answer_history(
    params: &ListParams,
    page: Authorized<AgentHistoryPage>,
) -> Result<Response, ApiError> {
    let list = page.map(|page| Listed {
        items: page.items.iter().map(AgentHistoryItemBody::from).collect(),
        next: page.next_cursor,
    });
    let fields = params.fields_readable(&HISTORY_FIELDS, &list.hidden_properties())?;
    Ok(super::auth::answer(Page::of_authorized(
        HISTORY_LIST,
        &list,
        &fields,
    )?))
}

/// The route of the history of an agent.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new().routes(routes!(list_agent_history))
}
