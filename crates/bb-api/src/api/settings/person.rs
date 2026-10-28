//! The settings of the person who asks.

use super::{
    ApiError, ApiJson, ApiPath, ApiQuery, AuthHandle, ForbiddenAnswer, Info, InvalidAnswer,
    KeyPath, LockedOrStaleAnswer, NewMyValueBody, NotFoundAnswer, NotSignedInAnswer, Response,
    Session, SettingBody, SettingsBody, SettingsScope, State, change, list, query, reset,
};

/// The settings of the person who asks, as they count for them: *My settings*.
#[utoipa::path(
    get,
    path = "/api/v1/me/settings",
    operation_id = "listMySettings",
    tag = "settings",
    params(query::MineParams),
    responses((status = 200, description = "The settings a person can choose for themselves, with the values that count for them.", body = SettingsBody), NotSignedInAnswer, NotFoundAnswer)
)]
pub(super) async fn list_my_settings(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<query::MineParams>,
) -> Result<Response, ApiError> {
    list(
        &auth,
        info,
        session_token,
        SettingsScope::Person,
        params.topic,
        params.project,
    )
    .await
}

/// Changes a value of the person's own settings.
#[utoipa::path(
    put,
    path = "/api/v1/me/settings/{key}",
    operation_id = "changeMySetting",
    tag = "settings",
    params(("key" = String, Path, description = "The key of the setting.")),
    request_body = NewMyValueBody,
    responses(
        (status = 200, description = "The setting with its new value.", body = SettingBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        LockedOrStaleAnswer
    )
)]
pub(super) async fn change_my_setting(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(path): ApiPath<KeyPath>,
    ApiJson(body): ApiJson<NewMyValueBody>,
) -> Result<Response, ApiError> {
    change(
        &auth,
        info,
        session_token,
        SettingsScope::Person,
        path.key,
        body.value,
        body.version,
    )
    .await
}

/// *Reset to default* for the person: takes their own value away, so the value from above counts
/// again.
#[utoipa::path(
    delete,
    path = "/api/v1/me/settings/{key}",
    operation_id = "resetMySetting",
    tag = "settings",
    params(("key" = String, Path, description = "The key of the setting.")),
    responses(
        (status = 200, description = "The setting as it is now.", body = SettingBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
pub(super) async fn reset_my_setting(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(path): ApiPath<KeyPath>,
) -> Result<Response, ApiError> {
    reset(&auth, info, session_token, SettingsScope::Person, path.key).await
}
