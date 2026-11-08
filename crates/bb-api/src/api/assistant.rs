//! The assistant (AI-10, AI-05, COMP-08, ADR-0013): a question that is answered and may come back with
//! proposed changes, and *Apply* for one of them. The conversation is the browser's: the server
//! stores none of it (M7-Q6), so every question carries the turns so far.

use axum::extract::State;
use axum::response::Response;
use bb_app::ai::assistant::{
    Applied, ApplyProposal, AskAssistantQuery, AssistantAnswer, AssistantStatus,
    AssistantStatusQuery, ChatTurn, LookingAt, Proposal,
};
use bb_app::ai::tools::ProposedChange;
use bb_core::ai::{ChatFrom, Destination, LookingAtKind};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer_authorized,
};
use crate::errors::{ApiError, ApiJson, ApiQuery};

/// One thing the panel looks at.
#[derive(Deserialize, ToSchema)]
pub(crate) struct AssistantLookingAtBody {
    /// What kind of thing it is.
    kind: LookingAtKind,
    /// An issue by its readable ID, a project by its short name, or the search of a list.
    value: String,
}

/// One turn of the conversation so far.
#[derive(Deserialize, ToSchema)]
pub(crate) struct AssistantTurnBody {
    /// Who said it.
    from: ChatFrom,
    /// What was said.
    text: String,
}

/// A question to the assistant.
#[derive(Deserialize, ToSchema)]
pub(crate) struct AssistantAskBody {
    /// What the person typed.
    text: String,
    /// What the panel looks at, at most five things; the person can take each away.
    #[serde(default)]
    looking_at: Vec<AssistantLookingAtBody>,
    /// The conversation so far, at most twenty turns. The server keeps none of it.
    #[serde(default)]
    turns: Vec<AssistantTurnBody>,
}

/// One field a proposed or applied change touches.
#[derive(Serialize, ToSchema)]
pub(crate) struct ProposedChangeBody {
    /// The field, like `title`, `priority` or `status`.
    field: String,
    /// The value now; absent when the field is new.
    #[schema(value_type = Option<Object>)]
    before: Option<serde_json::Value>,
    /// The value after; absent when the change takes the value away.
    #[schema(value_type = Option<Object>)]
    after: Option<serde_json::Value>,
}

impl From<&ProposedChange> for ProposedChangeBody {
    fn from(change: &ProposedChange) -> Self {
        Self {
            field: change.field.clone(),
            before: change.before.clone(),
            after: change.after.clone(),
        }
    }
}

/// A change the assistant proposes. Nothing is saved until the person applies it.
#[derive(Serialize, ToSchema)]
pub(crate) struct AssistantProposalBody {
    /// The signed reference that *Apply* takes back with the tool and the input.
    reference: String,
    /// The name of the tool.
    tool: String,
    /// The input of the tool, exactly as it was previewed. Send it back unchanged.
    #[schema(value_type = Object)]
    input: serde_json::Value,
    /// The fields it would change, with the value now and the value after.
    changes: Vec<ProposedChangeBody>,
}

impl From<&Proposal> for AssistantProposalBody {
    fn from(proposal: &Proposal) -> Self {
        Self {
            reference: proposal.reference.clone(),
            tool: proposal.tool.clone(),
            input: proposal.input.clone(),
            changes: proposal
                .changes
                .iter()
                .map(ProposedChangeBody::from)
                .collect(),
        }
    }
}

/// The answer of the assistant.
#[derive(Serialize, ToSchema)]
pub(crate) struct AssistantAnswerBody {
    /// The answer in Markdown.
    text: String,
    /// The same, written by the one Markdown writer with the option for AI answers: no picture, and
    /// every link with its full address.
    html: String,
    /// The changes it proposes.
    proposals: Vec<AssistantProposalBody>,
}

impl bb_policy::Masked for AssistantAnswerBody {}

impl From<&AssistantAnswer> for AssistantAnswerBody {
    fn from(said: &AssistantAnswer) -> Self {
        Self {
            text: said.text.clone(),
            html: said.html.clone(),
            proposals: said
                .proposals
                .iter()
                .map(AssistantProposalBody::from)
                .collect(),
        }
    }
}

/// A proposal to apply, as the assistant gave it.
#[derive(Deserialize, ToSchema)]
pub(crate) struct AssistantApplyBody {
    /// The reference of the proposal.
    reference: String,
    /// The tool of the proposal.
    tool: String,
    /// Its input, unchanged.
    #[schema(value_type = Object)]
    input: serde_json::Value,
}

/// What *Apply* saved.
#[derive(Serialize, ToSchema)]
pub(crate) struct AppliedProposalBody {
    /// The fields that were changed, with the value before and after.
    changes: Vec<ProposedChangeBody>,
}

impl bb_policy::Masked for AppliedProposalBody {}

impl From<&Applied> for AppliedProposalBody {
    fn from(applied: &Applied) -> Self {
        Self {
            changes: applied
                .changes
                .iter()
                .map(ProposedChangeBody::from)
                .collect(),
        }
    }
}

/// Asks the assistant a question about what the person looks at. It reads as the person, may
/// propose changes, and saves nothing. While AI does not count at the place it answers *AI is off*
/// and sends nothing.
#[utoipa::path(
    post,
    path = "/api/v1/assistant/questions",
    operation_id = "askAssistant",
    tag = "ai",
    request_body = AssistantAskBody,
    responses(
        (status = 200, description = "The answer and the changes it proposes.", body = AssistantAnswerBody),
        (status = 409, description = "AI is off here (ai.off), or the provider is not ready (ai.agreement-needed, ai.not-set-up).", body = bb_core::Problem, content_type = "application/problem+json"),
        (status = 502, description = "The AI provider did not answer (ai.unavailable).", body = bb_core::Problem, content_type = "application/problem+json"),
        InvalidAnswer,
        NotSignedInAnswer,
        NotFoundAnswer
    )
)]
async fn ask_assistant(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<AssistantAskBody>,
) -> Result<Response, ApiError> {
    let answer = auth
        .api()?
        .ask_assistant(
            info,
            AskAssistantQuery {
                session_token,
                text: body.text,
                looking_at: body
                    .looking_at
                    .into_iter()
                    .map(|each| LookingAt {
                        kind: each.kind,
                        value: each.value,
                    })
                    .collect(),
                turns: body
                    .turns
                    .into_iter()
                    .map(|turn| ChatTurn {
                        from: turn.from,
                        text: turn.text,
                    })
                    .collect(),
            },
        )
        .await?;
    answer_authorized(&answer.map(|answer| AssistantAnswerBody::from(&answer)))
}

/// Saves a change the assistant proposed, exactly as it was shown, as the person and with the
/// mark that the assistant wrote it. A version that moved on is refused with the newer value, as for
/// a person.
#[utoipa::path(
    post,
    path = "/api/v1/assistant/apply",
    operation_id = "applyAssistantProposal",
    tag = "ai",
    request_body = AssistantApplyBody,
    responses(
        (status = 200, description = "What was saved.", body = AppliedProposalBody),
        (status = 409, description = "The reference does not hold or ran out (ai.proposal-invalid), or the issue changed since (version.changed).", body = bb_core::Problem, content_type = "application/problem+json"),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn apply_assistant_proposal(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<AssistantApplyBody>,
) -> Result<Response, ApiError> {
    let applied = auth
        .api()?
        .apply_proposal(
            info,
            ApplyProposal {
                session_token,
                reference: body.reference,
                tool: body.tool,
                input: body.input,
            },
        )
        .await?;
    answer_authorized(&applied.map(|applied| AppliedProposalBody::from(&applied)))
}

/// Where the person looks, for the question *is the assistant there*.
#[derive(Deserialize, utoipa::IntoParams)]
pub(crate) struct AssistantStatusParams {
    /// The project the person looks at: its ID or short name. Left out, the question is about the
    /// organization.
    project: Option<String>,
}

/// Whether the assistant is there at a place, and where its answers come from.
#[derive(Serialize, ToSchema)]
pub(crate) struct AssistantStatusBody {
    /// AI counts at the place: the organization and the project both have it on. When it is
    /// `false` the button, the panel and the entries of quick search are not shown.
    available: bool,
    /// Where the data goes; missing while the assistant is not there.
    #[serde(skip_serializing_if = "Option::is_none")]
    destination: Option<Destination>,
    /// Whether people go to the provider by name; otherwise as their role.
    names_sent: bool,
}

impl bb_policy::Masked for AssistantStatusBody {}

impl From<&AssistantStatus> for AssistantStatusBody {
    fn from(status: &AssistantStatus) -> Self {
        Self {
            available: status.available,
            destination: status.destination,
            names_sent: status.names_sent,
        }
    }
}

/// Whether the assistant is there for the person at a place. It reads the settings and sends
/// nothing; a project the person may not see, or that is kept from AI, answers *not found*.
#[utoipa::path(
    get,
    path = "/api/v1/assistant",
    operation_id = "getAssistantStatus",
    tag = "ai",
    params(AssistantStatusParams),
    responses(
        (status = 200, description = "Whether the assistant is there.", body = AssistantStatusBody),
        NotSignedInAnswer,
        NotFoundAnswer
    )
)]
async fn get_assistant_status(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<AssistantStatusParams>,
) -> Result<Response, ApiError> {
    let status = auth
        .api()?
        .assistant_status(
            info,
            AssistantStatusQuery {
                session_token,
                project: params.project,
            },
        )
        .await?;
    answer_authorized(&status.map(|status| AssistantStatusBody::from(&status)))
}

pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(get_assistant_status))
        .routes(routes!(ask_assistant))
        .routes(routes!(apply_assistant_proposal))
}
