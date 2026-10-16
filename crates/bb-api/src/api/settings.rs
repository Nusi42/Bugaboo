//! The settings routes (CFG-04, CFG-06, ADR-0018): one set of routes serves every setting of the
//! registry, on the organization (`/settings`), on a project (`/projects/{id}/settings`) and for
//! the person who asks (`/me/settings`). A route lists the settings of a topic with what the
//! registry knows about them (name, explanation, type, choices, limits, levels, whether they can
//! be locked) and the value that counts with where it comes from; another changes a value, takes
//! it away (*Reset to default*), or locks and unlocks the organization's value.
//!
//! Which permission a route needs is declared once in `crate::permissions`: the organization's
//! routes *Change organization settings*, the project's *Change project settings*, and the
//! person's own routes only a sign-in (what a person may change in their own settings is decided
//! by the registry in the use case). The answers about a person's own settings are never stored
//! by a browser or a cache: a shared computer keeps nothing of them after the person has left.

use std::collections::BTreeMap;

use axum::extract::State;
use axum::response::Response;
use bb_app::access::ProjectLabel;
use bb_app::settings::{
    ChangeSetting, ImageHostsQuery, LockSetting, ResetSetting, SettingList, SettingSource,
    SettingView, SettingsQuery, SettingsScope,
};
use bb_core::settings::{DefaultValue, Format, Level, SettingValue};
use bb_policy::{Authorized, Masked};
use serde::{Deserialize, Serialize};
use utoipa::openapi::response::{Response as ResponseDescription, ResponseBuilder};
use utoipa::openapi::{ContentBuilder, Ref, RefOr};
use utoipa::{IntoResponses, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer, NotSignedInAnswer, Session,
    answer_authorized, problem_answers,
};
use bb_core::error_code::PROBLEM_CONTENT_TYPE;

use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};

mod bodies;
mod common;
mod image_hosts;
mod organization;
mod person;
mod project;

pub(crate) use bodies::*;
use common::{change, list, lock, reset};
use image_hosts::{__path_get_image_hosts, get_image_hosts};
use organization::{
    __path_change_organization_setting, __path_list_organization_settings,
    __path_lock_organization_setting, __path_reset_organization_setting,
    __path_unlock_organization_setting, change_organization_setting, list_organization_settings,
    lock_organization_setting, reset_organization_setting, unlock_organization_setting,
};
use person::{
    __path_change_my_setting, __path_list_my_settings, __path_reset_my_setting, change_my_setting,
    list_my_settings, reset_my_setting,
};
use project::{
    __path_change_project_setting, __path_list_project_settings, __path_reset_project_setting,
    change_project_setting, list_project_settings, reset_project_setting,
};

/// The routes of the settings.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_organization_settings))
        .routes(routes!(
            change_organization_setting,
            reset_organization_setting
        ))
        .routes(routes!(
            lock_organization_setting,
            unlock_organization_setting
        ))
        .routes(routes!(list_project_settings))
        .routes(routes!(change_project_setting, reset_project_setting))
        .routes(routes!(get_image_hosts))
        .routes(routes!(list_my_settings))
        .routes(routes!(change_my_setting, reset_my_setting))
}
