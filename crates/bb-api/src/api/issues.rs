//! The issue routes (ISS-01, ISS-01a): the issues of a project below
//! `/api/v1/projects/{project}/issues`, and one issue below `/api/v1/issues/{issue}` by its
//! readable ID (`WEB-142`, `WEB-K7Q2X`; an old one too, the answer holds the current one) or its
//! internal ID.
//!
//! An issue the person may not see answers `resource.not_found`, exactly like one that does not
//! exist or is in the trash, and no answer holds a count (PRIV-13). Every change names the
//! `version` it looked at. The use cases are in `bb-app`, behind
//! [`AuthApi`](bb_app::auth::AuthApi).

use axum::extract::State;
use axum::response::Response;
use bb_app::issues::{
    ChangeIssue, DeleteIssue, IssueDetail, IssueQuery, IssueTypeRef as AppIssueTypeRef, IssueView,
    IssuesQuery, MakeIssue, RestoreIssue,
};
use bb_core::ErrorCode;
use bb_core::fields::Field;
use bb_core::id::{IssueTypeId, UserId};
use bb_core::issue::{IssueIcon, Priority, Status, StatusGroup};
use bb_policy::{Masked, PersonRef};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer, answer_authorized, done,
};
use super::projects::ProjectRef;
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};
use crate::lean::{FieldSpec, ListParams, Listed, Page};

/// The fields of an issue in a list that can be asked for, and which of them a list holds when the
/// request names none: what a row of a list needs.
const ISSUE_FIELDS: [FieldSpec; 15] = [
    FieldSpec::by_default("id"),
    FieldSpec::by_default("key"),
    FieldSpec::on_request("project"),
    FieldSpec::by_default("issue_type"),
    FieldSpec::by_default("title"),
    FieldSpec::on_request("description"),
    FieldSpec::by_default("status"),
    FieldSpec::on_request("status_group"),
    FieldSpec::by_default("priority"),
    FieldSpec::on_request("reported_by"),
    FieldSpec::on_request("assigned_to"),
    FieldSpec::on_request("created_at"),
    FieldSpec::on_request("updated_at"),
    FieldSpec::on_request("closed_at"),
    FieldSpec::on_request("version"),
];
/// The name of the list, so its cursors are its own.
const ISSUES_LIST: &str = "issues";

/// An issue type as a small reference: what the line of an issue needs.
#[derive(Serialize, ToSchema, Masked)]
pub(crate) struct IssueTypeRef {
    /// The ID of the type.
    #[masked(open)]
    id: String,
    /// What it is called.
    #[masked(open)]
    name: String,
    /// Its icon.
    #[masked(open)]
    icon: IssueIcon,
    /// Whether it is retired: old issues keep it, new ones cannot have it.
    #[masked(open)]
    retired: bool,
}

impl From<&AppIssueTypeRef> for IssueTypeRef {
    fn from(issue_type: &AppIssueTypeRef) -> Self {
        Self {
            id: issue_type.id.to_string(),
            name: issue_type.name.clone(),
            icon: issue_type.icon,
            retired: issue_type.retired,
        }
    }
}

/// An issue: the fields that can be asked for. An answer holds only those that were asked for (a
/// list holds a few by default; one issue holds all) and that the person may see: a field that is
/// hidden from them is left out, and asking for it fails like asking for one that does not exist.
#[derive(Serialize, ToSchema, Masked)]
pub(crate) struct IssueItem {
    /// The internal ID, which never changes.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(open)]
    id: Option<String>,
    /// The readable ID now (`WEB-142`, `WEB-K7Q2X`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(open)]
    key: Option<String>,
    /// The project it is in.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(nested)]
    project: Option<ProjectRef>,
    /// Its issue type.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(nested)]
    issue_type: Option<IssueTypeRef>,
    /// What it is about.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "title")]
    title: Option<String>,
    /// The long text, in Markdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "description")]
    description: Option<String>,
    /// Where it stands in the workflow.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "status")]
    status: Option<Status>,
    /// The group of its status.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "status")]
    status_group: Option<StatusGroup>,
    /// How important it is.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "priority")]
    priority: Option<Priority>,
    /// Who made it, as the viewer may see people of this kind.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "reported-by", nested)]
    reported_by: Option<PersonRef>,
    /// Who works on it, as the viewer may see people of this kind.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "assigned-to", nested)]
    assigned_to: Option<Vec<PersonRef>>,
    /// When it was made.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(open)]
    created_at: Option<String>,
    /// When it was last changed.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(open)]
    updated_at: Option<String>,
    /// When it entered the group *Done*, while it is in it. Hidden with the status.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "status")]
    closed_at: Option<String>,
    /// Counts up with every change. A change is made on the version that was looked at.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(open)]
    version: Option<i64>,
}

impl From<&IssueView> for IssueItem {
    fn from(issue: &IssueView) -> Self {
        Self {
            id: Some(issue.id.to_string()),
            key: Some(issue.key.clone()),
            project: Some(ProjectRef::from(&issue.project)),
            issue_type: Some(IssueTypeRef::from(&issue.issue_type)),
            title: Some(issue.title.clone()),
            description: Some(issue.description.clone()),
            status: Some(issue.status),
            status_group: Some(issue.status_group),
            priority: Some(issue.priority),
            reported_by: Some(issue.reported_by.clone()),
            assigned_to: Some(issue.assigned_to.clone()),
            created_at: Some(issue.created_at.to_string()),
            updated_at: Some(issue.updated_at.to_string()),
            closed_at: issue.closed_at.map(|at| at.to_string()),
            version: Some(issue.version),
        }
    }
}

/// One issue with what the person may do with it.
#[derive(Serialize, ToSchema, Masked)]
pub(crate) struct IssueDetailBody {
    /// The issue, with every field the person may see.
    #[masked(nested)]
    issue: IssueItem,
    /// The description as safe HTML, made by the server with the same renderer as the editor's
    /// preview (the mark `x-safe-html`). Hidden with the description.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "description")]
    description_html: Option<String>,
    /// Whether the person may change the issue at all, in the fields they may change.
    #[masked(open)]
    may_change: bool,
    /// Whether the person may delete the issue into the trash.
    #[masked(open)]
    may_delete: bool,
    /// The fields the person may change in this issue; empty when they may not change it.
    #[masked(open)]
    changeable: Vec<Field>,
}

impl From<IssueDetail> for IssueDetailBody {
    fn from(detail: IssueDetail) -> Self {
        Self {
            issue: IssueItem::from(&detail.issue),
            description_html: Some(detail.description_html),
            may_change: detail.may_change,
            may_delete: detail.may_delete,
            changeable: detail.changeable,
        }
    }
}

/// An issue to make.
#[derive(Deserialize, ToSchema)]
pub(crate) struct NewIssueBody {
    /// The ID of its issue type; a retired type takes no new issues.
    issue_type: String,
    /// What it is about.
    title: String,
    /// The long text, in Markdown. Empty when left out.
    description: Option<String>,
    /// Where it starts. The first status of the starting workflow when left out.
    status: Option<Status>,
    /// How important it is. *Normal* when left out.
    priority: Option<Priority>,
    /// The IDs of the people it is assigned to. Nobody when left out.
    assigned_to: Option<Vec<String>>,
}

/// What to change about an issue: what is left out stays as it is.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ChangeIssueBody {
    /// The version the person looked at.
    version: i64,
    /// A new issue type; a retired type is refused.
    issue_type: Option<String>,
    /// A new title.
    title: Option<String>,
    /// A new description.
    description: Option<String>,
    /// A new status. Entering the group *Done* sets the closed date; leaving it clears it.
    status: Option<Status>,
    /// A new priority.
    priority: Option<Priority>,
    /// The IDs of the people it is assigned to from now on: all of them, not the ones to add.
    assigned_to: Option<Vec<String>>,
}

/// The ID in a request body; a text that is no ID is not a valid request.
fn id_in_body<Id: std::str::FromStr>(text: &str) -> Result<Id, ApiError> {
    text.parse()
        .map_err(|_| ApiError::new(ErrorCode::InvalidRequest))
}

/// The people in a request body.
fn people_in_body(people: Option<&[String]>) -> Result<Option<Vec<UserId>>, ApiError> {
    people
        .map(|people| people.iter().map(|text| id_in_body(text)).collect())
        .transpose()
}

/// The issues of a project the person sees, newest first, a page at a time. Without *See every
/// issue of the project* the person sees the issues they reported.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{project}/issues",
    operation_id = "listIssues",
    tag = "issues",
    params(("project" = String, Path, description = "The ID of the project, or its short name."), ListParams),
    responses((status = 200, description = "The issues, newest first.", body = Page<IssueItem>), NotSignedInAnswer, NotFoundAnswer)
)]
async fn list_issues(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(project): ApiPath<String>,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    // Who asks is judged first (401 before 400): a cursor that does not open is passed on as one
    // the use case cannot read, so its answer comes after the check of the person.
    let position = params.position(ISSUES_LIST);
    let cursor = match &position {
        Ok(position) => position.clone(),
        Err(_) => Some(String::new()),
    };
    let page = auth
        .api()?
        .issues(
            info,
            IssuesQuery {
                session_token,
                project,
                assigned_to: params.asks_for(&ISSUE_FIELDS, "assigned_to"),
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(super::access_bodies::cursor_error)?;
    position?;
    let list = page.map(|page| Listed {
        items: page.items.iter().map(IssueItem::from).collect(),
        next: page.next_cursor,
    });
    let fields = params.fields_readable(&ISSUE_FIELDS, &list.hidden_properties())?;
    Ok(answer(Page::of_authorized(ISSUES_LIST, &list, &fields)?))
}

/// Makes an issue in a project. Needs the permission *Make issues*; the fields set must be ones
/// the person may change (the title always).
#[utoipa::path(
    post,
    path = "/api/v1/projects/{project}/issues",
    operation_id = "makeIssue",
    tag = "issues",
    params(("project" = String, Path, description = "The ID of the project, or its short name.")),
    request_body = NewIssueBody,
    responses(
        (status = 200, description = "The issue.", body = IssueItem),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn make_issue(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(project): ApiPath<String>,
    ApiJson(body): ApiJson<NewIssueBody>,
) -> Result<Response, ApiError> {
    let issue_type: IssueTypeId = id_in_body(&body.issue_type)?;
    let assigned_to = people_in_body(body.assigned_to.as_deref())?;
    let made = auth
        .api()?
        .make_issue(
            info,
            MakeIssue {
                session_token,
                project,
                issue_type,
                title: body.title,
                description: body.description,
                status: body.status,
                priority: body.priority,
                assigned_to,
            },
        )
        .await?;
    answer_authorized(&made.map(|issue| IssueItem::from(&issue)))
}

/// One issue by its readable ID (an old one too; the answer holds the current one) or its
/// internal ID, with what the person may do with it. Needs the permission *See every issue of the
/// project*, or the issue is one the person reported.
#[utoipa::path(
    get,
    path = "/api/v1/issues/{issue}",
    operation_id = "getIssue",
    tag = "issues",
    params(("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue.")),
    responses((status = 200, description = "The issue.", body = IssueDetailBody), NotSignedInAnswer, NotFoundAnswer)
)]
async fn get_issue(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue): ApiPath<String>,
) -> Result<Response, ApiError> {
    let detail = auth
        .api()?
        .issue(
            info,
            IssueQuery {
                session_token,
                issue,
            },
        )
        .await?;
    answer_authorized(&detail.map(IssueDetailBody::from))
}

/// Changes the fields of an issue. Needs the permission *Change anybody's issues*, or *Change the
/// issues they reported* on an issue the person reported; each field must be one the person may
/// change.
#[utoipa::path(
    patch,
    path = "/api/v1/issues/{issue}",
    operation_id = "changeIssue",
    tag = "issues",
    params(("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue.")),
    request_body = ChangeIssueBody,
    responses(
        (status = 200, description = "The issue as it is now.", body = IssueItem),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn change_issue(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue): ApiPath<String>,
    ApiJson(body): ApiJson<ChangeIssueBody>,
) -> Result<Response, ApiError> {
    let issue_type: Option<IssueTypeId> = body.issue_type.as_deref().map(id_in_body).transpose()?;
    let assigned_to = people_in_body(body.assigned_to.as_deref())?;
    let changed = auth
        .api()?
        .change_issue(
            info,
            ChangeIssue {
                session_token,
                issue,
                version: body.version,
                issue_type,
                title: body.title,
                description: body.description,
                status: body.status,
                priority: body.priority,
                assigned_to,
            },
        )
        .await?;
    answer_authorized(&changed.map(|issue| IssueItem::from(&issue)))
}

/// Puts an issue into the trash. Needs the permission *Delete issues into the trash*. *Undo*
/// takes it out again.
#[utoipa::path(
    delete,
    path = "/api/v1/issues/{issue}",
    operation_id = "deleteIssue",
    tag = "issues",
    params(("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue.")),
    responses(
        (status = 204, description = "The issue is in the trash."),
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn delete_issue(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue): ApiPath<String>,
) -> Result<Response, ApiError> {
    auth.api()?
        .delete_issue(
            info,
            DeleteIssue {
                session_token,
                issue,
            },
        )
        .await?;
    Ok(done())
}

/// Takes an issue out of the trash (*Undo*). Needs the permission *Delete issues into the
/// trash*.
#[utoipa::path(
    post,
    path = "/api/v1/issues/{issue}/restore",
    operation_id = "restoreIssue",
    tag = "issues",
    params(("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue.")),
    responses(
        (status = 200, description = "The issue, out of the trash.", body = IssueItem),
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn restore_issue(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue): ApiPath<String>,
) -> Result<Response, ApiError> {
    let restored = auth
        .api()?
        .restore_issue(
            info,
            RestoreIssue {
                session_token,
                issue,
            },
        )
        .await?;
    answer_authorized(&restored.map(|issue| IssueItem::from(&issue)))
}

/// The routes of the issues.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_issues, make_issue))
        .routes(routes!(get_issue, change_issue, delete_issue))
        .routes(routes!(restore_issue))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perf_02_the_field_table_names_exactly_the_fields_of_an_issue() {
        let item = IssueItem {
            id: Some(String::new()),
            key: Some(String::new()),
            project: Some(ProjectRef {
                id: String::new(),
                short_name: String::new(),
                name: String::new(),
            }),
            issue_type: Some(IssueTypeRef {
                id: String::new(),
                name: String::new(),
                icon: IssueIcon::Bug,
                retired: false,
            }),
            title: Some(String::new()),
            description: Some(String::new()),
            status: Some(Status::Open),
            status_group: Some(StatusGroup::NotStarted),
            priority: Some(Priority::Normal),
            reported_by: Some(PersonRef::Someone),
            assigned_to: Some(Vec::new()),
            created_at: Some(String::new()),
            updated_at: Some(String::new()),
            closed_at: Some(String::new()),
            version: Some(1),
        };
        crate::lean::assert_fields_are_the_keys_of(&ISSUE_FIELDS, &item);
    }
}
