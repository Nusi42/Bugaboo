//! *Never sent to AI* (AI-03, ADR-0013): which built-in fields and projects the assistant, AI apps
//! and agents never read. Anybody signed in may read the marks; only administrators set them
//! (*Change organization settings* for a field, *Change project settings* for a project). An own
//! field carries its mark in its definition (`never_to_assistant`, with the fields).

use axum::extract::State;
use axum::response::Response;
use bb_app::ai::{NeverSentList, NeverSentQuery, NeverSentTarget, SetNeverSent};
use bb_core::fields::Field;
use bb_policy::Masked;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer_authorized,
};
use super::projects::ProjectRef;
use crate::errors::{ApiError, ApiJson, ApiPath};

/// One built-in field and whether it is kept from AI.
#[derive(Serialize, ToSchema)]
pub(crate) struct NeverSentFieldBody {
    /// The key of the field, like `description`.
    field: String,
    /// Whether it is kept from AI.
    marked: bool,
    /// The version to change it on; 0 for a field that never had a mark.
    version: i64,
}

/// One project that has a mark.
#[derive(Serialize, ToSchema)]
pub(crate) struct NeverSentProjectBody {
    /// The project.
    project: ProjectRef,
    /// Whether it is kept from AI, with every project inside it.
    marked: bool,
    /// The version to change it on.
    version: i64,
}

/// The marks the person may know about.
#[derive(Serialize, ToSchema)]
pub(crate) struct NeverSentBody {
    /// Every built-in field, in the order of the catalogue.
    fields: Vec<NeverSentFieldBody>,
    /// The projects that have a mark and that the person sees, at most 200.
    projects: Vec<NeverSentProjectBody>,
    /// Whether more projects than that were left out.
    truncated: bool,
}

impl Masked for NeverSentBody {}

impl From<&NeverSentList> for NeverSentBody {
    fn from(list: &NeverSentList) -> Self {
        Self {
            fields: list
                .fields
                .iter()
                .map(|each| NeverSentFieldBody {
                    field: each.field.code().to_owned(),
                    marked: each.marked,
                    version: each.version,
                })
                .collect(),
            projects: list
                .projects
                .iter()
                .map(|each| NeverSentProjectBody {
                    project: ProjectRef::from(&each.project),
                    marked: each.marked,
                    version: each.version,
                })
                .collect(),
            truncated: list.truncated,
        }
    }
}

/// Marks a subject or takes the mark away, on the version that was looked at.
#[derive(Deserialize, ToSchema)]
pub(crate) struct SetNeverSentBody {
    /// `true` keeps the subject from AI, `false` takes the mark away.
    marked: bool,
    /// The version the person looked at; 0 for a subject that never had a mark.
    version: i64,
}

/// Which built-in fields and projects are kept from AI. Open to everybody who is signed in.
#[utoipa::path(
    get,
    path = "/api/v1/never-sent-to-ai",
    operation_id = "listNeverSentToAi",
    tag = "ai",
    responses((status = 200, description = "The marks.", body = NeverSentBody), NotSignedInAnswer)
)]
async fn list_never_sent(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let list = auth
        .api()?
        .never_sent_list(info, NeverSentQuery { session_token })
        .await?;
    answer_authorized(&list.map(|list| NeverSentBody::from(&list)))
}

async fn set(
    auth: AuthHandle,
    (info, session_token): (bb_app::auth::RequestInfo, String),
    target: NeverSentTarget,
    body: SetNeverSentBody,
) -> Result<Response, ApiError> {
    let list = auth
        .api()?
        .set_never_sent(
            info,
            SetNeverSent {
                session_token,
                target,
                marked: body.marked,
                version: body.version,
            },
        )
        .await?;
    answer_authorized(&list.map(|list| NeverSentBody::from(&list)))
}

/// Marks a built-in field *Never sent to AI*, or takes the mark away. Needs *Change organization
/// settings*.
#[utoipa::path(
    put,
    path = "/api/v1/never-sent-to-ai/fields/{field}",
    operation_id = "setFieldNeverSentToAi",
    tag = "ai",
    params(("field" = String, Path, description = "The key of the built-in field, like `description`.")),
    request_body = SetNeverSentBody,
    responses(
        (status = 200, description = "Every mark, after the change.", body = NeverSentBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn set_field_never_sent(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(field): ApiPath<String>,
    ApiJson(body): ApiJson<SetNeverSentBody>,
) -> Result<Response, ApiError> {
    let field =
        Field::from_code(&field).ok_or_else(|| ApiError::new(bb_core::ErrorCode::NotFound))?;
    set(
        auth,
        (info, session_token),
        NeverSentTarget::Field(field),
        body,
    )
    .await
}

/// Marks a project *Never sent to AI*, with every project inside it, or takes the mark away. Needs
/// *Change project settings* in the project.
#[utoipa::path(
    put,
    path = "/api/v1/never-sent-to-ai/projects/{project}",
    operation_id = "setProjectNeverSentToAi",
    tag = "ai",
    params(("project" = String, Path, description = "The ID or short name of the project.")),
    request_body = SetNeverSentBody,
    responses(
        (status = 200, description = "Every mark, after the change.", body = NeverSentBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn set_project_never_sent(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(project): ApiPath<String>,
    ApiJson(body): ApiJson<SetNeverSentBody>,
) -> Result<Response, ApiError> {
    set(
        auth,
        (info, session_token),
        NeverSentTarget::Project(project),
        body,
    )
    .await
}

/// The routes of *Never sent to AI*.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_never_sent))
        .routes(routes!(set_field_never_sent))
        .routes(routes!(set_project_never_sent))
}
