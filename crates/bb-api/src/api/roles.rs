//! The role routes below `/api/v1/roles` (SEC-08, AUTH-03): the list and the page of a role,
//! making a role (from nothing or from a copy), changing it, giving a starting role its starting
//! permissions back, and deleting it; and the permission catalogue the role page is made from.
//!
//! All of them need the permission *Make, change and delete roles*. What the use cases refuse
//! beyond that (nobody adds a permission they do not hold, the last administrator, a role in
//! use deleted without a replacement) comes back as the refusal of the use case.

use axum::extract::State;
use axum::response::Response;
use bb_app::roles::{
    ChangeRole, CopyRole, DeleteRole, MakeRole, PermissionCatalogue, PermissionsQuery, ResetRole,
    RoleDetail, RoleQuery, RoleView, RolesQuery,
};
use bb_core::id::RoleId;
use bb_policy::{Counts, Masked, Permission, Topic};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::access_bodies::cursor_error;
use super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer, answer_authorized, done,
};
use super::people::parse;
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};
use crate::lean::{FieldSpec, ListParams, Listed, Page};

/// The fields of a role in a list that can be asked for, and which of them a list holds when the
/// request names none: what the list of roles needs.
const ROLE_FIELDS: [FieldSpec; 6] = [
    FieldSpec::by_default("id"),
    FieldSpec::by_default("name"),
    FieldSpec::by_default("starting"),
    FieldSpec::by_default("two_step"),
    FieldSpec::on_request("position"),
    FieldSpec::on_request("permissions"),
];
/// The name of the list, so its cursors are its own.
const ROLES_LIST: &str = "roles";

/// A role in a list: the fields that can be asked for.
#[derive(Serialize, ToSchema)]
pub(crate) struct RoleItem {
    /// The ID of the role.
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    /// What it is called.
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    /// The starting role it is a copy of (`administrator`, …), `null` for a role of its own.
    #[serde(skip_serializing_if = "Option::is_none")]
    starting: Option<Option<String>>,
    /// Whether everybody with it needs two-step sign-in.
    #[serde(skip_serializing_if = "Option::is_none")]
    two_step: Option<bool>,
    /// The place in the order of roles.
    #[serde(skip_serializing_if = "Option::is_none")]
    position: Option<i64>,
    /// What it allows, in the order of the catalogue.
    #[serde(skip_serializing_if = "Option::is_none")]
    permissions: Option<Vec<Permission>>,
}

impl Masked for RoleItem {}

impl From<&RoleView> for RoleItem {
    fn from(role: &RoleView) -> Self {
        Self {
            id: Some(role.id.to_string()),
            name: Some(role.name.clone()),
            starting: Some(role.starting.clone()),
            two_step: Some(role.two_step),
            position: Some(role.position),
            permissions: Some(role.permissions.clone()),
        }
    }
}

/// A role as it is after it was made or changed.
#[derive(Serialize, ToSchema)]
pub(crate) struct RoleBody {
    /// The ID of the role.
    id: String,
    /// What it is called.
    name: String,
    /// The starting role it is a copy of (`administrator`, …), `null` for a role of its own.
    starting: Option<String>,
    /// The place in the order of roles.
    position: i64,
    /// Whether everybody with it needs two-step sign-in.
    two_step: bool,
    /// What it allows, in the order of the catalogue.
    permissions: Vec<Permission>,
}

impl Masked for RoleBody {}

impl From<&RoleView> for RoleBody {
    fn from(role: &RoleView) -> Self {
        Self {
            id: role.id.to_string(),
            name: role.name.clone(),
            starting: role.starting.clone(),
            position: role.position,
            two_step: role.two_step,
            permissions: role.permissions.clone(),
        }
    }
}

/// What a role is used for, as far as the person who asks may see.
#[derive(Serialize, ToSchema)]
pub(crate) struct RoleUsedByBody {
    /// Whether somebody has the role for the whole organization.
    whole_organization: bool,
    /// In how many projects the person who asks can see that somebody has the role. Projects
    /// they cannot see are left out, so this number never tells of them.
    projects: u64,
    /// How many people have the role through a row of their own, in the places the person who
    /// asks can see.
    people: u64,
    /// How many groups have the role, in the places the person who asks can see.
    groups: u64,
}

/// One role with what it is used for.
#[derive(Serialize, ToSchema)]
pub(crate) struct RoleDetailBody {
    /// The role.
    role: RoleBody,
    /// What it is used for.
    used_by: RoleUsedByBody,
}

impl Masked for RoleDetailBody {}

impl From<&RoleDetail> for RoleDetailBody {
    fn from(detail: &RoleDetail) -> Self {
        Self {
            role: RoleBody::from(&detail.role),
            used_by: RoleUsedByBody {
                whole_organization: detail.used_by.whole_organization,
                projects: detail.used_by.projects,
                people: detail.used_by.people,
                groups: detail.used_by.groups,
            },
        }
    }
}

/// A role to make.
#[derive(Deserialize, ToSchema)]
pub(crate) struct NewRoleBody {
    /// What the role is called.
    name: String,
    /// What it allows.
    permissions: Vec<Permission>,
    /// Whether everybody with it needs two-step sign-in.
    two_step: bool,
}

/// A role to make from a copy of another.
#[derive(Deserialize, ToSchema)]
pub(crate) struct CopyRoleBody {
    /// What the copy is called.
    name: String,
}

/// What to change about a role: what is left out stays as it is.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ChangeRoleBody {
    /// A new name.
    name: Option<String>,
    /// The permissions from now on.
    permissions: Option<Vec<Permission>>,
    /// The two-step switch from now on.
    two_step: Option<bool>,
}

/// What to say about deleting a role.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct DeleteRoleFilter {
    /// The role that takes the place of the deleted one in every row that gave it. A role that
    /// somebody has is not deleted without one.
    replacement: Option<String>,
}

/// Where a permission counts, as the API names it.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CountsBody {
    /// Only from a role given for the whole organization: a role given on a project that holds
    /// the permission gives nothing for it, and the role page says so.
    Organization,
    /// In projects: from a role given for the whole organization or on a project.
    Project,
}

impl From<Counts> for CountsBody {
    fn from(counts: Counts) -> Self {
        match counts {
            Counts::Organization => Self::Organization,
            Counts::Project => Self::Project,
        }
    }
}

/// One permission of the catalogue as the role page lists it. The sentence is the message
/// `permission.<key>` and the name of the topic `permission-topic.<topic>`.
#[derive(Serialize, ToSchema)]
pub(crate) struct PermissionBody {
    /// The key of the permission.
    key: Permission,
    /// What it is about.
    topic: Topic,
    /// Where it counts.
    scope: CountsBody,
}

/// The permission catalogue, topic by topic.
#[derive(Serialize, ToSchema)]
pub(crate) struct PermissionsBody {
    /// The permissions in the order of the catalogue.
    permissions: Vec<PermissionBody>,
}

impl Masked for PermissionsBody {}

impl From<&PermissionCatalogue> for PermissionsBody {
    fn from(catalogue: &PermissionCatalogue) -> Self {
        Self {
            permissions: catalogue
                .permissions
                .iter()
                .map(|view| PermissionBody {
                    key: view.permission,
                    topic: view.topic,
                    scope: view.counts.into(),
                })
                .collect(),
        }
    }
}

/// The roles, in the order of roles, a page at a time. Needs the permission *Make, change and
/// delete roles*.
#[utoipa::path(
    get,
    path = "/api/v1/roles",
    operation_id = "listRoles",
    tag = "roles",
    extensions(("x-never-stored" = json!(true))),
    params(ListParams),
    responses((status = 200, description = "The roles, in the order of roles.", body = Page<RoleItem>), NotSignedInAnswer, ForbiddenAnswer)
)]
async fn list_roles(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    let position = params.position(ROLES_LIST);
    let cursor = match &position {
        Ok(position) => position.clone(),
        Err(_) => Some(String::new()),
    };
    let found = auth
        .api()?
        .roles(
            info,
            RolesQuery {
                session_token,
                place: None,
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(cursor_error)?;
    position?;
    let listed = found.map(|list| Listed {
        items: list.roles.iter().map(RoleItem::from).collect(),
        next: list.next_cursor,
    });
    let fields = params.fields_readable(&ROLE_FIELDS, &listed.hidden_properties())?;
    Ok(answer(Page::of_authorized(ROLES_LIST, &listed, &fields)?))
}

/// Makes a role. Needs the permission *Make, change and delete roles*; nobody gives a role a
/// permission they do not hold themselves.
#[utoipa::path(
    post,
    path = "/api/v1/roles",
    operation_id = "makeRole",
    tag = "roles",
    request_body = NewRoleBody,
    responses(
        (status = 200, description = "The role.", body = RoleBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        ConflictAnswer
    )
)]
async fn make_role(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<NewRoleBody>,
) -> Result<Response, ApiError> {
    let made = auth
        .api()?
        .make_role(
            info,
            MakeRole {
                session_token,
                name: body.name,
                permissions: body.permissions,
                two_step: body.two_step,
            },
        )
        .await?;
    answer_authorized(&made.map(|role| RoleBody::from(&role)))
}

/// One role, with its permissions and what it is used for. Needs the permission *Make, change and
/// delete roles*.
#[utoipa::path(
    get,
    path = "/api/v1/roles/{role_id}",
    operation_id = "getRole",
    tag = "roles",
    extensions(("x-never-stored" = json!(true))),
    params(("role_id" = String, Path, description = "The ID of the role.")),
    responses((status = 200, description = "The role.", body = RoleDetailBody), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn get_role(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(role_id): ApiPath<String>,
) -> Result<Response, ApiError> {
    let role = parse::<RoleId>(&role_id)?;
    let found = auth
        .api()?
        .role(
            info,
            RoleQuery {
                session_token,
                role,
            },
        )
        .await?;
    answer_authorized(&found.map(|detail| RoleDetailBody::from(&detail)))
}

/// Changes a role: its name, its permissions, its two-step switch. Needs the permission *Make,
/// change and delete roles*; nobody adds a permission they do not hold themselves, and nothing
/// that would leave nobody who can manage roles.
#[utoipa::path(
    patch,
    path = "/api/v1/roles/{role_id}",
    operation_id = "changeRole",
    tag = "roles",
    params(("role_id" = String, Path, description = "The ID of the role.")),
    request_body = ChangeRoleBody,
    responses(
        (status = 200, description = "The role as it is now.", body = RoleBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn change_role(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(role_id): ApiPath<String>,
    ApiJson(body): ApiJson<ChangeRoleBody>,
) -> Result<Response, ApiError> {
    let role = parse::<RoleId>(&role_id)?;
    let changed = auth
        .api()?
        .change_role(
            info,
            ChangeRole {
                session_token,
                role,
                name: body.name,
                permissions: body.permissions,
                two_step: body.two_step,
            },
        )
        .await?;
    answer_authorized(&changed.map(|role| RoleBody::from(&role)))
}

/// Deletes a role. A role that somebody has is deleted only with a `replacement`, which takes its
/// place in every row that gave it. Needs the permission *Make, change and delete roles*.
#[utoipa::path(
    delete,
    path = "/api/v1/roles/{role_id}",
    operation_id = "deleteRole",
    tag = "roles",
    params(("role_id" = String, Path, description = "The ID of the role."), DeleteRoleFilter),
    responses(
        (status = 204, description = "The role is gone."),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn delete_role(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(role_id): ApiPath<String>,
    ApiQuery(filter): ApiQuery<DeleteRoleFilter>,
) -> Result<Response, ApiError> {
    let role = parse::<RoleId>(&role_id)?;
    // A replacement that is no ID is a role that is not there.
    let replacement = filter
        .replacement
        .as_deref()
        .map(parse::<RoleId>)
        .transpose()?;
    auth.api()?
        .delete_role(
            info,
            DeleteRole {
                session_token,
                role,
                replacement,
            },
        )
        .await?;
    Ok(done())
}

/// Makes a role from a copy of another: its permissions, its two-step switch and its levels.
/// Needs the permission *Make, change and delete roles*; nobody copies a role that holds more
/// than they hold themselves.
#[utoipa::path(
    post,
    path = "/api/v1/roles/{role_id}/copy",
    operation_id = "copyRole",
    tag = "roles",
    params(("role_id" = String, Path, description = "The ID of the role to copy.")),
    request_body = CopyRoleBody,
    responses(
        (status = 200, description = "The new role.", body = RoleBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn copy_role(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(role_id): ApiPath<String>,
    ApiJson(body): ApiJson<CopyRoleBody>,
) -> Result<Response, ApiError> {
    let role = parse::<RoleId>(&role_id)?;
    let copied = auth
        .api()?
        .copy_role(
            info,
            CopyRole {
                session_token,
                role,
                name: body.name,
            },
        )
        .await?;
    answer_authorized(&copied.map(|role| RoleBody::from(&role)))
}

/// Gives a starting role back the permissions and levels it had at the start (*Reset to the
/// starting permissions*). Needs the permission *Make, change and delete roles*.
#[utoipa::path(
    post,
    path = "/api/v1/roles/{role_id}/reset",
    operation_id = "resetRole",
    tag = "roles",
    params(("role_id" = String, Path, description = "The ID of the role.")),
    responses(
        (status = 200, description = "The role as it is now.", body = RoleBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn reset_role(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(role_id): ApiPath<String>,
) -> Result<Response, ApiError> {
    let role = parse::<RoleId>(&role_id)?;
    let reset = auth
        .api()?
        .reset_role(
            info,
            ResetRole {
                session_token,
                role,
            },
        )
        .await?;
    answer_authorized(&reset.map(|role| RoleBody::from(&role)))
}

/// The permissions of the catalogue, topic by topic: what the role page lists. Needs the
/// permission *Make, change and delete roles*.
#[utoipa::path(
    get,
    path = "/api/v1/permissions",
    operation_id = "listPermissions",
    tag = "roles",
    extensions(("x-never-stored" = json!(true))),
    responses((status = 200, description = "The permissions of the catalogue.", body = PermissionsBody), NotSignedInAnswer, ForbiddenAnswer)
)]
async fn list_permissions(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let catalogue = auth
        .api()?
        .permissions(info, PermissionsQuery { session_token })
        .await?;
    answer_authorized(&catalogue.map(|catalogue| PermissionsBody::from(&catalogue)))
}

/// The routes of the roles.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_roles, make_role))
        .routes(routes!(get_role, change_role, delete_role))
        .routes(routes!(copy_role))
        .routes(routes!(reset_role))
        .routes(routes!(list_permissions))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perf_02_the_field_table_names_exactly_the_fields_of_a_role_in_a_list() {
        let item = RoleItem {
            id: Some("i".to_owned()),
            name: Some("n".to_owned()),
            starting: Some(None),
            two_step: Some(false),
            position: Some(0),
            permissions: Some(Vec::new()),
        };
        crate::lean::assert_fields_are_the_keys_of(&ROLE_FIELDS, &item);
    }
}
