//! Workflows in the API (WF-01, ADR-0031): a status is sent as its key, its name and its status
//! group, never as a word of a fixed list, and every issue carries the status changes the reader
//! may make now. The editor's routes are in `workflows/editor.rs`.

use axum::extract::State;
use axum::response::Response;
use bb_app::workflows::{ProjectStatuses, ProjectStatusesQuery, StatusChangeView, StatusView};
use bb_core::fields::FieldKey;
use bb_core::issue::StatusGroup;
use bb_core::workflow::StatusKey;
use bb_policy::Masked;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, Info, NotFoundAnswer, NotSignedInAnswer, Session, answer_authorized,
};
use super::body_id;
use crate::errors::{ApiError, ApiPath};

mod bodies;
mod editor;
mod who_may_change;

/// A status as it is shown: the browser keeps no list of statuses, it shows what the server sends.
#[derive(Clone, Serialize, ToSchema)]
pub(crate) struct StatusRef {
    /// The key, which never changes: it is what a request names a status by.
    key: String,
    /// What it is called.
    name: String,
    /// The status group it belongs to.
    group: StatusGroup,
}

impl From<&StatusView> for StatusRef {
    fn from(status: &StatusView) -> Self {
        Self {
            key: status.key.to_string(),
            name: status.name.clone(),
            group: status.group,
        }
    }
}

#[cfg(test)]
impl StatusRef {
    /// A status with nothing in it, for the tests that fill every member of an answer.
    pub(crate) fn blank() -> Self {
        Self {
            key: String::new(),
            name: String::new(),
            group: StatusGroup::NotStarted,
        }
    }
}

/// A status change the reader may make on an issue now.
#[derive(Serialize, ToSchema)]
pub(crate) struct StatusChangeRef {
    /// The status it ends in.
    to: StatusRef,
    /// The text of the button.
    button: String,
    /// The fields the change still needs before it can be made (*Must be filled in* fields that
    /// are empty, and *Ask for* fields), by their keys. Empty when it needs none.
    needs: Vec<FieldKey>,
}

impl From<&StatusChangeView> for StatusChangeRef {
    fn from(change: &StatusChangeView) -> Self {
        Self {
            to: StatusRef::from(&change.to),
            button: change.button.clone(),
            needs: change.needs.clone(),
        }
    }
}

/// The status in a request body; a text that is no status key is not a valid request.
pub(crate) fn status_in(text: Option<&str>) -> Result<Option<StatusKey>, ApiError> {
    text.map(body_id::<StatusKey>).transpose()
}

/// The statuses the issues of a project can have: one list for all issue types. A status with the
/// same key, name and group in several workflows is listed once; one with the same key and another
/// name or group is listed for each.
#[derive(Serialize, ToSchema)]
pub(crate) struct ProjectStatusesBody {
    /// The statuses, in the order the workflows list them.
    statuses: Vec<StatusRef>,
    /// The status changes by their words, once for the same status and text, in the order the
    /// workflows list them: what a choice of a status change for many issues offers.
    changes: Vec<ProjectStatusChangeBody>,
}

/// A status change of some workflow of the project.
#[derive(Serialize, ToSchema)]
pub(crate) struct ProjectStatusChangeBody {
    /// The status it ends in.
    to: StatusRef,
    /// The text of the button.
    button: String,
}

impl Masked for ProjectStatusesBody {}

impl From<&ProjectStatuses> for ProjectStatusesBody {
    fn from(list: &ProjectStatuses) -> Self {
        Self {
            statuses: list.statuses.iter().map(StatusRef::from).collect(),
            changes: list
                .changes
                .iter()
                .map(|change| ProjectStatusChangeBody {
                    to: StatusRef::from(&change.to),
                    button: change.button.clone(),
                })
                .collect(),
        }
    }
}

/// The statuses the issues of a project can have, for choosing one for many issues at once. Each
/// issue is still judged by its own workflow. Needs the permission *See the project*.
#[utoipa::path(
    get,
    path = "/api/v1/projects/{project}/statuses",
    operation_id = "getProjectStatuses",
    tag = "workflows",
    params(("project" = String, Path, description = "The ID of the project, or its short name.")),
    responses((status = 200, description = "The statuses.", body = ProjectStatusesBody), NotSignedInAnswer, NotFoundAnswer)
)]
async fn get_project_statuses(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(project): ApiPath<String>,
) -> Result<Response, ApiError> {
    let statuses = auth
        .api()?
        .project_statuses(
            info,
            ProjectStatusesQuery {
                session_token,
                project,
            },
        )
        .await?;
    answer_authorized(&statuses.map(|list| ProjectStatusesBody::from(&list)))
}

/// The routes of the workflows.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(get_project_statuses))
        .merge(editor::routes())
        .merge(who_may_change::routes())
}
