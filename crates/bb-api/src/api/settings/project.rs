//! The settings of a project.

use super::{
    ApiError, ApiJson, ApiPath, ApiQuery, AuthHandle, ForbiddenAnswer, Info, InvalidAnswer,
    LockedAnswer, NewValueBody, NotFoundAnswer, NotSignedInAnswer, ProjectKeyPath, ProjectPath,
    Response, Session, SettingBody, SettingsBody, SettingsScope, State, change, list, query, reset,
};

/// The settings of one project, as they count there (a sub-project takes what the project above
/// sets). Needs the permission *Change project settings* in the project.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{project}/settings",
    operation_id = "listProjectSettings",
    tag = "settings",
    params(("project" = String, Path, description = "The ID or the short name of the project."), query::TopicParams),
    responses((status = 200, description = "The settings that can be set on a project, with the values that count there.", body = SettingsBody), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
pub(super) async fn list_project_settings(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(path): ApiPath<ProjectPath>,
    ApiQuery(params): ApiQuery<query::TopicParams>,
) -> Result<Response, ApiError> {
    list(
        &auth,
        info,
        session_token,
        SettingsScope::ProjectAt(path.project),
        params.topic,
        None,
    )
    .await
}

/// Changes the value of a setting on a project. Needs the permission *Change project settings*
/// in the project.
#[utoipa::path(
    put,
    path = "/api/v1/projects/{project}/settings/{key}",
    operation_id = "changeProjectSetting",
    tag = "settings",
    params(
        ("project" = String, Path, description = "The ID or the short name of the project."),
        ("key" = String, Path, description = "The key of the setting.")
    ),
    request_body = NewValueBody,
    responses(
        (status = 200, description = "The setting with its new value.", body = SettingBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        LockedAnswer
    )
)]
pub(super) async fn change_project_setting(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(path): ApiPath<ProjectKeyPath>,
    ApiJson(body): ApiJson<NewValueBody>,
) -> Result<Response, ApiError> {
    change(
        &auth,
        info,
        session_token,
        SettingsScope::ProjectAt(path.project),
        path.key,
        body.value,
    )
    .await
}

/// *Reset to default* for a project: takes its value away, so the project above (or the
/// organization) counts again. Needs the permission *Change project settings* in the project.
#[utoipa::path(
    delete,
    path = "/api/v1/projects/{project}/settings/{key}",
    operation_id = "resetProjectSetting",
    tag = "settings",
    params(
        ("project" = String, Path, description = "The ID or the short name of the project."),
        ("key" = String, Path, description = "The key of the setting.")
    ),
    responses(
        (status = 200, description = "The setting as it is now.", body = SettingBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
pub(super) async fn reset_project_setting(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(path): ApiPath<ProjectKeyPath>,
) -> Result<Response, ApiError> {
    reset(
        &auth,
        info,
        session_token,
        SettingsScope::ProjectAt(path.project),
        path.key,
    )
    .await
}
