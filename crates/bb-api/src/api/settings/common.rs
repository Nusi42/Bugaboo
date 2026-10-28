//! What listing, changing, resetting and locking a setting share.

use super::{
    ApiError, AuthHandle, ChangeSetting, LockSetting, ResetSetting, Response, SettingsQuery,
    SettingsScope, ValueBody, list_body, setting_body,
};

pub(super) async fn list(
    auth: &AuthHandle,
    info: bb_app::auth::RequestInfo,
    session_token: String,
    scope: SettingsScope,
    topic: Option<String>,
    project: Option<String>,
) -> Result<Response, ApiError> {
    let list = auth
        .api()?
        .settings(
            info,
            SettingsQuery {
                session_token,
                scope,
                topic,
                project,
            },
        )
        .await?;
    list_body(list)
}

pub(super) async fn change(
    auth: &AuthHandle,
    info: bb_app::auth::RequestInfo,
    session_token: String,
    scope: SettingsScope,
    key: String,
    value: ValueBody,
    version: Option<i64>,
) -> Result<Response, ApiError> {
    let view = auth
        .api()?
        .change_setting(
            info,
            ChangeSetting {
                version,
                session_token,
                scope,
                key,
                value: value.into(),
            },
        )
        .await?;
    setting_body(view)
}

pub(super) async fn reset(
    auth: &AuthHandle,
    info: bb_app::auth::RequestInfo,
    session_token: String,
    scope: SettingsScope,
    key: String,
) -> Result<Response, ApiError> {
    let view = auth
        .api()?
        .reset_setting(
            info,
            ResetSetting {
                session_token,
                scope,
                key,
            },
        )
        .await?;
    setting_body(view)
}

pub(super) async fn lock(
    auth: &AuthHandle,
    info: bb_app::auth::RequestInfo,
    session_token: String,
    key: String,
    locked: bool,
) -> Result<Response, ApiError> {
    let view = auth
        .api()?
        .lock_setting(
            info,
            LockSetting {
                session_token,
                key,
                locked,
            },
        )
        .await?;
    setting_body(view)
}
