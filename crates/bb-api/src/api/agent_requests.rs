//! Requests for agents of programs (ADR-0029, amendment M9): the person's list of them, *Approve*
//! with the password once, and *Decline*. They start at a session: a key of any kind never answers
//! a request. Programs ask and collect through the MCP tools `ask_for_agents` and
//! `collect_agent_keys`.

use axum::extract::State;
use axum::response::Response;
use bb_app::agents::{
    AgentRequestPage, AgentRequestView, AgentRequestsQuery, ApproveAgentRequests,
    DeclineAgentRequest,
};
use bb_core::agent_request::RequestStands;
use bb_core::id::AgentRequestId;
use bb_policy::Masked;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer, NotSignedInAnswer, Session,
    answer_authorized,
};
use super::{body_ids, path_id};
use crate::errors::{ApiError, ApiJson, ApiPath};

/// One agent a program asks for.
#[derive(Serialize, ToSchema)]
pub(crate) struct AskedAgentBody {
    /// The name the agent would have.
    name: String,
    /// What it is for, in a few words.
    purpose: String,
    /// How many days its key should last; left out for the default of the organization.
    #[serde(skip_serializing_if = "Option::is_none")]
    key_days: Option<i64>,
    /// The agent the approval made; left out before.
    #[serde(skip_serializing_if = "Option::is_none")]
    agent: Option<String>,
}

/// A request for agents as its person sees it.
#[derive(Serialize, ToSchema)]
pub(crate) struct AgentRequestBody {
    /// The ID of the request.
    id: String,
    /// The name of the app or of the key that asked.
    asked_by: String,
    /// Whether an AI app asked, not a key.
    by_app: bool,
    /// The web address the app comes back to; none for a key and for an app on the person's
    /// computer.
    web_address: Option<String>,
    /// Why it asked.
    reason: String,
    /// The agents it asks for.
    agents: Vec<AskedAgentBody>,
    /// Where it stands.
    stands: RequestStands,
    /// When it was asked.
    asked_at: String,
    /// Until when it waits for an answer, or for its keys once approved.
    waits_until: String,
}

impl Masked for AgentRequestBody {}

impl From<&AgentRequestView> for AgentRequestBody {
    fn from(view: &AgentRequestView) -> Self {
        Self {
            id: view.id.to_string(),
            asked_by: view.asked_by.clone(),
            by_app: view.by_app,
            web_address: view.web_address.clone(),
            reason: view.reason.clone(),
            agents: view
                .agents
                .iter()
                .map(|asked| AskedAgentBody {
                    name: asked.name.clone(),
                    purpose: asked.purpose.clone(),
                    key_days: asked.key_days,
                    agent: asked.agent.map(|agent| agent.to_string()),
                })
                .collect(),
            stands: view.stands,
            asked_at: view.asked_at.to_string(),
            waits_until: view.waits_until.to_string(),
        }
    }
}

/// The requests for agents of a person, newest first.
#[derive(Serialize, ToSchema)]
pub(crate) struct AgentRequestsBody {
    /// The requests of the last week, at most 100.
    requests: Vec<AgentRequestBody>,
    /// Whether there were more than the list shows.
    truncated: bool,
}

impl Masked for AgentRequestsBody {}

impl From<AgentRequestPage> for AgentRequestsBody {
    fn from(page: AgentRequestPage) -> Self {
        Self {
            requests: page.items.iter().map(AgentRequestBody::from).collect(),
            truncated: page.truncated,
        }
    }
}

/// The requests to approve, and the password, asked again once for all of them.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ApproveAgentRequestsBody {
    /// The password of the person, asked again.
    password: String,
    /// The IDs of the requests, as the person ticked them.
    requests: Vec<String>,
}

/// The requests for agents asked of the person in the last week, newest first.
#[utoipa::path(
    get,
    path = "/api/v1/agents/requests",
    operation_id = "listAgentRequests",
    tag = "agents",
    responses((status = 200, description = "The requests, newest first.", body = AgentRequestsBody), NotSignedInAnswer, ForbiddenAnswer)
)]
async fn list_agent_requests(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let page = auth
        .api()?
        .agent_requests(info, AgentRequestsQuery { session_token })
        .await?;
    answer_authorized(&page.map(AgentRequestsBody::from))
}

/// Approves requests for agents with the password once: every agent of them is made in one change,
/// and their keys wait for the program that asked. When one cannot be made, none is.
#[utoipa::path(
    post,
    path = "/api/v1/agents/requests/approve",
    operation_id = "approveAgentRequests",
    tag = "agents",
    request_body = ApproveAgentRequestsBody,
    responses(
        (status = 200, description = "The requests, approved.", body = AgentRequestsBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn approve_agent_requests(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    body: Result<ApiJson<ApproveAgentRequestsBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = body.and_then(|ApiJson(body)| {
        let requests = body_ids::<AgentRequestId>(&body.requests)?;
        Ok((body.password, requests))
    });
    let (password, requests) = auth.after_who_asks(&info, &session_token, read).await?;
    let page = auth
        .api()?
        .approve_agent_requests(
            info,
            ApproveAgentRequests {
                session_token,
                password,
                requests,
            },
        )
        .await?;
    answer_authorized(&page.map(AgentRequestsBody::from))
}

/// Declines a request for agents: it ends, and nothing is made.
#[utoipa::path(
    post,
    path = "/api/v1/agents/requests/{request}/decline",
    operation_id = "declineAgentRequest",
    tag = "agents",
    params(("request" = String, Path, description = "The ID of the request.")),
    responses(
        (status = 200, description = "The request, declined.", body = AgentRequestBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn decline_agent_request(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(request): ApiPath<String>,
) -> Result<Response, ApiError> {
    let id: AgentRequestId = auth
        .after_who_asks(&info, &session_token, path_id(&request))
        .await?;
    let view = auth
        .api()?
        .decline_agent_request(info, DeclineAgentRequest { session_token, id })
        .await?;
    answer_authorized(&view.map(|view| AgentRequestBody::from(&view)))
}

/// The routes of requests for agents.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_agent_requests))
        .routes(routes!(approve_agent_requests))
        .routes(routes!(decline_agent_request))
}
