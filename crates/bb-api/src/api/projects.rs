//! The project routes below `/api/v1/projects` (PRJ-01, PRJ-02): the projects a person sees as a
//! tree or a search, one project by its address, making projects, and changing the name and the
//! short name. Moving a project, with its preview, is in `projects_move.rs`.
//!
//! A project the person may not see answers `resource.not_found`, exactly like one that does not
//! exist, and no answer holds a count of projects. The use cases are in `bb-app`, behind
//! [`AuthApi`](bb_app::auth::AuthApi).

use axum::extract::State;
use axum::response::Response;
use bb_app::auth::AppError;
use bb_app::projects::{
    ChangeProject, MakeProject, MoveRight, ProjectDetail, ProjectItem as AppProjectItem,
    ProjectQuery, ProjectRef as AppProjectRef, ProjectView, ProjectsQuery,
};
use bb_core::ErrorCode;
use bb_core::fields::FieldKey;
use bb_core::id::ProjectId;
use bb_core::own_fields_value::OwnValues;
use bb_policy::{Masked, Permission};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer, answer_authorized,
};
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};
use crate::lean::{FieldSpec, ListParams, Listed, Page};

/// A project as a small reference: what a name in a sentence needs.
#[derive(Serialize, ToSchema)]
pub(crate) struct ProjectRef {
    /// The ID of the project.
    pub(crate) id: String,
    /// The letters of its issue IDs and address.
    pub(crate) short_name: String,
    /// What it is called.
    pub(crate) name: String,
}

impl Masked for ProjectRef {}

impl From<&AppProjectRef> for ProjectRef {
    fn from(project: &AppProjectRef) -> Self {
        Self {
            id: project.id.to_string(),
            short_name: project.short_name.to_string(),
            name: project.name.clone(),
        }
    }
}

/// A project after it was made, changed or moved.
#[derive(Serialize, ToSchema)]
pub(crate) struct ProjectBody {
    /// The ID of the project.
    id: String,
    /// The project it is inside, when the person may see that one.
    parent: Option<String>,
    /// The letters of its issue IDs and address.
    short_name: String,
    /// What it is called.
    name: String,
    /// Counts up with every change. A change is made on the version that was looked at.
    version: i64,
}

impl Masked for ProjectBody {}

impl From<ProjectView> for ProjectBody {
    fn from(project: ProjectView) -> Self {
        Self {
            id: project.id.to_string(),
            parent: project.parent.map(|parent| parent.to_string()),
            short_name: project.short_name.to_string(),
            name: project.name,
            version: project.version,
        }
    }
}

/// The fields of a project in a list that can be asked for, and which of them a list holds when the
/// request names none: what the tree of the sidebar needs.
const PROJECT_FIELDS: [FieldSpec; 5] = [
    FieldSpec::by_default("id"),
    FieldSpec::by_default("short_name"),
    FieldSpec::by_default("name"),
    FieldSpec::by_default("has_sub_projects"),
    FieldSpec::on_request("parent"),
];
/// The name of the list, so its cursors are its own.
const PROJECTS_LIST: &str = "projects";

/// A project in a list: the fields that can be asked for. An answer holds only those that were
/// asked for.
#[derive(Serialize, ToSchema)]
pub(crate) struct ProjectItem {
    /// The ID of the project.
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    /// The letters of its issue IDs and address.
    #[serde(skip_serializing_if = "Option::is_none")]
    short_name: Option<String>,
    /// What it is called.
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    /// Whether it holds sub-projects the person sees. A yes or no, never a number.
    #[serde(skip_serializing_if = "Option::is_none")]
    has_sub_projects: Option<bool>,
    /// The project it is inside, when the person may see that one.
    #[serde(skip_serializing_if = "Option::is_none")]
    parent: Option<ProjectRef>,
}

impl Masked for ProjectItem {}

impl From<&AppProjectItem> for ProjectItem {
    fn from(project: &AppProjectItem) -> Self {
        Self {
            id: Some(project.id.to_string()),
            short_name: Some(project.short_name.to_string()),
            name: Some(project.name.clone()),
            has_sub_projects: Some(project.has_sub_projects),
            parent: project.parent.as_ref().map(ProjectRef::from),
        }
    }
}

/// What narrows a list of projects.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct ProjectFilter {
    /// The projects directly inside this one. Without it (and without `search`) the list is the top
    /// of the tree.
    inside: Option<String>,
    /// Words that name the projects to find anywhere in the tree, in their name or short name.
    search: Option<String>,
    /// Leave out this project and everything inside it. Ignored for a project the person does not
    /// see, or that does not exist.
    outside: Option<String>,
    /// Only the places this project may be moved into by the person: not itself, not inside
    /// itself, not where it is now, and only where they may move projects. Ignored for a project
    /// the person does not see, or that does not exist.
    movable: Option<String>,
}

/// Where a person may move a project to.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum MoveRightBody {
    /// Not anywhere.
    Nowhere,
    /// Into other projects, not to the top level.
    IntoProjects,
    /// Into other projects, and to the top level.
    IntoProjectsOrTop,
}

impl From<MoveRight> for MoveRightBody {
    fn from(right: MoveRight) -> Self {
        match right {
            MoveRight::Nowhere => Self::Nowhere,
            MoveRight::IntoProjects => Self::IntoProjects,
            MoveRight::IntoProjectsOrTop => Self::IntoProjectsOrTop,
        }
    }
}

/// One project, with where it is and what the person may do there.
#[derive(Serialize, ToSchema)]
pub(crate) struct ProjectDetailBody {
    /// The project. Its `short_name` is the one it has now, also when the address named an old one.
    project: ProjectBody,
    /// *Where you are*: the projects above it that the person sees, from the top down. It stops at
    /// the first one they cannot see.
    trail: Vec<ProjectRef>,
    /// The trail was cut.
    truncated: bool,
    /// What the person may do here.
    permissions: Vec<Permission>,
    /// Whether the person may change the short name.
    may_change_short_name: bool,
    /// Where the person may move the project to.
    may_move: MoveRightBody,
    /// The values of the own fields of the project, by the key (ID) of the field: only the fields
    /// the person may see.
    own_fields: OwnValues,
    /// The own fields the person may change in this project, by their keys.
    changeable_fields: Vec<FieldKey>,
}

impl Masked for ProjectDetailBody {
    fn mask(value: &mut bb_policy::MaskedValue, mask: &bb_policy::FieldMask) {
        bb_policy::drop_hidden_own_fields(value, "own_fields", mask);
    }
}

impl From<ProjectDetail> for ProjectDetailBody {
    fn from(detail: ProjectDetail) -> Self {
        Self {
            project: detail.project.into(),
            trail: detail.trail.iter().map(ProjectRef::from).collect(),
            truncated: detail.truncated,
            permissions: detail.permissions,
            may_change_short_name: detail.may_change_short_name,
            may_move: detail.may_move.into(),
            own_fields: detail.own_fields,
            changeable_fields: detail.changeable_fields,
        }
    }
}

/// A project to make.
#[derive(Deserialize, ToSchema)]
pub(crate) struct NewProjectBody {
    /// The letters of its issue IDs and address: 2 to 10 capital letters or digits, starting with a
    /// letter.
    short_name: String,
    /// What it is called.
    name: String,
}

/// What to change about a project: what is left out stays as it is.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ChangeProjectBody {
    /// The version the person looked at.
    version: i64,
    /// A new name.
    name: Option<String>,
    /// A new short name. The old one stays with the project.
    short_name: Option<String>,
    /// The values of own fields to set, by the key (ID) of the field; `null` takes a value away.
    #[serde(default)]
    fields: std::collections::BTreeMap<String, serde_json::Value>,
}

/// The ID in an address of the API; a text that is no ID names no project.
pub(crate) fn project_id(text: &str) -> Result<ProjectId, ApiError> {
    text.parse().map_err(|_| ApiError::new(ErrorCode::NotFound))
}

/// The projects the person sees, a page at a time: the top of the tree, the projects inside one,
/// or the projects that a search finds. Needs no permission of its own: it lists what the person
/// may see, which is nothing for a person without a role.
#[utoipa::path(
    get,
    path = "/api/v1/projects",
    operation_id = "listProjects",
    tag = "projects",
    params(ListParams, ProjectFilter),
    responses((status = 200, description = "The projects, by name.", body = Page<ProjectItem>), NotSignedInAnswer, NotFoundAnswer)
)]
async fn list_projects(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<ListParams>,
    ApiQuery(filter): ApiQuery<ProjectFilter>,
) -> Result<Response, ApiError> {
    // Who asks is judged first (401 before 400): a cursor that does not open is passed on as one
    // the use case cannot read, so its answer comes after the check of the person.
    let position = params.position(PROJECTS_LIST);
    let cursor = match &position {
        Ok(position) => position.clone(),
        Err(_) => Some(String::new()),
    };
    let inside = filter.inside.as_deref().map(project_id).transpose();
    let outside = filter.outside.as_deref().map(project_id).transpose();
    let movable = filter.movable.as_deref().map(project_id).transpose();
    let page = auth
        .api()?
        .projects(
            info,
            ProjectsQuery {
                session_token,
                inside: inside.as_ref().ok().copied().flatten(),
                search: filter.search,
                outside: outside.as_ref().ok().copied().flatten(),
                movable: movable.as_ref().ok().copied().flatten(),
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
    // A project that is no ID is a project that is not there.
    inside?;
    outside?;
    movable?;
    let list = page.map(|page| Listed {
        items: page.items.iter().map(ProjectItem::from).collect(),
        next: page.next_cursor,
    });
    let fields = params.fields_readable(&PROJECT_FIELDS, &list.hidden_properties())?;
    Ok(answer(Page::of_authorized(PROJECTS_LIST, &list, &fields)?))
}

/// One project by its address: its ID, or its short name (an old one too). Needs no permission of
/// its own: a project the person may not see is not found.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{project}",
    operation_id = "getProject",
    tag = "projects",
    params(("project" = String, Path, description = "The ID of the project, or its short name.")),
    responses((status = 200, description = "The project.", body = ProjectDetailBody), NotSignedInAnswer, NotFoundAnswer)
)]
async fn get_project(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(address): ApiPath<String>,
) -> Result<Response, ApiError> {
    let detail = auth
        .api()?
        .project(
            info,
            ProjectQuery {
                session_token,
                address,
            },
        )
        .await?;
    answer_authorized(&detail.map(ProjectDetailBody::from))
}

/// Makes a project at the top level. Needs the permission *Make projects at the top level*.
#[utoipa::path(
    post,
    path = "/api/v1/projects",
    operation_id = "makeProject",
    tag = "projects",
    request_body = NewProjectBody,
    responses(
        (status = 200, description = "The project.", body = ProjectBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        ConflictAnswer
    )
)]
async fn make_project(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<NewProjectBody>,
) -> Result<Response, ApiError> {
    let made = auth
        .api()?
        .make_project(
            info,
            MakeProject {
                session_token,
                parent: None,
                short_name: body.short_name,
                name: body.name,
            },
        )
        .await?;
    answer_authorized(&made.map(ProjectBody::from))
}

/// Makes a sub-project. Needs the permission *Make sub-projects inside the project* in the
/// project it goes into.
#[utoipa::path(
    post,
    path = "/api/v1/projects/{project}/sub-projects",
    operation_id = "makeSubProject",
    tag = "projects",
    params(("project" = String, Path, description = "The ID of the project it goes into.")),
    request_body = NewProjectBody,
    responses(
        (status = 200, description = "The project.", body = ProjectBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn make_sub_project(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(parent): ApiPath<String>,
    ApiJson(body): ApiJson<NewProjectBody>,
) -> Result<Response, ApiError> {
    let parent = project_id(&parent)?;
    let made = auth
        .api()?
        .make_project(
            info,
            MakeProject {
                session_token,
                parent: Some(parent),
                short_name: body.short_name,
                name: body.name,
            },
        )
        .await?;
    answer_authorized(&made.map(ProjectBody::from))
}

/// Changes the name or the short name of a project. A short name that is changed leaves the old
/// one with the project, so its old addresses keep working. Needs the permission *Change the
/// project's name, short name and settings*; a short name also needs the right to make projects
/// where the project sits.
#[utoipa::path(
    patch,
    path = "/api/v1/projects/{project}",
    operation_id = "changeProject",
    tag = "projects",
    params(("project" = String, Path, description = "The ID of the project.")),
    request_body = ChangeProjectBody,
    responses(
        (status = 200, description = "The project as it is now.", body = ProjectBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn change_project(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(project): ApiPath<String>,
    ApiJson(body): ApiJson<ChangeProjectBody>,
) -> Result<Response, ApiError> {
    let project = project_id(&project)?;
    let changed = auth
        .api()?
        .change_project(
            info,
            ChangeProject {
                session_token,
                project,
                version: body.version,
                name: body.name,
                short_name: body.short_name,
                fields: body.fields,
            },
        )
        .await?;
    answer_authorized(&changed.map(ProjectBody::from))
}

/// The routes of the projects, but the moving ones.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_projects, make_project))
        .routes(routes!(get_project, change_project))
        .routes(routes!(make_sub_project))
        .merge(super::projects_move::routes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perf_02_the_field_table_names_exactly_the_fields_of_a_project_in_a_list() {
        let item = ProjectItem {
            id: Some("i".to_owned()),
            short_name: Some("s".to_owned()),
            name: Some("n".to_owned()),
            has_sub_projects: Some(false),
            parent: Some(ProjectRef {
                id: "p".to_owned(),
                short_name: "P".to_owned(),
                name: "p".to_owned(),
            }),
        };
        crate::lean::assert_fields_are_the_keys_of(&PROJECT_FIELDS, &item);
    }
}
