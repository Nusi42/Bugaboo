//! *Create several agents* (AI-30, ADR-0029 amendment M9): `POST /api/v1/agents/several` makes every
//! agent of a list with one key each, in one change, after the password once. It starts at a session:
//! a key of any kind never makes agents or keys. The keys are answered once.

use axum::extract::State;
use axum::response::Response;
use bb_app::agents::{CreateSeveralAgents, CreatedAgent, CreatedAgents, SeveralAgent};
use bb_policy::Masked;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::access_keys::CreatedAccessKeyBody;
use super::agents::AgentBody;
use super::auth::{
    AuthHandle, ForbiddenAnswer, Info, InvalidAnswer, NotSignedInAnswer, Session, answer_authorized,
};
use crate::errors::{ApiError, ApiJson};

/// One agent of the list.
#[derive(Deserialize, ToSchema)]
pub(crate) struct SeveralAgentBody {
    /// What the agent is called; unique among the person's agents. Its key is called the same.
    name: String,
    /// When its key stops working (a date and time such as `2027-03-01T00:00:00Z`). Left out: the
    /// default number of days from now. It cannot be later than the longest time.
    #[serde(default)]
    expires_at: Option<String>,
}

/// The agents to make, and the password of the person, asked again.
#[derive(Deserialize, ToSchema)]
pub(crate) struct CreateSeveralAgentsBody {
    /// The password of the person, asked again once for every key.
    password: String,
    /// The agents, in the order they are shown; at least one and at most `agentsAtOnceMost`.
    agents: Vec<SeveralAgentBody>,
}

/// An agent that was made, with its key.
#[derive(Serialize, ToSchema)]
pub(crate) struct CreatedAgentBody {
    /// The agent.
    agent: AgentBody,
    /// Its key, with the secret: it is not stored and cannot be shown again.
    key: CreatedAccessKeyBody,
}

/// The agents that were made, in the order of the list.
#[derive(Serialize, ToSchema)]
pub(crate) struct CreatedAgentsBody {
    /// The agents with their keys.
    items: Vec<CreatedAgentBody>,
}

impl Masked for CreatedAgentsBody {}

impl From<CreatedAgents> for CreatedAgentsBody {
    fn from(made: CreatedAgents) -> Self {
        Self {
            items: made
                .items
                .into_iter()
                .map(|CreatedAgent { agent, key }| CreatedAgentBody {
                    agent: AgentBody::from(&agent),
                    key: CreatedAccessKeyBody::from(key),
                })
                .collect(),
        }
    }
}

/// Makes several agents, each with one key, after the password once. When one row cannot be made,
/// none is, and the answer names the row (`agents/<row>/name` or `agents/<row>/expires_at`).
#[utoipa::path(
    post,
    path = "/api/v1/agents/several",
    operation_id = "createSeveralAgents",
    tag = "agents",
    request_body = CreateSeveralAgentsBody,
    responses(
        (status = 200, description = "The agents with their keys, shown once.", body = CreatedAgentsBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer
    )
)]
async fn create_several_agents(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    body: Result<ApiJson<CreateSeveralAgentsBody>, ApiError>,
) -> Result<Response, ApiError> {
    let ApiJson(body) = auth.after_who_asks(&info, &session_token, body).await?;
    let made = auth
        .api()?
        .create_several_agents(
            info,
            CreateSeveralAgents {
                session_token,
                password: body.password,
                agents: body
                    .agents
                    .into_iter()
                    .map(|row| SeveralAgent {
                        name: row.name,
                        expires_at: row.expires_at,
                    })
                    .collect(),
            },
        )
        .await?;
    answer_authorized(&made.map(CreatedAgentsBody::from))
}

/// The route of several agents.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new().routes(routes!(create_several_agents))
}
