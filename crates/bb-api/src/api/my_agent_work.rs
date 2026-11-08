//! The work of a person's own agents in one list (AI-37, *My agents*): `/api/v1/me/agents/work`.
//!
//! Every route starts at a session: an agent, a key or an app never lists a person's agents.

use axum::extract::State;
use axum::response::Response;
use bb_app::agent_work::{MyAgentWork, MyAgentWorkQuery};
use bb_core::agent_work::WaitsFor;
use bb_policy::{Masked, PersonRef};
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::agent_work_bodies::{StandingRuleItem, WorkBody};
use super::auth::{
    AuthHandle, ForbiddenAnswer, Info, NotSignedInAnswer, Session, answer_authorized,
};
use super::relations::IssueRef;
use crate::errors::ApiError;

/// One piece of work of one of the person's agents, with its issue.
#[derive(Serialize, ToSchema)]
pub(crate) struct MyWorkBody {
    /// The work, with its brief, attempts, questions and reviews.
    work: WorkBody,
    /// The issue, as the person may see it.
    issue: IssueRef,
    /// What the work waits for, when it waits for the person.
    waits_for: Option<WaitsFor>,
}

impl Masked for MyWorkBody {}

/// An agent another person handed to this person, who has not accepted it yet.
#[derive(Serialize, ToSchema)]
pub(crate) struct OfferedAgentBody {
    /// The ID of the agent.
    id: String,
    /// Its name.
    name: String,
    /// The person who hands it over, as *Who sees people* says for the person asking.
    from: PersonRef,
}

/// The current work of the person's agents.
#[derive(Serialize, ToSchema)]
pub(crate) struct MyAgentWorkBody {
    /// The agents handed to the person that wait for them to accept.
    offered: Vec<OfferedAgentBody>,
    /// Whether the agents handed to the person were cut at the most an agent list holds.
    offered_truncated: bool,
    /// The work, newest first: what has not ended, and what was accepted by itself and not looked
    /// at yet.
    pieces: Vec<MyWorkBody>,
    /// Whether the list was cut at the most it holds.
    truncated: bool,
    /// The standing rules of the person's agents that work.
    rules: Vec<StandingRuleItem>,
    /// Whether the rules were cut at the most it holds.
    rules_truncated: bool,
}

impl Masked for MyAgentWorkBody {}

impl From<&MyAgentWork> for MyAgentWorkBody {
    fn from(all: &MyAgentWork) -> Self {
        Self {
            offered: all
                .offered
                .iter()
                .map(|agent| OfferedAgentBody {
                    id: agent.id.to_string(),
                    name: agent.name.clone(),
                    from: agent.from.clone(),
                })
                .collect(),
            offered_truncated: all.offered_truncated,
            rules_truncated: all.rules_truncated,
            rules: all.rules.iter().map(StandingRuleItem::from).collect(),
            pieces: all
                .items
                .iter()
                .map(|item| MyWorkBody {
                    work: WorkBody::from(&item.work),
                    issue: IssueRef::from(&item.issue),
                    waits_for: item.waits_for,
                })
                .collect(),
            truncated: all.truncated,
        }
    }
}

/// The work of the person's own agents: what waits for the person, the work in progress, and the
/// work that was accepted by itself and nobody has looked at.
#[utoipa::path(
    get,
    path = "/api/v1/me/agents/work",
    operation_id = "listMyAgentWork",
    tag = "agent-work",
    responses((status = 200, description = "The work of the person's agents.", body = MyAgentWorkBody), NotSignedInAnswer, ForbiddenAnswer)
)]
async fn list_my_agent_work(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let found = auth
        .api()?
        .my_agent_work(info, MyAgentWorkQuery { session_token })
        .await?;
    answer_authorized(&found.map(|all| MyAgentWorkBody::from(&all)))
}

/// The routes of the work of one's own agents.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new().routes(routes!(list_my_agent_work))
}
