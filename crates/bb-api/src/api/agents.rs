//! Agents (AI-30, ADR-0029): the routes of a person's own agents below `/api/v1/agents`, the list of
//! every agent for administrators below `/api/v1/organization/agents`, and *Stop all my agents*
//! at `/api/v1/me/agents/stop-all`.
//!
//! Making, changing and handing agents start at a session: an agent, a key or an app never makes or
//! changes agents. The one route an agent key may call is the agent's own record (`getAgent`).
//! The keys of an agent are made and taken back below the agent; making one asks for the password
//! again and answers the secret once.

use axum::extract::State;
use axum::response::Response;
use bb_app::agents::{
    AcceptAgent, AgentAction, AgentPage, AgentQuery, AgentView, AgentsQuery, ChangeAgent,
    CreateAgent, HandAgent, RenameAgent, StopAllAgents,
};
use bb_app::auth::{AppError, CreateAccessKey, DeleteAccessKey};
use bb_core::ErrorCode;
use bb_core::access_key::KeyChoice;
use bb_core::agent::{AgentState, LimitKind};
use bb_core::id::{AccessKeyId, AgentId, UserId};
use bb_policy::{Authorized, Masked, PersonRef};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::access_bodies::cursor_error;
use super::access_keys::{AccessKeyBody, CreatedAccessKeyBody};
use super::auth::{
    AuthHandle, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer, NotSignedInAnswer, Session,
    answer_authorized, done,
};
use super::{NameBody, body_id, path_id};
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};
use crate::lean::{FieldSpec, ListParams, Listed, Page};

/// The fields of an agent that can be asked for, and which of them a list holds when the request
/// names none.
const AGENT_FIELDS: [FieldSpec; 11] = [
    FieldSpec::by_default("id"),
    FieldSpec::by_default("name"),
    FieldSpec::by_default("state"),
    FieldSpec::by_default("paused_for"),
    FieldSpec::by_default("person"),
    FieldSpec::by_default("changes_a_minute"),
    FieldSpec::by_default("issues_at_once"),
    FieldSpec::by_default("attempts_on_one_issue"),
    FieldSpec::by_default("offered"),
    FieldSpec::by_default("version"),
    FieldSpec::on_request("created_at"),
    FieldSpec::on_request("last_used_at"),
];

/// An agent in a list: the fields that can be asked for. An answer holds only those that were
/// asked for.
#[derive(Serialize, ToSchema)]
pub(crate) struct AgentItem {
    /// The ID of the agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    /// Its name.
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    /// Whether it may act: `active`, `paused` or `stopped`.
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<AgentState>,
    /// The limit the agent reached when it was paused, so a person can read why: `changes-a-minute`,
    /// `issues-at-once` or `attempts-on-one-issue`. Left out when it was not paused by a limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    paused_for: Option<LimitKind>,
    /// The person responsible for it, as *Who sees people* says for the person asking.
    #[serde(skip_serializing_if = "Option::is_none")]
    person: Option<PersonRef>,
    /// Changes it may make in one minute, at most.
    #[serde(skip_serializing_if = "Option::is_none")]
    changes_a_minute: Option<i64>,
    /// Issues it may work on at once, at most.
    #[serde(skip_serializing_if = "Option::is_none")]
    issues_at_once: Option<i64>,
    /// Attempts it may make on one issue, at most.
    #[serde(skip_serializing_if = "Option::is_none")]
    attempts_on_one_issue: Option<i64>,
    /// Whether it is offered to another person who has not accepted yet.
    #[serde(skip_serializing_if = "Option::is_none")]
    offered: Option<bool>,
    /// The version of the agent; send it back with a change.
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<i64>,
    /// When it was created.
    #[serde(skip_serializing_if = "Option::is_none")]
    created_at: Option<String>,
    /// When it last signed in with a key; left out when it never did. Asked for by name.
    #[serde(skip_serializing_if = "Option::is_none")]
    last_used_at: Option<String>,
}

impl Masked for AgentItem {}

impl From<&AgentView> for AgentItem {
    fn from(agent: &AgentView) -> Self {
        Self {
            id: Some(agent.id.to_string()),
            name: Some(agent.name.clone()),
            state: Some(agent.state),
            paused_for: agent.paused_for,
            person: Some(agent.person.clone()),
            changes_a_minute: Some(agent.limits.changes_a_minute),
            issues_at_once: Some(agent.limits.issues_at_once),
            attempts_on_one_issue: Some(agent.limits.attempts_on_one_issue),
            offered: Some(agent.offered),
            version: Some(agent.version),
            created_at: Some(agent.made_at.to_string()),
            last_used_at: agent.last_used_at.map(|at| at.to_string()),
        }
    }
}

/// One agent with its keys (at most `MOST_AGENT_KEYS`, newest first).
#[derive(Serialize, ToSchema)]
pub(crate) struct AgentBody {
    #[serde(flatten)]
    agent: AgentItem,
    /// The keys of the agent. Never the secret.
    keys: Vec<AccessKeyBody>,
}

impl Masked for AgentBody {}

impl From<&AgentView> for AgentBody {
    fn from(agent: &AgentView) -> Self {
        Self {
            agent: AgentItem::from(agent),
            keys: agent.keys.iter().map(AccessKeyBody::from).collect(),
        }
    }
}

/// What to do to an agent: give it another name, or pause, resume or stop it. Exactly one of
/// `name` and `action`.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ChangeAgentBody {
    /// The new name.
    #[serde(default)]
    name: Option<String>,
    /// `pause`, `resume` or `stop`.
    #[serde(default)]
    action: Option<AgentActionBody>,
    /// The version the person saw.
    version: i64,
}

/// What to do to the state of an agent.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum AgentActionBody {
    /// Let it rest.
    Pause,
    /// Let a paused agent work again.
    Resume,
    /// Stop it for good.
    Stop,
}

/// An agent to hand to another person, who has to accept it.
#[derive(Deserialize, ToSchema)]
pub(crate) struct HandAgentBody {
    /// The ID of the person.
    to: String,
    /// The version the person saw.
    version: i64,
}

/// An agent key to renew: the new key keeps the name of the old one, which is taken back.
#[derive(Deserialize, ToSchema)]
pub(crate) struct RenewAgentKeyBody {
    /// The password of the person, asked again.
    password: String,
    /// When the new key stops working. Left out: the default number of days from now.
    #[serde(default)]
    expires_at: Option<String>,
}

/// An agent key to make.
#[derive(Deserialize, ToSchema)]
pub(crate) struct CreateAgentKeyBody {
    /// The password of the person, asked again.
    password: String,
    /// What the key is called.
    name: String,
    /// When it stops working (a date and time such as `2027-03-01T00:00:00Z`). Left out: the
    /// default number of days from now. It cannot be later than the longest time.
    #[serde(default)]
    expires_at: Option<String>,
}

/// The name of the lists of agents, so their cursors are their own.
const AGENTS_LIST: &str = "agents";

/// The agents of the person, a page at a time.
#[utoipa::path(
    get,
    path = "/api/v1/agents",
    operation_id = "listMyAgents",
    tag = "agents",
    params(ListParams),
    responses((status = 200, description = "The agents of the person, by ID.", body = Page<AgentItem>), NotSignedInAnswer, ForbiddenAnswer, InvalidAnswer)
)]
async fn list_my_agents(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    let (cursor, bad_cursor) = params.start(AGENTS_LIST);
    let page = auth
        .api()?
        .my_agents(
            info,
            AgentsQuery {
                session_token,
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(cursor_error)?;
    bad_cursor?;
    answer_page(&params, page)
}

/// The answer of a list of agents: the page, with the fields that were asked for.
fn answer_page(params: &ListParams, page: Authorized<AgentPage>) -> Result<Response, ApiError> {
    let list = page.map(|page| Listed {
        items: page.items.iter().map(AgentItem::from).collect(),
        next: page.next_cursor,
    });
    let fields = params.fields_readable(&AGENT_FIELDS, &list.hidden_properties())?;
    Ok(super::auth::answer(Page::of_authorized(
        AGENTS_LIST,
        &list,
        &fields,
    )?))
}

/// Makes an agent. The person must hold *Create agents that work for you* in at least one project.
#[utoipa::path(
    post,
    path = "/api/v1/agents",
    operation_id = "createAgent",
    tag = "agents",
    request_body = NameBody,
    responses(
        (status = 200, description = "The agent.", body = AgentBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer
    )
)]
async fn create_agent(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    body: Result<ApiJson<NameBody>, ApiError>,
) -> Result<Response, ApiError> {
    let ApiJson(body) = auth.after_who_asks(&info, &session_token, body).await?;
    let made = auth
        .api()?
        .create_agent(
            info,
            CreateAgent {
                session_token,
                name: body.name,
            },
        )
        .await?;
    answer_authorized(&made.map(|agent| AgentBody::from(&agent)))
}

/// One agent: its state, its limits and its keys. An agent reads only its own record.
#[utoipa::path(
    get,
    path = "/api/v1/agents/{agent}",
    operation_id = "getAgent",
    tag = "agents",
    params(("agent" = String, Path, description = "The ID of the agent.")),
    responses((status = 200, description = "The agent.", body = AgentBody), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn get_agent(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(agent_id): ApiPath<String>,
) -> Result<Response, ApiError> {
    let id: AgentId = auth
        .after_who_asks(&info, &session_token, path_id(&agent_id))
        .await?;
    let found = auth
        .api()?
        .agent(info, AgentQuery { session_token, id })
        .await?;
    answer_authorized(&found.map(|agent| AgentBody::from(&agent)))
}

/// Gives an agent another name, or pauses, resumes or stops it. An administrator may stop any
/// agent.
#[utoipa::path(
    patch,
    path = "/api/v1/agents/{agent}",
    operation_id = "changeAgent",
    tag = "agents",
    params(("agent" = String, Path, description = "The ID of the agent.")),
    request_body = ChangeAgentBody,
    responses(
        (status = 200, description = "The agent.", body = AgentBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn change_agent(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(agent_id): ApiPath<String>,
    body: Result<ApiJson<ChangeAgentBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = body.and_then(|ApiJson(body)| Ok((body, path_id::<AgentId>(&agent_id)?)));
    let (body, id) = auth.after_who_asks(&info, &session_token, read).await?;
    let api = auth.api()?;
    let changed = match (body.name, body.action) {
        (Some(name), None) => {
            api.rename_agent(
                info,
                RenameAgent {
                    session_token,
                    id,
                    name,
                    version: body.version,
                },
            )
            .await?
        }
        (None, Some(action)) => {
            let action = match action {
                AgentActionBody::Pause => AgentAction::Pause,
                AgentActionBody::Resume => AgentAction::Resume,
                AgentActionBody::Stop => AgentAction::Stop,
            };
            api.change_agent(
                info,
                ChangeAgent {
                    session_token,
                    id,
                    action,
                    version: body.version,
                },
            )
            .await?
        }
        _ => return Err(ApiError::from(AppError::Refused(ErrorCode::InvalidRequest))),
    };
    answer_authorized(&changed.map(|agent| AgentBody::from(&agent)))
}

/// Offers an agent to another person, who has to accept it.
#[utoipa::path(
    post,
    path = "/api/v1/agents/{agent}/hand-over",
    operation_id = "handAgent",
    tag = "agents",
    params(("agent" = String, Path, description = "The ID of the agent.")),
    request_body = HandAgentBody,
    responses((status = 204, description = "Offered."), InvalidAnswer, NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn hand_agent(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(agent_id): ApiPath<String>,
    body: Result<ApiJson<HandAgentBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = body.and_then(|ApiJson(body)| {
        let to: UserId = body_id(&body.to)?;
        Ok((body, path_id::<AgentId>(&agent_id)?, to))
    });
    let (body, id, to) = auth.after_who_asks(&info, &session_token, read).await?;
    auth.api()?
        .hand_agent(
            info,
            HandAgent {
                session_token,
                id,
                to,
                version: body.version,
            },
        )
        .await?;
    Ok(done())
}

/// Accepts an agent that was handed to the person. Its keys are taken back.
#[utoipa::path(
    post,
    path = "/api/v1/agents/{agent}/accept",
    operation_id = "acceptAgent",
    tag = "agents",
    params(("agent" = String, Path, description = "The ID of the agent.")),
    responses((status = 204, description = "Accepted."), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn accept_agent(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(agent_id): ApiPath<String>,
) -> Result<Response, ApiError> {
    let id: AgentId = auth
        .after_who_asks(&info, &session_token, path_id(&agent_id))
        .await?;
    auth.api()?
        .accept_agent(info, AcceptAgent { session_token, id })
        .await?;
    Ok(done())
}

/// Makes a key for an agent. The key is shown once, runs out, and is always the agent's.
#[utoipa::path(
    post,
    path = "/api/v1/agents/{agent}/keys",
    operation_id = "createAgentKey",
    tag = "agents",
    params(("agent" = String, Path, description = "The ID of the agent.")),
    request_body = CreateAgentKeyBody,
    responses(
        (status = 200, description = "The key, with its secret.", body = CreatedAccessKeyBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn create_agent_key(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(agent_id): ApiPath<String>,
    body: Result<ApiJson<CreateAgentKeyBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = body.and_then(|ApiJson(body)| Ok((body, path_id::<AgentId>(&agent_id)?)));
    let (body, agent) = auth.after_who_asks(&info, &session_token, read).await?;
    let created = auth
        .api()?
        .create_access_key(
            info,
            CreateAccessKey {
                session_token,
                password: body.password,
                name: body.name,
                // What an agent may do is its work, cut from its person (`Narrowing::agent`).
                choice: KeyChoice::SeeAndChange,
                permissions: Vec::new(),
                projects: Vec::new(),
                expires_at: body.expires_at,
                agent: Some(agent),
                renews: None,
            },
        )
        .await?;
    answer_authorized(&created.map(CreatedAccessKeyBody::from))
}

/// Renews a key of an agent: a new key with the name of the old one, which is taken back in the same
/// change. The new key is shown once.
#[utoipa::path(
    post,
    path = "/api/v1/agents/{agent}/keys/{access_key}/renew",
    operation_id = "renewAgentKey",
    tag = "agents",
    params(
        ("agent" = String, Path, description = "The ID of the agent."),
        ("access_key" = String, Path, description = "The ID of the key to renew.")
    ),
    request_body = RenewAgentKeyBody,
    responses(
        (status = 200, description = "The new key, with its secret.", body = CreatedAccessKeyBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn renew_agent_key(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath((agent_id, key_id)): ApiPath<(String, String)>,
    body: Result<ApiJson<RenewAgentKeyBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = body.and_then(|ApiJson(body)| {
        Ok((
            body,
            path_id::<AgentId>(&agent_id)?,
            path_id::<AccessKeyId>(&key_id)?,
        ))
    });
    let (body, agent, old) = auth.after_who_asks(&info, &session_token, read).await?;
    let created = auth
        .api()?
        .create_access_key(
            info,
            CreateAccessKey {
                session_token,
                password: body.password,
                name: String::new(),
                choice: KeyChoice::SeeAndChange,
                permissions: Vec::new(),
                projects: Vec::new(),
                expires_at: body.expires_at,
                agent: Some(agent),
                renews: Some(old),
            },
        )
        .await?;
    answer_authorized(&created.map(CreatedAccessKeyBody::from))
}

/// Takes a key of an agent back at once.
#[utoipa::path(
    delete,
    path = "/api/v1/agents/{agent}/keys/{access_key}",
    operation_id = "deleteAgentKey",
    tag = "agents",
    params(
        ("agent" = String, Path, description = "The ID of the agent."),
        ("access_key" = String, Path, description = "The ID of the key.")
    ),
    responses((status = 204, description = "Taken back."), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn delete_agent_key(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath((agent_id, key_id)): ApiPath<(String, String)>,
) -> Result<Response, ApiError> {
    let read = path_id::<AgentId>(&agent_id)
        .and_then(|agent| Ok((agent, path_id::<AccessKeyId>(&key_id)?)));
    let (agent, id) = auth.after_who_asks(&info, &session_token, read).await?;
    auth.api()?
        .delete_access_key(
            info,
            DeleteAccessKey {
                session_token,
                id,
                agent: Some(agent),
            },
        )
        .await?;
    Ok(done())
}

/// *Stop all my agents*: every agent of the person that is not stopped yet.
#[utoipa::path(
    post,
    path = "/api/v1/me/agents/stop-all",
    operation_id = "stopAllMyAgents",
    tag = "agents",
    responses((status = 204, description = "Stopped."), NotSignedInAnswer, ForbiddenAnswer)
)]
async fn stop_all_my_agents(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    auth.api()?
        .stop_all_agents(info, StopAllAgents { session_token })
        .await?;
    Ok(done())
}

/// Every agent of the organization, for administrators.
#[utoipa::path(
    get,
    path = "/api/v1/organization/agents",
    operation_id = "listOrganizationAgents",
    tag = "agents",
    params(ListParams),
    responses((status = 200, description = "Every agent, by ID.", body = Page<AgentItem>), NotSignedInAnswer, ForbiddenAnswer, InvalidAnswer)
)]
async fn list_organization_agents(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    let (cursor, bad_cursor) = params.start(AGENTS_LIST);
    let page = auth
        .api()?
        .organization_agents(
            info,
            AgentsQuery {
                session_token,
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(cursor_error)?;
    bad_cursor?;
    answer_page(&params, page)
}

/// The routes of agents.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_my_agents, create_agent))
        .routes(routes!(get_agent, change_agent))
        .routes(routes!(hand_agent))
        .routes(routes!(accept_agent))
        .routes(routes!(create_agent_key))
        .routes(routes!(renew_agent_key))
        .routes(routes!(delete_agent_key))
        .routes(routes!(stop_all_my_agents))
        .routes(routes!(list_organization_agents))
}
