//! The routes of the workflow editor below `/api/v1/workflows` (WF-02, WF-04, ISS-02, PRJ-02,
//! ADR-0031) and *Try it* below an issue: the workflow of a place, making one, saying which one a
//! place uses, publishing under the edit lock, taking one away, and the answer of the judge for a
//! person. All need *Manage workflows* at the place. The use cases are in `bb-app`, behind
//! [`AuthApi`](bb_app::auth::AuthApi).

use axum::extract::State;
use axum::response::Response;
use bb_app::workflows::{
    CreateWorkflow, DeleteWorkflow, PublishPreviewQuery, PublishWorkflow, StatusMoveInput,
    TryWorkflowQuery, UseWorkflow, WorkflowEditorQuery, WorkflowsQuery,
};
use bb_core::id::{IssueTypeId, LockToken, UserId, WorkflowId};
use serde::Deserialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::bodies::{
    ChangeDraftBody, PublishPreviewAnswer, PublishPreviewBody, PublishWorkflowBody, PublishedBody,
    StatusDraftBody, TriedBody, WorkflowEditorBody, WorkflowListBody, workflow_in_path,
};
use crate::api::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer_authorized, done,
};
use crate::api::body_id;
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};

/// The issue type in a request, if one is named.
pub(super) fn issue_type_in(text: Option<&str>) -> Result<Option<IssueTypeId>, ApiError> {
    text.map(body_id::<IssueTypeId>).transpose()
}

/// Which workflow to edit.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct WorkflowEditorParams {
    /// The project, by its ID or its short name; leave it out for the whole organization.
    project: Option<String>,
    /// The issue type; leave it out for all types.
    issue_type: Option<String>,
}

/// The place a choice of a workflow is for.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct WorkflowsParams {
    /// The project, by its ID or its short name; leave it out for the whole organization.
    project: Option<String>,
}

/// The workflow to create for a place.
#[derive(Deserialize, ToSchema)]
pub(crate) struct NewWorkflowBody {
    /// The project, by its ID or its short name; leave it out for the whole organization.
    project: Option<String>,
    /// The issue type; leave it out for all types.
    issue_type: Option<String>,
}

/// Which workflow a place uses.
#[derive(Deserialize, ToSchema)]
pub(crate) struct UseWorkflowBody {
    /// The project, by its ID or its short name; leave it out for the whole organization.
    project: Option<String>,
    /// The issue type; leave it out for all types.
    issue_type: Option<String>,
    /// The ID of the workflow the place uses from now on. Leave it out (or `null`) for the place to
    /// follow its parent again.
    workflow: Option<String>,
}

/// The version of the workflow to take away.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct DeleteWorkflowParams {
    /// The version the person looked at.
    version: i64,
}

/// For whom to try the status changes of an issue.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct TryParams {
    /// The ID of the person.
    person: String,
}

/// The workflows there are, by name, for choosing one for a place. Needs the permission *Manage
/// workflows* at the place.
#[utoipa::path(
    get,
    path = "/api/v1/workflows",
    operation_id = "listWorkflows",
    tag = "workflows",
    params(WorkflowsParams),
    responses((status = 200, description = "The workflows.", body = WorkflowListBody), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn list_workflows(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<WorkflowsParams>,
) -> Result<Response, ApiError> {
    let list = auth
        .api()?
        .workflows(
            info,
            WorkflowsQuery {
                session_token,
                project: params.project,
            },
        )
        .await?;
    answer_authorized(&list.map(|list| WorkflowListBody::from(&list)))
}

/// The workflow that counts at a place, for somebody who may change it: its statuses, status
/// changes and rules, with the roles, groups and fields the rules can name. Needs the permission
/// *Manage workflows* at the place.
#[utoipa::path(
    get,
    path = "/api/v1/workflows/editor",
    operation_id = "getWorkflowEditor",
    tag = "workflows",
    params(WorkflowEditorParams),
    responses((status = 200, description = "The workflow with what its rules can name.", body = WorkflowEditorBody), InvalidAnswer, NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn get_workflow_editor(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<WorkflowEditorParams>,
) -> Result<Response, ApiError> {
    let editor = auth
        .api()?
        .workflow_editor(
            info,
            WorkflowEditorQuery {
                session_token,
                project: params.project,
                issue_type: issue_type_in(params.issue_type.as_deref())?,
            },
        )
        .await?;
    answer_authorized(&editor.map(|editor| WorkflowEditorBody::from(&editor)))
}

/// Creates the workflow of a place as a copy of the one that counts there (*Customize*). Needs the
/// permission *Manage workflows* at the place.
#[utoipa::path(
    post,
    path = "/api/v1/workflows",
    operation_id = "createWorkflow",
    tag = "workflows",
    request_body = NewWorkflowBody,
    responses((status = 200, description = "The workflow of the place.", body = WorkflowEditorBody), InvalidAnswer, NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer, ConflictAnswer)
)]
async fn create_workflow(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<NewWorkflowBody>,
) -> Result<Response, ApiError> {
    let editor = auth
        .api()?
        .create_workflow(
            info,
            CreateWorkflow {
                session_token,
                project: body.project,
                issue_type: issue_type_in(body.issue_type.as_deref())?,
            },
        )
        .await?;
    answer_authorized(&editor.map(|editor| WorkflowEditorBody::from(&editor)))
}

/// Says which workflow a place uses, or that it follows its parent again. Needs the permission
/// *Manage workflows* at the place.
#[utoipa::path(
    put,
    path = "/api/v1/workflows/uses",
    operation_id = "useWorkflow",
    tag = "workflows",
    request_body = UseWorkflowBody,
    responses((status = 200, description = "The workflow that counts at the place now.", body = WorkflowEditorBody), InvalidAnswer, NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn use_workflow(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<UseWorkflowBody>,
) -> Result<Response, ApiError> {
    let editor = auth
        .api()?
        .use_workflow(
            info,
            UseWorkflow {
                session_token,
                project: body.project,
                issue_type: issue_type_in(body.issue_type.as_deref())?,
                workflow: body
                    .workflow
                    .as_deref()
                    .map(body_id::<WorkflowId>)
                    .transpose()?,
            },
        )
        .await?;
    answer_authorized(&editor.map(|editor| WorkflowEditorBody::from(&editor)))
}

/// Publishes a workflow under the edit lock the person holds: the statuses, the status changes and
/// their rules are replaced by what is sent, on the version the person looked at. A status that is
/// dropped while issues are in it needs a place to move them to, and they are moved by a change to
/// many issues, with history. Needs the permission *Manage workflows* wherever the workflow is
/// used.
#[utoipa::path(
    put,
    path = "/api/v1/workflows/{workflow}",
    operation_id = "publishWorkflow",
    tag = "workflows",
    params(("workflow" = String, Path, description = "The ID of the workflow.")),
    request_body = PublishWorkflowBody,
    responses((status = 200, description = "The workflow with its new version.", body = PublishedBody), InvalidAnswer, NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer, ConflictAnswer)
)]
async fn publish_workflow(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(workflow): ApiPath<String>,
    ApiJson(body): ApiJson<PublishWorkflowBody>,
) -> Result<Response, ApiError> {
    let published = auth
        .api()?
        .publish_workflow(
            info,
            PublishWorkflow {
                session_token,
                workflow: workflow_in_path(&workflow)?,
                version: body.version,
                lock_token: body_id::<LockToken>(&body.lock_token)?,
                name: body.name.clone(),
                statuses: body
                    .statuses
                    .iter()
                    .map(StatusDraftBody::draft)
                    .collect::<Result<_, _>>()?,
                changes: body
                    .changes
                    .iter()
                    .map(ChangeDraftBody::draft)
                    .collect::<Result<_, _>>()?,
                moves: body
                    .moves
                    .iter()
                    .map(|each| each.parts().map(|(from, to)| StatusMoveInput { from, to }))
                    .collect::<Result<_, _>>()?,
            },
        )
        .await?;
    answer_authorized(&published.map(|published| PublishedBody::from(&published)))
}

/// Says which statuses publishing a workflow would drop and which issues are in them, so the editor
/// can ask where those issues go. The issues the person sees are named; the others are only
/// counted. Needs the permission *Manage workflows* wherever the workflow is used.
#[utoipa::path(
    post,
    path = "/api/v1/workflows/{workflow}/publish-preview",
    operation_id = "previewPublishWorkflow",
    tag = "workflows",
    params(("workflow" = String, Path, description = "The ID of the workflow.")),
    request_body = PublishPreviewBody,
    responses((status = 200, description = "The dropped statuses and their issues.", body = PublishPreviewAnswer), InvalidAnswer, NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn preview_publish_workflow(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(workflow): ApiPath<String>,
    ApiJson(body): ApiJson<PublishPreviewBody>,
) -> Result<Response, ApiError> {
    let preview = auth
        .api()?
        .publish_preview(
            info,
            PublishPreviewQuery {
                session_token,
                workflow: workflow_in_path(&workflow)?,
                statuses: body
                    .statuses
                    .iter()
                    .map(StatusDraftBody::draft)
                    .collect::<Result<_, _>>()?,
                changes: body
                    .changes
                    .iter()
                    .map(ChangeDraftBody::draft)
                    .collect::<Result<_, _>>()?,
            },
        )
        .await?;
    answer_authorized(&preview.map(|preview| PublishPreviewAnswer::from(&preview)))
}

/// Takes a workflow away together with the places it was used at: those places follow their
/// parent again. Needs the permission *Manage workflows* wherever the workflow is used.
#[utoipa::path(
    delete,
    path = "/api/v1/workflows/{workflow}",
    operation_id = "deleteWorkflow",
    tag = "workflows",
    params(("workflow" = String, Path, description = "The ID of the workflow."), DeleteWorkflowParams),
    responses((status = 204, description = "The workflow is gone."), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer, ConflictAnswer)
)]
async fn delete_workflow(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(workflow): ApiPath<String>,
    ApiQuery(params): ApiQuery<DeleteWorkflowParams>,
) -> Result<Response, ApiError> {
    auth.api()?
        .delete_workflow(
            info,
            DeleteWorkflow {
                session_token,
                workflow: workflow_in_path(&workflow)?,
                version: params.version,
            },
        )
        .await?;
    Ok(done())
}

/// *Try it*: the status changes a person would get on an issue, and why not the others. Needs the
/// permission *Manage workflows* in the project of the issue.
#[utoipa::path(
    get,
    path = "/api/v1/issues/{issue}/status-changes",
    operation_id = "tryWorkflow",
    tag = "workflows",
    params(("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue."), TryParams),
    responses((status = 200, description = "The status changes and the answer for each.", body = TriedBody), InvalidAnswer, NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer)
)]
async fn try_workflow(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue): ApiPath<String>,
    ApiQuery(params): ApiQuery<TryParams>,
) -> Result<Response, ApiError> {
    let tried = auth
        .api()?
        .try_workflow(
            info,
            TryWorkflowQuery {
                session_token,
                issue,
                person: body_id::<UserId>(&params.person)?,
            },
        )
        .await?;
    answer_authorized(&tried.map(|tried| TriedBody::from(&tried)))
}

/// The routes of the workflow editor.
pub(super) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_workflows, create_workflow))
        .routes(routes!(get_workflow_editor))
        .routes(routes!(use_workflow))
        .routes(routes!(publish_workflow, delete_workflow))
        .routes(routes!(preview_publish_workflow))
        .routes(routes!(try_workflow))
}
