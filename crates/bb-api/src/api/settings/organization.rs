//! The settings of the whole organization, with their locks.

use super::{
    ApiError, ApiJson, ApiPath, ApiQuery, AuthHandle, ForbiddenAnswer, Info, InvalidAnswer,
    KeyPath, NewValueBody, NotFoundAnswer, NotSignedInAnswer, Response, Session, SettingBody,
    SettingsBody, SettingsScope, State, change, list, lock, query, reset,
};

/// The settings of the whole organization. Needs the permission *Change organization settings*.
#[utoipa::path(
    get,
    path = "/api/v1/settings",
    operation_id = "listOrganizationSettings",
    tag = "settings",
    params(query::TopicParams),
    responses((status = 200, description = "The settings of the organization with their values.", body = SettingsBody), NotSignedInAnswer, ForbiddenAnswer)
)]
pub(super) async fn list_organization_settings(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<query::TopicParams>,
) -> Result<Response, ApiError> {
    list(
        &auth,
        info,
        session_token,
        SettingsScope::Organization,
        params.topic,
        None,
    )
    .await
}

/// Changes the organization's value of a setting. Needs the permission *Change organization
/// settings*.
#[utoipa::path(
    put,
    path = "/api/v1/settings/{key}",
    operation_id = "changeOrganizationSetting",
    tag = "settings",
    params(("key" = String, Path, description = "The key of the setting.")),
    request_body = NewValueBody,
    responses(
        (status = 200, description = "The setting with its new value.", body = SettingBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
pub(super) async fn change_organization_setting(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(path): ApiPath<KeyPath>,
    ApiJson(body): ApiJson<NewValueBody>,
) -> Result<Response, ApiError> {
    change(
        &auth,
        info,
        session_token,
        SettingsScope::Organization,
        path.key,
        body.value,
        None,
    )
    .await
}

/// *Reset to default* for the organization: takes its value away, so the standard value counts
/// again. Needs the permission *Change organization settings*.
#[utoipa::path(
    delete,
    path = "/api/v1/settings/{key}",
    operation_id = "resetOrganizationSetting",
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
pub(super) async fn reset_organization_setting(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(path): ApiPath<KeyPath>,
) -> Result<Response, ApiError> {
    reset(
        &auth,
        info,
        session_token,
        SettingsScope::Organization,
        path.key,
    )
    .await
}

/// Locks the organization's value of a setting, so nobody lower down can choose another. Needs
/// the permission *Change organization settings*.
#[utoipa::path(
    put,
    path = "/api/v1/settings/{key}/lock",
    operation_id = "lockOrganizationSetting",
    tag = "settings",
    params(("key" = String, Path, description = "The key of the setting.")),
    responses(
        (status = 200, description = "The setting, locked.", body = SettingBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
pub(super) async fn lock_organization_setting(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(path): ApiPath<KeyPath>,
) -> Result<Response, ApiError> {
    lock(&auth, info, session_token, path.key, true).await
}

/// Unlocks the organization's value of a setting. Needs the permission *Change organization
/// settings*.
#[utoipa::path(
    delete,
    path = "/api/v1/settings/{key}/lock",
    operation_id = "unlockOrganizationSetting",
    tag = "settings",
    params(("key" = String, Path, description = "The key of the setting.")),
    responses(
        (status = 200, description = "The setting, unlocked.", body = SettingBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
pub(super) async fn unlock_organization_setting(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(path): ApiPath<KeyPath>,
) -> Result<Response, ApiError> {
    lock(&auth, info, session_token, path.key, false).await
}
