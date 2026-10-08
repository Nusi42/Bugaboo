//! Access keys (AUTH-07): the routes of a person's own keys below `/api/v1/access-keys`. They
//! belong to the person's own account, so they declare `own-account` and are open to a session
//! only: a key can never make, list or delete keys, which keeps a stolen key from outliving its
//! owner's decision (`bb_app::auth`, `Ctx::active`).
//!
//! Making a key asks for the password again and answers the secret once: that answer is a
//! secret in the route table (`crate::permissions`), so it is never stored and never compressed
//! (`crate::etag`, WP-02).

use axum::extract::State;
use axum::response::Response;
use bb_app::access::ProjectLabel;
use bb_app::auth::{
    AccessKeyRulesQuery, AccessKeyView, AccessKeysQuery, AppError, DeleteAccessKey, MakeAccessKey,
    ProjectChoice, ProjectChoicesQuery, RequireSessionQuery,
};
use bb_core::ErrorCode;
use bb_core::access_key::KeyChoice;
use bb_core::id::{AccessKeyId, ProjectId};
use bb_policy::{Masked, Permission, Topic};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer, NotSignedInAnswer, Session,
    answer_authorized, done,
};
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};
use crate::lean::{FieldSpec, ListParams, Listed, Page};

/// The fields of a key that can be asked for, and which of them a list holds when the request
/// names none: what the list under *My settings* shows.
const KEY_FIELDS: [FieldSpec; 10] = [
    FieldSpec::by_default("id"),
    FieldSpec::by_default("name"),
    FieldSpec::by_default("prefix"),
    FieldSpec::by_default("choice"),
    FieldSpec::by_default("expires_at"),
    FieldSpec::on_request("permissions"),
    FieldSpec::on_request("projects"),
    FieldSpec::on_request("created_at"),
    FieldSpec::on_request("last_used_at"),
    FieldSpec::on_request("expired"),
];
/// The name of the list, so its cursors are its own.
const KEYS_LIST: &str = "access-keys";

/// A project a key is limited to, as its owner may see it: a reference by ID, and the names only
/// of a project the owner may know about (*a project you can't see* otherwise).
#[derive(Serialize, ToSchema)]
pub(crate) struct KeyProjectRef {
    /// `visible` or `hidden`.
    shown: ProjectShown,
    /// The ID of the project. Only for a visible one.
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    /// Its short name. Only for a visible one.
    #[serde(skip_serializing_if = "Option::is_none")]
    short_name: Option<String>,
    /// Its name. Only for a visible one.
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

/// Whether the owner may know the project.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
enum ProjectShown {
    Visible,
    Hidden,
}

impl From<&ProjectLabel> for KeyProjectRef {
    fn from(label: &ProjectLabel) -> Self {
        match label {
            ProjectLabel::Visible {
                id,
                short_name,
                name,
            } => Self {
                shown: ProjectShown::Visible,
                id: Some(id.to_string()),
                short_name: Some(short_name.clone()),
                name: Some(name.clone()),
            },
            ProjectLabel::Hidden => Self {
                shown: ProjectShown::Hidden,
                id: None,
                short_name: None,
                name: None,
            },
        }
    }
}

/// An access key in a list: the fields that can be asked for. An answer holds only those that
/// were asked for.
#[derive(Serialize, ToSchema)]
pub(crate) struct AccessKeyItem {
    /// The ID of the key.
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    /// What the person called it.
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    /// The public start of the key, after `bb_`.
    #[serde(skip_serializing_if = "Option::is_none")]
    prefix: Option<String>,
    /// What it may do.
    #[serde(skip_serializing_if = "Option::is_none")]
    choice: Option<KeyChoice>,
    /// The single permissions, when the choice is `permissions`.
    #[serde(skip_serializing_if = "Option::is_none")]
    permissions: Option<Vec<Permission>>,
    /// The projects the key is limited to; empty for every project.
    #[serde(skip_serializing_if = "Option::is_none")]
    projects: Option<Vec<KeyProjectRef>>,
    /// When it was made.
    #[serde(skip_serializing_if = "Option::is_none")]
    created_at: Option<String>,
    /// When it stops working.
    #[serde(skip_serializing_if = "Option::is_none")]
    expires_at: Option<String>,
    /// The last time it was used, to the minute. Left out when it was never used.
    #[serde(skip_serializing_if = "Option::is_none")]
    last_used_at: Option<String>,
    /// Whether it has stopped working.
    #[serde(skip_serializing_if = "Option::is_none")]
    expired: Option<bool>,
}

impl Masked for AccessKeyItem {}

impl From<&AccessKeyView> for AccessKeyItem {
    fn from(key: &AccessKeyView) -> Self {
        Self {
            id: Some(key.id.to_string()),
            name: Some(key.name.clone()),
            prefix: Some(key.prefix.clone()),
            choice: Some(key.choice),
            permissions: Some(key.permissions.clone()),
            projects: Some(key.projects.iter().map(KeyProjectRef::from).collect()),
            created_at: Some(key.created_at.to_string()),
            expires_at: Some(key.expires_at.to_string()),
            last_used_at: key.last_used_at.map(|moment| moment.to_string()),
            expired: Some(key.expired),
        }
    }
}

/// An access key as its owner sees it after making it.
#[derive(Serialize, ToSchema)]
pub(crate) struct AccessKeyBody {
    /// The ID of the key.
    id: String,
    /// What the person called it.
    name: String,
    /// The public start of the key, after `bb_`.
    prefix: String,
    /// What it may do.
    choice: KeyChoice,
    /// The single permissions, when the choice is `permissions`.
    permissions: Vec<Permission>,
    /// The projects the key is limited to; empty for every project.
    projects: Vec<KeyProjectRef>,
    /// When it was made.
    created_at: String,
    /// When it stops working.
    expires_at: String,
}

impl From<&AccessKeyView> for AccessKeyBody {
    fn from(key: &AccessKeyView) -> Self {
        Self {
            id: key.id.to_string(),
            name: key.name.clone(),
            prefix: key.prefix.clone(),
            choice: key.choice,
            permissions: key.permissions.clone(),
            projects: key.projects.iter().map(KeyProjectRef::from).collect(),
            created_at: key.created_at.to_string(),
            expires_at: key.expires_at.to_string(),
        }
    }
}

/// A new access key: the secret is shown only now.
#[derive(Serialize, ToSchema)]
pub(crate) struct CreatedAccessKeyBody {
    /// The key as it will be listed.
    key: AccessKeyBody,
    /// The whole key, `bb_`, the prefix, `_` and the secret. Send it as
    /// `Authorization: Bearer <key>`. It is not stored and cannot be shown again.
    secret: String,
}

impl Masked for CreatedAccessKeyBody {}

/// One permission a key can be limited to, with the topic it is listed under. The sentence is the
/// message `permission.<key>` and the name of the topic `permission-topic.<topic>`.
#[derive(Serialize, ToSchema)]
pub(crate) struct PermissionChoiceBody {
    /// The key of the permission.
    key: Permission,
    /// What it is about.
    topic: Topic,
}

/// How long a new access key lives in this organization, and what a key can be limited to.
#[derive(Serialize, ToSchema)]
pub(crate) struct AccessKeyRulesBody {
    /// The days a new key lives when the person picks no other date.
    default_days: i64,
    /// The most days a key can live.
    longest_days: i64,
    /// The single permissions a key can be limited to under *More options*, in the order of the
    /// catalogue.
    permissions: Vec<PermissionChoiceBody>,
}

impl Masked for AccessKeyRulesBody {}

/// An access key to make.
#[derive(Deserialize, ToSchema)]
pub(crate) struct MakeKeyBody {
    /// The password of the person, asked again so a stolen session cannot make a key.
    password: String,
    /// What the key is called.
    name: String,
    /// What it may do.
    choice: KeyChoice,
    /// The single permissions; only with the choice `permissions`, and then at least one.
    #[serde(default)]
    permissions: Vec<Permission>,
    /// The projects the key is limited to; leave empty for every project. The key counts in
    /// them and in their sub-projects, and gives nothing for the whole organization.
    #[serde(default)]
    projects: Vec<String>,
    /// When it stops working (a date and time such as `2027-03-01T00:00:00Z`). Left out: the
    /// default number of days from now. It cannot be later than the longest time.
    #[serde(default)]
    expires_at: Option<String>,
}

/// The access keys of the person, a page at a time.
#[utoipa::path(
    get,
    path = "/api/v1/access-keys",
    operation_id = "listAccessKeys",
    tag = "access-keys",
    params(ListParams),
    responses((status = 200, description = "The keys of the person, newest first.", body = Page<AccessKeyItem>), NotSignedInAnswer, ForbiddenAnswer)
)]
async fn list_access_keys(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    // Who asks is judged first, then the cursor, then the fields (the same order as every list).
    let position = params.position(KEYS_LIST);
    let cursor = match &position {
        Ok(position) => position.clone(),
        Err(_) => Some(String::new()),
    };
    let page = auth
        .api()?
        .access_keys(
            info,
            AccessKeysQuery {
                session_token,
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(|error| match error {
            // The only thing the use case can find invalid is the cursor.
            AppError::Refused(ErrorCode::InvalidRequest) => ApiError::new(ErrorCode::BadCursor),
            other => ApiError::from(other),
        })?;
    position?;
    let list = page.map(|page| Listed {
        items: page.items.iter().map(AccessKeyItem::from).collect(),
        next: page.next_cursor,
    });
    let fields = params.fields_readable(&KEY_FIELDS, &list.hidden_properties())?;
    Ok(super::auth::answer(Page::of_authorized(
        KEYS_LIST, &list, &fields,
    )?))
}

/// Makes an access key. The key is shown once. The password is asked again.
#[utoipa::path(
    post,
    path = "/api/v1/access-keys",
    operation_id = "makeAccessKey",
    tag = "access-keys",
    request_body = MakeKeyBody,
    responses(
        (status = 200, description = "The key, with its secret.", body = CreatedAccessKeyBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn make_access_key(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    body: Result<ApiJson<MakeKeyBody>, ApiError>,
) -> Result<Response, ApiError> {
    // Who asks comes before what is asked: a key is turned away before its body is looked at.
    auth.api()?
        .require_session(
            info.clone(),
            RequireSessionQuery {
                session_token: session_token.clone(),
            },
        )
        .await?;
    let ApiJson(body) = body?;
    let invalid = || ApiError::new(ErrorCode::InvalidRequest);
    let projects = body
        .projects
        .iter()
        .map(|project| project.parse::<ProjectId>().map_err(|_| invalid()))
        .collect::<Result<Vec<_>, _>>()?;
    let created = auth
        .api()?
        .make_access_key(
            info,
            MakeAccessKey {
                session_token,
                password: body.password,
                name: body.name,
                choice: body.choice,
                permissions: body.permissions,
                projects,
                expires_at: body.expires_at,
            },
        )
        .await?;
    answer_authorized(&created.map(|created| CreatedAccessKeyBody {
        key: AccessKeyBody::from(&created.key),
        secret: created.secret,
    }))
}

/// Deletes an access key at once. A deleted key cannot come back.
#[utoipa::path(
    delete,
    path = "/api/v1/access-keys/{access_key_id}",
    operation_id = "deleteAccessKey",
    tag = "access-keys",
    params(("access_key_id" = String, Path, description = "The ID of the key.")),
    responses((status = 204, description = "Deleted."), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn delete_access_key(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(access_key_id): ApiPath<String>,
) -> Result<Response, ApiError> {
    // Who asks comes before what is asked: a key is turned away before the address is looked at.
    auth.api()?
        .require_session(
            info.clone(),
            RequireSessionQuery {
                session_token: session_token.clone(),
            },
        )
        .await?;
    let id: AccessKeyId = access_key_id
        .parse()
        .map_err(|_| ApiError::new(ErrorCode::NotFound))?;
    auth.api()?
        .delete_access_key(info, DeleteAccessKey { session_token, id })
        .await?;
    Ok(done())
}

/// How long a new access key lives in this organization.
#[utoipa::path(
    get,
    path = "/api/v1/access-keys/rules",
    operation_id = "getAccessKeyRules",
    tag = "access-keys",
    responses((status = 200, description = "The default and the longest time a key lives, in days.", body = AccessKeyRulesBody), NotSignedInAnswer, ForbiddenAnswer)
)]
async fn get_access_key_rules(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let rules = auth
        .api()?
        .access_key_rules(info, AccessKeyRulesQuery { session_token })
        .await?;
    answer_authorized(&rules.map(|rules| {
        AccessKeyRulesBody {
            default_days: rules.default_days,
            longest_days: rules.longest_days,
            permissions: rules
                .permissions
                .into_iter()
                .map(|(key, topic)| PermissionChoiceBody { key, topic })
                .collect(),
        }
    }))
}

/// The fields of a project that can be asked for when choosing projects for a key.
const PROJECT_FIELDS: [FieldSpec; 3] = [
    FieldSpec::by_default("id"),
    FieldSpec::by_default("short_name"),
    FieldSpec::by_default("name"),
];
/// The name of the list, so its cursors are its own.
const PROJECTS_LIST: &str = "access-key-projects";

/// A project a person may limit a key to.
#[derive(Serialize, ToSchema)]
pub(crate) struct ProjectChoiceItem {
    /// The ID of the project.
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    /// Its short name.
    #[serde(skip_serializing_if = "Option::is_none")]
    short_name: Option<String>,
    /// Its name.
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

impl Masked for ProjectChoiceItem {}

impl From<&ProjectChoice> for ProjectChoiceItem {
    fn from(project: &ProjectChoice) -> Self {
        Self {
            id: Some(project.id.to_string()),
            short_name: Some(project.short_name.clone()),
            name: Some(project.name.clone()),
        }
    }
}

/// The projects the person may limit a key to: those they know about, by short name, a page at a
/// time.
#[utoipa::path(
    get,
    path = "/api/v1/access-keys/projects",
    operation_id = "listProjectChoices",
    tag = "access-keys",
    params(ListParams),
    responses((status = 200, description = "The projects the person may know about, by short name.", body = Page<ProjectChoiceItem>), NotSignedInAnswer, ForbiddenAnswer)
)]
async fn list_project_choices(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    let position = params.position(PROJECTS_LIST);
    let cursor = match &position {
        Ok(position) => position.clone(),
        Err(_) => Some(String::new()),
    };
    let page = auth
        .api()?
        .project_choices(
            info,
            ProjectChoicesQuery {
                session_token,
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(|error| match error {
            // The only thing the use case can find invalid is the cursor.
            AppError::Refused(ErrorCode::InvalidRequest) => ApiError::new(ErrorCode::BadCursor),
            other => ApiError::from(other),
        })?;
    position?;
    let list = page.map(|page| Listed {
        items: page.items.iter().map(ProjectChoiceItem::from).collect(),
        next: page.next_cursor,
    });
    let fields = params.fields_readable(&PROJECT_FIELDS, &list.hidden_properties())?;
    Ok(super::auth::answer(Page::of_authorized(
        PROJECTS_LIST,
        &list,
        &fields,
    )?))
}

/// The routes of a person's own access keys.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_access_keys, make_access_key))
        .routes(routes!(get_access_key_rules))
        .routes(routes!(list_project_choices))
        .routes(routes!(delete_access_key))
}

#[cfg(test)]
mod tests {
    use bb_core::id::AccessKeyId;

    use super::*;
    use crate::lean::assert_fields_are_the_keys_of;

    /// The moment every moment starts from, whatever type the view gives it.
    fn epoch<T: Default>() -> T {
        T::default()
    }

    #[test]
    fn perf_02_the_field_table_names_exactly_the_fields_of_an_access_key() {
        let item = AccessKeyItem::from(&AccessKeyView {
            id: AccessKeyId::from_random([1; 16]),
            name: "Nightly report".to_owned(),
            prefix: "k7q2x9mf".to_owned(),
            choice: KeyChoice::SeeOnly,
            permissions: Vec::new(),
            projects: Vec::new(),
            created_at: epoch(),
            expires_at: epoch(),
            last_used_at: Some(epoch()),
            expired: true,
        });
        assert_fields_are_the_keys_of(&KEY_FIELDS, &item);
    }

    #[test]
    fn perf_02_the_field_table_names_exactly_the_fields_of_a_project_to_choose() {
        let item = ProjectChoiceItem::from(&ProjectChoice {
            id: "0190f4c2-7a3e-7c1d-9b52-3d6e8f1a2b4c".parse().unwrap(),
            short_name: "WEB".to_owned(),
            name: "Web".to_owned(),
        });
        assert_fields_are_the_keys_of(&PROJECT_FIELDS, &item);
    }
}
