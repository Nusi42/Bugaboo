//! The field permissions below `/api/v1/field-permissions` (SEC-04, UX §5.2): which level every
//! role has for every field of the catalogue, for the whole organization or for one project, and
//! changing one level. *Who sees people* has the same shape (`people_visibility.rs`), so the
//! table and its parts are written once here.
//!
//! Both routes need *Decide what roles see* in the place (a project is given as `?project=`, and
//! one the person may not know about is not found). A level above the person's own is refused
//! (`access.too-much`). The use cases are in `bb-app`, behind
//! [`AuthApi`](bb_app::auth::AuthApi).

use axum::extract::State;
use axum::response::Response;
use bb_app::access::{FieldPermissionsQuery, LevelCell, LevelOrigin, LevelRow, LevelTable};
use bb_app::roles::SetFieldLevel;
use bb_core::access::FieldLevel;
use bb_core::fields::FieldKey;
use bb_core::id::{IssueTypeId, ProjectId, RoleId};
use bb_policy::Masked;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::access_bodies::NamedRef;
use super::access_keys::ProjectLabelBody;
use super::auth::{
    AuthHandle, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer, NotSignedInAnswer, Session,
    answer_authorized, done,
};
use super::path_id;
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};

/// Which place a table or a change is about.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct PlaceParams {
    /// The ID of the project. Without it the answer is about the whole organization: the level
    /// each role has of its own.
    project: Option<String>,
}

impl PlaceParams {
    /// The project, if one is named; a text that is no ID names a project that is not there.
    pub(super) fn project(&self) -> Result<Option<ProjectId>, ApiError> {
        self.project
            .as_deref()
            .map(path_id::<ProjectId>)
            .transpose()
    }
}

/// Which place and which issue type a table, a change or a check of access is about.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct TypeParams {
    /// The ID of the project. Without it the answer is about the whole organization: the level
    /// each role has of its own.
    project: Option<String>,
    /// The ID of an issue type: the levels that count for issues of this type (ADR-0027).
    /// Without it the answer is about all issue types.
    issue_type: Option<String>,
}

impl TypeParams {
    /// The project, if one is named; a text that is no ID names a project that is not there.
    pub(super) fn project(&self) -> Result<Option<ProjectId>, ApiError> {
        self.project
            .as_deref()
            .map(path_id::<ProjectId>)
            .transpose()
    }

    /// The issue type, if one is named; a text that is no ID names a type that is not there.
    pub(super) fn issue_type(&self) -> Result<Option<IssueTypeId>, ApiError> {
        self.issue_type
            .as_deref()
            .map(path_id::<IssueTypeId>)
            .transpose()
    }
}

/// Where the level of a cell was set.
#[derive(Serialize, ToSchema)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum LevelOriginBody {
    /// For the project itself.
    SetHere,
    /// For a project above, which this project and its sub-projects inherit.
    Inherited {
        /// The project above, or *a project you can't see*.
        from: ProjectLabelBody,
    },
    /// The role's own level, for the whole organization.
    RoleOwn,
    /// Nothing is set: closed.
    Unset,
}

impl From<&LevelOrigin> for LevelOriginBody {
    fn from(origin: &LevelOrigin) -> Self {
        match origin {
            LevelOrigin::SetHere => Self::SetHere,
            LevelOrigin::Inherited { from } => Self::Inherited {
                from: ProjectLabelBody::from(from),
            },
            LevelOrigin::RoleOwn => Self::RoleOwn,
            LevelOrigin::Unset => Self::Unset,
        }
    }
}

/// The level of one role.
#[derive(Serialize, ToSchema)]
pub(crate) struct LevelCellBody<L> {
    /// The ID of the role.
    role: String,
    /// Its level in the place.
    level: L,
    /// The level was set for the issue type of the table, not for all issue types.
    for_type: bool,
    /// Where it was set.
    origin: LevelOriginBody,
}

/// A field or a kind of people information with the level of every role.
#[derive(Serialize, ToSchema)]
pub(crate) struct LevelRowBody<K, L> {
    /// The field or the kind.
    subject: K,
    /// The name of an own field; none for a field of the catalogue or a kind.
    name: Option<String>,
    /// The highest level the person who asks may set here: their own.
    yours: L,
    /// One cell for each role, in the order of `roles`.
    cells: Vec<LevelCellBody<L>>,
}

/// What each role sees, for one place.
#[derive(Serialize, ToSchema)]
pub(crate) struct LevelTableBody<K, L> {
    /// The project, or `null` for the whole organization.
    project: Option<ProjectLabelBody>,
    /// The ID of the issue type the levels are for, or `null` for all issue types.
    issue_type: Option<String>,
    /// The roles, in the order of roles: the columns.
    roles: Vec<NamedRef>,
    /// There are more roles than the table holds; the first ones are shown.
    truncated: bool,
    /// The levels a cell can be set to, the lowest first.
    levels: Vec<L>,
    /// The fields or kinds, in the order of their catalogue: the rows.
    rows: Vec<LevelRowBody<K, L>>,
}

impl<K, L> Masked for LevelTableBody<K, L> {}

impl<K: Copy, L: Copy> LevelTableBody<K, L> {
    /// The body of a table; `levels` are the choices of a cell.
    pub(super) fn of(table: &LevelTable<K, L>, levels: &[L]) -> Self {
        Self {
            project: table.place.as_ref().map(ProjectLabelBody::from),
            issue_type: table.issue_type.map(|issue_type| issue_type.to_string()),
            roles: table.roles.iter().map(NamedRef::from).collect(),
            truncated: table.truncated,
            levels: levels.to_vec(),
            rows: table.rows.iter().map(row_body).collect(),
        }
    }
}

fn row_body<K: Copy, L: Copy>(row: &LevelRow<K, L>) -> LevelRowBody<K, L> {
    LevelRowBody {
        subject: row.subject,
        name: row.name.clone(),
        yours: row.yours,
        cells: row.cells.iter().map(cell_body).collect(),
    }
}

fn cell_body<L: Copy>(cell: &LevelCell<L>) -> LevelCellBody<L> {
    LevelCellBody {
        role: cell.role.to_string(),
        level: cell.level,
        for_type: cell.for_type,
        origin: LevelOriginBody::from(&cell.origin),
    }
}

/// The level to set, or `null` to take away what the project set for the role, so the level
/// from above counts again.
#[derive(Deserialize, ToSchema)]
pub(crate) struct NewFieldLevelBody {
    /// One of the `levels` of the table, or `null` (only for a project).
    level: Option<FieldLevel>,
}

/// The path of one cell: a role and a field.
#[derive(Deserialize)]
pub(crate) struct FieldCellPath {
    role: String,
    field: FieldKey,
}

/// Which level every role has for every field, for the whole organization or for one project,
/// and where each level was set. Needs the permission *Decide which fields and which people
/// information each role sees*.
#[utoipa::path(
    get,
    path = "/api/v1/field-permissions",
    operation_id = "getFieldPermissions",
    tag = "access",
    params(TypeParams),
    responses((status = 200, description = "The field permissions.", body = LevelTableBody<FieldKey, FieldLevel>), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn get_field_permissions(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(place): ApiQuery<TypeParams>,
) -> Result<Response, ApiError> {
    let project = place.project()?;
    let issue_type = place.issue_type()?;
    let table = auth
        .api()?
        .field_permissions(
            info,
            FieldPermissionsQuery {
                session_token,
                project,
                issue_type,
            },
        )
        .await?;
    answer_authorized(&table.map(|table| LevelTableBody::of(&table, FieldLevel::ALL)))
}

/// Sets the level of a field for a role, for the whole organization (the role's own level) or for
/// one project and its sub-projects; `null` takes a project's setting away. A level above the
/// person's own is refused. Needs the permission *Decide which fields and which people
/// information each role sees* in the place.
#[utoipa::path(
    put,
    path = "/api/v1/field-permissions/{role}/{field}",
    operation_id = "changeFieldPermission",
    tag = "access",
    params(
        ("role" = String, Path, description = "The ID of the role."),
        ("field" = FieldKey, Path, description = "The key of the field: its word, or the ID of an own field."),
        TypeParams
    ),
    request_body = NewFieldLevelBody,
    responses(
        (status = 204, description = "The level was set."),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn change_field_permission(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(path): ApiPath<FieldCellPath>,
    ApiQuery(place): ApiQuery<TypeParams>,
    ApiJson(body): ApiJson<NewFieldLevelBody>,
) -> Result<Response, ApiError> {
    let project = place.project()?;
    let issue_type = place.issue_type()?;
    let role = path_id::<RoleId>(&path.role)?;
    auth.api()?
        .set_field_level(
            info,
            SetFieldLevel {
                session_token,
                role,
                project,
                issue_type,
                field: path.field,
                level: body.level,
            },
        )
        .await?;
    Ok(done())
}

/// The routes of the field permissions.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(get_field_permissions))
        .routes(routes!(change_field_permission))
}
