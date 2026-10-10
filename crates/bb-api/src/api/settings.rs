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

problem_answers! {
    /// A lower level cannot change what the organization locked.
    LockedAnswer => ("409", "The organization locked this setting (setting.locked)."),
}

/// What a setting holds: yes or no, a whole number, or text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub(crate) enum ValueBody {
    /// Yes or no.
    Bool(bool),
    /// A whole number.
    Integer(i64),
    /// Text, or one choice from a list written as text.
    Text(String),
}

impl From<SettingValue> for ValueBody {
    fn from(value: SettingValue) -> Self {
        match value {
            SettingValue::Bool(value) => Self::Bool(value),
            SettingValue::Integer(value) => Self::Integer(value),
            SettingValue::Text(value) => Self::Text(value),
        }
    }
}

impl From<ValueBody> for SettingValue {
    fn from(value: ValueBody) -> Self {
        match value {
            ValueBody::Bool(value) => Self::Bool(value),
            ValueBody::Integer(value) => Self::Integer(value),
            ValueBody::Text(value) => Self::Text(value),
        }
    }
}

/// The type of a setting.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
enum KindBody {
    /// Yes or no.
    Bool,
    /// A whole number.
    Integer,
    /// Text.
    Text,
}

/// What kind of text a text setting holds.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
enum FormatBody {
    /// Any text.
    Free,
    /// One of a fixed list (`choices`).
    Choice,
    /// `device`, or a time zone like `Europe/Berlin`.
    TimeZone,
    /// The address of a page of the app.
    PageAddress,
    /// The keys of the keyboard shortcuts, as one JSON object.
    ShortcutKeys,
    /// A list of host names separated by commas.
    Hosts,
}

/// Where a setting can be set.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
enum LevelBody {
    /// The whole organization.
    Organization,
    /// One project and its sub-projects.
    Project,
    /// One person.
    Person,
}

impl From<Level> for LevelBody {
    fn from(level: Level) -> Self {
        match level {
            Level::Organization => Self::Organization,
            Level::Project => Self::Project,
            Level::Person => Self::Person,
        }
    }
}

/// One value of a setting that has a fixed list.
#[derive(Serialize, ToSchema)]
struct ChoiceBody {
    /// The text that is stored and sent.
    value: String,
    /// The key of the message that names the choice.
    label_key: String,
}

/// The smallest and the largest value of a whole-number setting.
#[derive(Serialize, ToSchema)]
struct BoundsBody {
    /// The smallest value.
    least: i64,
    /// The largest value.
    most: i64,
}

/// The size of a list a text setting holds (items separated by commas).
#[derive(Serialize, ToSchema)]
struct ListLimitsBody {
    /// The most items.
    most_items: u32,
    /// The most characters of one item.
    longest_item: u32,
}

/// Where the value that counts comes from.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
enum SourceKindBody {
    /// Nobody set it: the standard value.
    Default,
    /// The organization set it.
    Organization,
    /// A project set it.
    Project,
    /// The person set it themselves.
    Person,
}

/// A project a value comes from.
#[derive(Serialize, ToSchema)]
struct ProjectRefBody {
    /// The ID of the project.
    id: String,
    /// Its short name.
    short_name: String,
    /// Its name.
    name: String,
}

/// Where the value that counts comes from.
#[derive(Serialize, ToSchema)]
struct SourceBody {
    /// Which level.
    kind: SourceKindBody,
    /// For a value from a project: which one. Missing when the person cannot see the project.
    #[serde(skip_serializing_if = "Option::is_none")]
    project: Option<ProjectRefBody>,
}

impl From<&SettingSource> for SourceBody {
    fn from(source: &SettingSource) -> Self {
        let (kind, project) = match source {
            SettingSource::Default => (SourceKindBody::Default, None),
            SettingSource::Organization => (SourceKindBody::Organization, None),
            SettingSource::Person => (SourceKindBody::Person, None),
            SettingSource::Project(ProjectLabel::Visible {
                id,
                short_name,
                name,
            }) => (
                SourceKindBody::Project,
                Some(ProjectRefBody {
                    id: id.to_string(),
                    short_name: short_name.clone(),
                    name: name.clone(),
                }),
            ),
            SettingSource::Project(ProjectLabel::Hidden) => (SourceKindBody::Project, None),
        };
        Self { kind, project }
    }
}

/// One setting: what the registry knows about it, and the value that counts.
#[derive(Serialize, ToSchema)]
pub(crate) struct SettingBody {
    /// The key: the topic, a dot and the name, like `topic.name`.
    key: String,
    /// The key of the message that names the setting.
    name_key: String,
    /// The key of the message that explains it in one line.
    help_key: String,
    /// What it holds.
    kind: KindBody,
    /// For text: what kind of text.
    format: FormatBody,
    /// For a setting with a fixed list: the values it can have.
    choices: Vec<ChoiceBody>,
    /// For a whole number: the smallest and the largest value.
    #[serde(skip_serializing_if = "Option::is_none")]
    bounds: Option<BoundsBody>,
    /// For a list in a text: its size.
    #[serde(skip_serializing_if = "Option::is_none")]
    list: Option<ListLimitsBody>,
    /// Where it can be set.
    levels: Vec<LevelBody>,
    /// Whether administrators can lock it.
    lockable: bool,
    /// The standard value.
    default: ValueBody,
    /// The value that counts here.
    value: ValueBody,
    /// Where it comes from.
    source: SourceBody,
    /// Whether this level has a value of its own, which *Reset to default* takes away.
    own: bool,
    /// On the organization's level: whether the value is locked. Below it: whether the
    /// organization's lock is in effect, so the value cannot be changed.
    locked: bool,
}

impl Masked for SettingBody {}

impl From<&SettingView> for SettingBody {
    fn from(view: &SettingView) -> Self {
        let definition = view.definition;
        let format = definition.format();
        Self {
            key: definition.key().to_owned(),
            name_key: definition.name().to_string(),
            help_key: definition.help().to_string(),
            kind: match definition.default() {
                DefaultValue::Bool(_) => KindBody::Bool,
                DefaultValue::Integer(_) => KindBody::Integer,
                DefaultValue::Text(_) => KindBody::Text,
            },
            format: match format {
                Format::Free => FormatBody::Free,
                Format::Choices(_) => FormatBody::Choice,
                Format::TimeZone => FormatBody::TimeZone,
                Format::PageAddress => FormatBody::PageAddress,
                Format::ShortcutKeys => FormatBody::ShortcutKeys,
                Format::Hosts => FormatBody::Hosts,
            },
            choices: format
                .choices()
                .iter()
                .map(|choice| ChoiceBody {
                    value: choice.value().to_owned(),
                    label_key: choice.label().to_string(),
                })
                .collect(),
            bounds: definition
                .bounds()
                .map(|(least, most)| BoundsBody { least, most }),
            list: definition
                .list_limits()
                .map(|(most_items, longest_item)| ListLimitsBody {
                    most_items: u32::try_from(most_items).unwrap_or(u32::MAX),
                    longest_item: u32::try_from(longest_item).unwrap_or(u32::MAX),
                }),
            levels: definition
                .levels()
                .iter()
                .map(|level| LevelBody::from(*level))
                .collect(),
            lockable: definition.lockable(),
            default: definition.default().to_value().into(),
            value: view.value.clone().into(),
            source: SourceBody::from(&view.source),
            own: view.own,
            locked: view.locked,
        }
    }
}

/// The settings of a topic, in the order of the registry.
#[derive(Serialize, ToSchema)]
pub(crate) struct SettingsBody {
    /// The settings.
    settings: Vec<SettingBody>,
}

impl Masked for SettingsBody {}

fn list_body(list: Authorized<SettingList>) -> Result<Response, ApiError> {
    answer_authorized(&list.map(|list| SettingsBody {
        settings: list.settings.iter().map(SettingBody::from).collect(),
    }))
}

fn setting_body(view: Authorized<SettingView>) -> Result<Response, ApiError> {
    answer_authorized(&view.map(|view| SettingBody::from(&view)))
}

/// A new value for a setting.
#[derive(Deserialize, ToSchema)]
pub(crate) struct NewValueBody {
    /// The value. It must be of the type of the setting and one it accepts.
    value: ValueBody,
}

/// What a route of a list reads from its address.
mod query {
    use serde::Deserialize;
    use utoipa::IntoParams;

    /// Which settings to list.
    #[derive(Deserialize, IntoParams)]
    #[into_params(parameter_in = Query)]
    pub(super) struct TopicParams {
        /// Only the settings whose key starts with one of these texts, separated by commas:
        /// `security.` for a topic, or longer (`auth.access-key-`) for part of one, so
        /// `security.,auth.access-key-` is the settings of one page. Left out, every setting of
        /// the level.
        pub(super) topic: Option<String>,
    }

    /// Which of a person's own settings to list, and where they work.
    #[derive(Deserialize, IntoParams)]
    #[into_params(parameter_in = Query)]
    pub(super) struct MineParams {
        /// Only the settings whose key starts with one of these texts, separated by commas, like
        /// `region.` for a topic.
        pub(super) topic: Option<String>,
        /// The ID or the short name of the project the person works in: the values of the project (and of the
        /// projects above it) then count for them.
        pub(super) project: Option<String>,
    }
}

/// The path of a setting.
#[derive(Deserialize)]
pub(crate) struct KeyPath {
    key: String,
}

/// The path of a setting of a project.
#[derive(Deserialize)]
pub(crate) struct ProjectKeyPath {
    project: String,
    key: String,
}

/// The path of the settings of a project.
#[derive(Deserialize)]
pub(crate) struct ProjectPath {
    project: String,
}

async fn list(
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

async fn change(
    auth: &AuthHandle,
    info: bb_app::auth::RequestInfo,
    session_token: String,
    scope: SettingsScope,
    key: String,
    value: ValueBody,
) -> Result<Response, ApiError> {
    let view = auth
        .api()?
        .change_setting(
            info,
            ChangeSetting {
                session_token,
                scope,
                key,
                value: value.into(),
            },
        )
        .await?;
    setting_body(view)
}

async fn reset(
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

/// The settings of the whole organization. Needs the permission *Change organization settings*.
#[utoipa::path(
    get,
    path = "/api/v1/settings",
    operation_id = "listOrganizationSettings",
    tag = "settings",
    params(query::TopicParams),
    responses((status = 200, description = "The settings of the organization with their values.", body = SettingsBody), NotSignedInAnswer, ForbiddenAnswer)
)]
async fn list_organization_settings(
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
async fn change_organization_setting(
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
async fn reset_organization_setting(
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

async fn lock(
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
async fn lock_organization_setting(
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
async fn unlock_organization_setting(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(path): ApiPath<KeyPath>,
) -> Result<Response, ApiError> {
    lock(&auth, info, session_token, path.key, false).await
}

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
async fn list_project_settings(
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
async fn change_project_setting(
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
async fn reset_project_setting(
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

/// The settings of the person who asks, as they count for them: *My settings*.
#[utoipa::path(
    get,
    path = "/api/v1/me/settings",
    operation_id = "listMySettings",
    tag = "settings",
    params(query::MineParams),
    responses((status = 200, description = "The settings a person can choose for themselves, with the values that count for them.", body = SettingsBody), NotSignedInAnswer, NotFoundAnswer)
)]
async fn list_my_settings(
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
async fn change_my_setting(
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
        SettingsScope::Person,
        path.key,
        body.value,
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
async fn reset_my_setting(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(path): ApiPath<KeyPath>,
) -> Result<Response, ApiError> {
    reset(&auth, info, session_token, SettingsScope::Person, path.key).await
}

/// The hosts that images in text may come from, already split into one host for each item.
#[derive(Serialize, ToSchema)]
pub(crate) struct ImageHostsBody {
    /// The hosts, such as `images.example.com`. Empty means no image from outside.
    hosts: Vec<String>,
}

impl Masked for ImageHostsBody {}

/// The hosts that images in text may come from (the setting `content.image-hosts`), for the preview
/// of the editor. Every signed-in person may read them: they are in the content security policy
/// that every page carries. The list is split here once; the browser never splits it again.
#[utoipa::path(
    get,
    path = "/api/v1/content/image-hosts",
    operation_id = "getImageHosts",
    tag = "settings",
    responses((status = 200, description = "The hosts.", body = ImageHostsBody), NotSignedInAnswer)
)]
async fn get_image_hosts(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let hosts = auth
        .api()?
        .image_hosts(info, ImageHostsQuery { session_token })
        .await?;
    answer_authorized(&hosts.map(|found| ImageHostsBody { hosts: found.hosts }))
}

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
