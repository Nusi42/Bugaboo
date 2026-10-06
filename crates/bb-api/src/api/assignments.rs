//! *Who has which role* below `/api/v1/assignments` and `/api/v1/projects/{project}/assignments`
//! (SEC-02, SEC-03): the rows of the whole organization or of one project (with the rows that
//! come from above), giving a role or *No access* to a person or a group, taking a row away, and
//! the choices the form offers.
//!
//! All of them need the permission *Decide who has which role* where the rows are (the whole
//! organization, or the project in the address); a project the person may not know about is not
//! found. What the use cases refuse beyond that (nobody gives what they do not hold, the last
//! administrator, a role and *No access* in one project) comes back as the refusal of the use
//! case.

use axum::extract::State;
use axum::response::Response;
use bb_app::auth::RequestInfo;
use bb_app::people::{AssignmentChoicesQuery, ChoiceKind};
use bb_app::roles::{
    AssignRole, AssignmentView, AssignmentsQuery, Given, RemoveAssignment, Subject, WhoView,
};
use bb_core::ErrorCode;
use bb_core::access::AppliesTo;
use bb_core::id::{AssignmentId, GroupId, ProjectId, RoleId, UserId};
use bb_policy::{Masked, PersonRef};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::access_bodies::{AppliesToBody, GroupRef, PlaceRef, RoleRef, cursor_error};
use super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer, answer_authorized, done,
};
use super::groups::{ChoiceItem, choice_page};
use super::people::parse;
use super::projects::project_id;
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};
use crate::lean::{FieldSpec, ListParams, Listed, Page};

/// The fields of a row in a list that can be asked for, and which of them a list holds when the
/// request names none: everything a row says.
const ROW_FIELDS: [FieldSpec; 8] = [
    FieldSpec::by_default("id"),
    FieldSpec::by_default("person"),
    FieldSpec::by_default("group"),
    FieldSpec::by_default("role"),
    FieldSpec::by_default("place"),
    FieldSpec::by_default("applies_to"),
    FieldSpec::by_default("inherited"),
    FieldSpec::on_request("given_at"),
];
/// The names of the lists, so their cursors are their own.
const ASSIGNMENTS_LIST: &str = "assignments";
const CHOICES_LIST: &str = "assignment-choices";

/// A row of *Who has which role*: the fields that can be asked for.
#[derive(Serialize, ToSchema)]
pub(crate) struct AssignmentItem {
    /// The ID of the row.
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    /// The person the row is for, shown as the person who asks may see them; not there for a
    /// group.
    #[serde(skip_serializing_if = "Option::is_none")]
    person: Option<PersonRef>,
    /// The group the row is for; not there for a person.
    #[serde(skip_serializing_if = "Option::is_none")]
    group: Option<GroupRef>,
    /// The role; not there for *No access*.
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<RoleRef>,
    /// Where the row is.
    #[serde(skip_serializing_if = "Option::is_none")]
    place: Option<PlaceRef>,
    /// Where it counts.
    #[serde(skip_serializing_if = "Option::is_none")]
    applies_to: Option<AppliesToBody>,
    /// Whether the row comes from above the place that was asked for (a parent project, or the
    /// whole organization).
    #[serde(skip_serializing_if = "Option::is_none")]
    inherited: Option<bool>,
    /// When the row was made.
    #[serde(skip_serializing_if = "Option::is_none")]
    given_at: Option<String>,
}

impl Masked for AssignmentItem {}

impl From<&AssignmentView> for AssignmentItem {
    fn from(row: &AssignmentView) -> Self {
        let (person, group) = match &row.who {
            WhoView::Person { person } => (Some(person.clone()), None),
            WhoView::Group { group } => (None, Some(GroupRef::from(group))),
        };
        Self {
            id: Some(row.id.to_string()),
            person,
            group,
            role: row.role.as_ref().map(RoleRef::from),
            place: Some(PlaceRef::from(row.project.as_ref())),
            applies_to: Some(row.applies_to.into()),
            inherited: Some(row.inherited),
            given_at: Some(row.given_at.to_string()),
        }
    }
}

/// What kind of thing a choice is.
#[derive(Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ChoiceKindBody {
    /// People.
    Person,
    /// Groups.
    Group,
    /// Roles.
    Role,
}

impl From<ChoiceKindBody> for ChoiceKind {
    fn from(kind: ChoiceKindBody) -> Self {
        match kind {
            ChoiceKindBody::Person => Self::Person,
            ChoiceKindBody::Group => Self::Group,
            ChoiceKindBody::Role => Self::Role,
        }
    }
}

/// What the choices are for.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct ChoicesFilter {
    /// What to list: people (when left out), groups or roles.
    #[param(inline)]
    kind: Option<ChoiceKindBody>,
    /// Words that every found name holds, without regard to capitals.
    search: Option<String>,
}

/// Whom a row is for: name a person or a group, not both.
#[derive(Deserialize, ToSchema)]
pub(crate) struct WhoBody {
    /// The ID of the person.
    person: Option<String>,
    /// The ID of the group.
    group: Option<String>,
}

/// A row for the whole organization to give: to a person or to a group, and the role.
#[derive(Deserialize, ToSchema)]
pub(crate) struct NewOrganizationRowBody {
    /// Whom the row is for.
    who: WhoBody,
    /// The ID of the role.
    role: String,
}

/// A row on a project to give: to a person or to a group, a role or *No access*, and where it
/// counts.
#[derive(Deserialize, ToSchema)]
pub(crate) struct NewProjectRowBody {
    /// Whom the row is for.
    who: WhoBody,
    /// The ID of the role; left out for *No access*.
    role: Option<String>,
    /// Where it counts: only the project, or the project and its sub-projects. *No access* always
    /// covers the project and its sub-projects.
    applies_to: AppliesToBody,
}

/// Whom a body names. A person or group that is no ID is one that is not there.
pub(super) fn subject_of(who: &WhoBody) -> Result<Subject, ApiError> {
    match (who.person.as_deref(), who.group.as_deref()) {
        (Some(person), None) => parse::<UserId>(person).map(Subject::Person),
        (None, Some(group)) => parse::<GroupId>(group).map(Subject::Group),
        // A row is for exactly one person or group.
        _ => Err(ApiError::new(ErrorCode::InvalidRequest)),
    }
}

/// The rows of a place, as the answer of a list route.
async fn list_rows(
    auth: AuthHandle,
    info: RequestInfo,
    session_token: String,
    params: ListParams,
    project: Option<ProjectId>,
) -> Result<Response, ApiError> {
    let position = params.position(ASSIGNMENTS_LIST);
    let cursor = match &position {
        Ok(position) => position.clone(),
        Err(_) => Some(String::new()),
    };
    let found = auth
        .api()?
        .assignments(
            info,
            AssignmentsQuery {
                session_token,
                project,
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(cursor_error)?;
    position?;
    let listed = found.map(|list| Listed {
        items: list.rows.iter().map(AssignmentItem::from).collect(),
        next: list.next_cursor,
    });
    let fields = params.fields_readable(&ROW_FIELDS, &listed.hidden_properties())?;
    Ok(answer(Page::of_authorized(
        ASSIGNMENTS_LIST,
        &listed,
        &fields,
    )?))
}

/// Gives a row, and answers with it.
async fn give_row(
    auth: AuthHandle,
    info: RequestInfo,
    input: AssignRole,
) -> Result<Response, ApiError> {
    let made = auth.api()?.assign_role(info, input).await?;
    answer_authorized(&made.map(|row| AssignmentItem::from(&row)))
}

/// The choices of a place, as the answer of a list route.
async fn list_choices(
    auth: AuthHandle,
    info: RequestInfo,
    session_token: String,
    params: ListParams,
    filter: ChoicesFilter,
    project: Option<ProjectId>,
) -> Result<Response, ApiError> {
    let position = params.position(CHOICES_LIST);
    let cursor = match &position {
        Ok(position) => position.clone(),
        Err(_) => Some(String::new()),
    };
    let found = auth
        .api()?
        .assignment_choices(
            info,
            AssignmentChoicesQuery {
                session_token,
                project,
                kind: filter.kind.map_or(ChoiceKind::Person, ChoiceKind::from),
                search: filter.search,
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(cursor_error)?;
    position?;
    choice_page(CHOICES_LIST, &params, found)
}

/// The rows of *Who has which role* for the whole organization, a page at a time. Needs the
/// permission *Decide who has which role* for the whole organization.
#[utoipa::path(
    get,
    path = "/api/v1/assignments",
    operation_id = "listAssignments",
    tag = "roles",
    extensions(("x-never-stored" = json!(true))),
    params(ListParams),
    responses((status = 200, description = "The rows for the whole organization.", body = Page<AssignmentItem>), NotSignedInAnswer, ForbiddenAnswer)
)]
async fn list_assignments(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    list_rows(auth, info, session_token, params, None).await
}

/// Gives a role to a person or a group for the whole organization. Needs the permission *Decide
/// who has which role* for the whole organization; nobody gives a role with more than they hold
/// themselves.
#[utoipa::path(
    post,
    path = "/api/v1/assignments",
    operation_id = "assignRole",
    tag = "roles",
    request_body = NewOrganizationRowBody,
    responses(
        (status = 200, description = "The row.", body = AssignmentItem),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn assign_role(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<NewOrganizationRowBody>,
) -> Result<Response, ApiError> {
    let subject = subject_of(&body.who)?;
    let role = parse::<RoleId>(&body.role)?;
    give_row(
        auth,
        info,
        AssignRole {
            session_token,
            subject,
            given: Given::Role(role),
            project: None,
            applies_to: AppliesTo::Organization,
        },
    )
    .await
}

/// The rows of *Who has which role* for a project, those on the project first and then those
/// that come from above, a page at a time. Needs the permission *Decide who has which role* in
/// the project.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{project}/assignments",
    operation_id = "listProjectAssignments",
    tag = "roles",
    extensions(("x-never-stored" = json!(true))),
    params(("project" = String, Path, description = "The ID of the project."), ListParams),
    responses((status = 200, description = "The rows, those on the project first.", body = Page<AssignmentItem>), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn list_project_assignments(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(project): ApiPath<String>,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    let project = project_id(&project)?;
    list_rows(auth, info, session_token, params, Some(project)).await
}

/// Gives a role, or *No access*, to a person or a group on a project. Needs the permission
/// *Decide who has which role* in the project; nobody gives a role with more than they hold
/// there themselves.
#[utoipa::path(
    post,
    path = "/api/v1/projects/{project}/assignments",
    operation_id = "assignProjectRole",
    tag = "roles",
    params(("project" = String, Path, description = "The ID of the project.")),
    request_body = NewProjectRowBody,
    responses(
        (status = 200, description = "The row.", body = AssignmentItem),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn assign_project_role(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(project): ApiPath<String>,
    ApiJson(body): ApiJson<NewProjectRowBody>,
) -> Result<Response, ApiError> {
    let project = project_id(&project)?;
    let subject = subject_of(&body.who)?;
    let given = match body.role.as_deref() {
        Some(role) => Given::Role(parse::<RoleId>(role)?),
        None => Given::NoAccess,
    };
    give_row(
        auth,
        info,
        AssignRole {
            session_token,
            subject,
            given,
            project: Some(project),
            applies_to: body.applies_to.into(),
        },
    )
    .await
}

/// Takes a row of *Who has which role* away. Needs the permission *Decide who has which role*
/// where the row is; nothing that would leave nobody who can manage roles.
#[utoipa::path(
    delete,
    path = "/api/v1/assignments/{assignment_id}",
    operation_id = "removeAssignment",
    tag = "roles",
    params(("assignment_id" = String, Path, description = "The ID of the row.")),
    responses(
        (status = 204, description = "The row is gone."),
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn remove_assignment(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(assignment_id): ApiPath<String>,
) -> Result<Response, ApiError> {
    let assignment = parse::<AssignmentId>(&assignment_id)?;
    auth.api()?
        .remove_assignment(
            info,
            RemoveAssignment {
                session_token,
                assignment,
            },
        )
        .await?;
    Ok(done())
}

/// The people, groups or roles that the form of a row for the whole organization offers, by
/// name, a page at a time. Needs the permission *Decide who has which role* for the whole
/// organization. Names only, never e-mail addresses.
#[utoipa::path(
    get,
    path = "/api/v1/assignment-choices",
    operation_id = "listAssignmentChoices",
    tag = "roles",
    extensions(("x-never-stored" = json!(true))),
    params(ListParams, ChoicesFilter),
    responses((status = 200, description = "What to choose from.", body = Page<ChoiceItem>), NotSignedInAnswer, ForbiddenAnswer)
)]
async fn list_assignment_choices(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<ListParams>,
    ApiQuery(filter): ApiQuery<ChoicesFilter>,
) -> Result<Response, ApiError> {
    list_choices(auth, info, session_token, params, filter, None).await
}

/// The people, groups or roles that the form of a row on a project offers, by name, a page at a
/// time. Needs the permission *Decide who has which role* in the project. Names only, never
/// e-mail addresses.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{project}/assignment-choices",
    operation_id = "listProjectAssignmentChoices",
    tag = "roles",
    extensions(("x-never-stored" = json!(true))),
    params(("project" = String, Path, description = "The ID of the project."), ListParams, ChoicesFilter),
    responses((status = 200, description = "What to choose from.", body = Page<ChoiceItem>), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn list_project_assignment_choices(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(project): ApiPath<String>,
    ApiQuery(params): ApiQuery<ListParams>,
    ApiQuery(filter): ApiQuery<ChoicesFilter>,
) -> Result<Response, ApiError> {
    let project = project_id(&project)?;
    list_choices(auth, info, session_token, params, filter, Some(project)).await
}

/// The routes of *Who has which role*.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_assignments, assign_role))
        .routes(routes!(list_project_assignments, assign_project_role))
        .routes(routes!(remove_assignment))
        .routes(routes!(list_assignment_choices))
        .routes(routes!(list_project_assignment_choices))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perf_02_the_field_table_names_exactly_the_fields_of_a_row_in_a_list() {
        let item = AssignmentItem {
            id: Some("i".to_owned()),
            person: Some(PersonRef::Someone),
            group: Some(GroupRef {
                id: "g".to_owned(),
                name: "n".to_owned(),
            }),
            role: Some(RoleRef {
                id: "r".to_owned(),
                name: "n".to_owned(),
            }),
            place: Some(PlaceRef::Organization),
            applies_to: Some(AppliesToBody::Organization),
            inherited: Some(false),
            given_at: Some("t".to_owned()),
        };
        crate::lean::assert_fields_are_the_keys_of(&ROW_FIELDS, &item);
    }
}
