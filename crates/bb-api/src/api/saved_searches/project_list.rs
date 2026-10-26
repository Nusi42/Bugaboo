//! The routes of the list a project shows, below `/api/v1/projects/{project}/list`.

use axum::extract::State;
use axum::response::Response;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::super::auth::{
    AuthHandle, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer, NotSignedInAnswer, Session,
    answer_authorized,
};
use super::super::body_id;
use super::searches::session_first;
use super::{
    OptionalSavedSearchBody, ProjectListChoiceBody, ProjectListQuery, SavedSearchId, SetProjectList,
};
use crate::errors::{ApiError, ApiJson, ApiPath};

/// The list a project shows: the saved search the project chose, when the person may open it, else
/// `null` and the client shows the built-in default list. Needs the right to see the project.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{project}/list",
    operation_id = "getProjectList",
    tag = "saved-searches",
    params(("project" = String, Path, description = "The ID or the short name of the project.")),
    responses(
        (status = 200, description = "The list.", body = OptionalSavedSearchBody),
        NotSignedInAnswer,
        NotFoundAnswer
    )
)]
async fn get_project_list(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(project): ApiPath<String>,
) -> Result<Response, ApiError> {
    session_first(&auth, &info, &session_token).await?;
    let list = auth
        .api()?
        .project_list(
            info,
            ProjectListQuery {
                session_token,
                project,
            },
        )
        .await?;
    answer_authorized(&list.map(|list| OptionalSavedSearchBody::from(&list)))
}

/// Chooses the saved search a project shows as its list, or none. Needs *Change project settings*;
/// the search has to be shared with the project.
#[utoipa::path(
    put,
    path = "/api/v1/projects/{project}/list",
    operation_id = "setProjectList",
    tag = "saved-searches",
    params(("project" = String, Path, description = "The ID or the short name of the project.")),
    request_body = ProjectListChoiceBody,
    responses(
        (status = 200, description = "The list as it is now.", body = OptionalSavedSearchBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn set_project_list(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(project): ApiPath<String>,
    body: Result<ApiJson<ProjectListChoiceBody>, ApiError>,
) -> Result<Response, ApiError> {
    session_first(&auth, &info, &session_token).await?;
    let ApiJson(body) = body?;
    let search = body
        .search
        .as_deref()
        .map(body_id::<SavedSearchId>)
        .transpose()?;
    let list = auth
        .api()?
        .set_project_list(
            info,
            SetProjectList {
                session_token,
                project,
                search,
            },
        )
        .await?;
    answer_authorized(&list.map(|list| OptionalSavedSearchBody::from(&list)))
}

/// The routes of the list of a project.
pub(super) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new().routes(routes!(get_project_list, set_project_list))
}
