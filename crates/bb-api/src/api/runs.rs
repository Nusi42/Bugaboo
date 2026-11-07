//! The routes of runs (AI-38): a run of an AI app, an agent or a change to many issues, the changes
//! in it as the reader may see them, and *Undo the run*. Only the person a run belongs to reads it
//! or undoes it; for anybody else it is not there.

use axum::extract::State;
use axum::response::Response;
use bb_app::runs::{
    RunChangeView, RunChangesQuery, RunQuery, RunView, UndoRun, UndoSkipView, UndoView,
};
use bb_core::id::{EventId, RunId};
use bb_core::many_changes::ManyChangeStatus;
use bb_core::run::{RunKind, UndoSkipKind, UndoVersion};
use bb_policy::Masked;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer, answer_authorized,
};
use super::many_changes::FieldChangeBody;
use super::relations::IssueRef;
use super::{body_id, path_id};
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};
use crate::lean::{FieldSpec, ListParams, Listed, Page};

/// A run, as the person it belongs to sees it.
#[derive(Serialize, ToSchema)]
pub(crate) struct RunBody {
    /// The ID of the run.
    id: String,
    /// What it is of.
    kind: RunKind,
    /// Where it is.
    status: ManyChangeStatus,
    /// The name of the AI app, for a session of an app.
    #[serde(skip_serializing_if = "Option::is_none")]
    app: Option<String>,
    /// When it began.
    started_at: String,
    /// When it ended.
    #[serde(skip_serializing_if = "Option::is_none")]
    ended_at: Option<String>,
}

impl Masked for RunBody {}

impl From<&RunView> for RunBody {
    fn from(run: &RunView) -> Self {
        Self {
            id: run.id.to_string(),
            kind: run.kind,
            status: run.status,
            app: run.app.clone(),
            started_at: run.started_at.to_string(),
            ended_at: run.ended_at.map(|at| at.to_string()),
        }
    }
}

/// One change of a run: an issue and what changed on it.
#[derive(Serialize, ToSchema)]
pub(crate) struct RunChangeBody {
    /// The ID of the event, which a page continues after.
    event: String,
    /// When it was made.
    at: String,
    /// The issue.
    issue: IssueRef,
    /// What changed, leaving out the fields the reader may not see.
    changes: Vec<FieldChangeBody>,
}

impl Masked for RunChangeBody {}

impl From<&RunChangeView> for RunChangeBody {
    fn from(change: &RunChangeView) -> Self {
        Self {
            event: change.event.to_string(),
            at: change.at.to_string(),
            issue: IssueRef::from(&change.issue),
            changes: change.changes.iter().map(FieldChangeBody::from).collect(),
        }
    }
}

/// A field that *Undo the run* leaves as it is.
#[derive(Serialize, ToSchema)]
pub(crate) struct UndoSkipBody {
    /// The issue, when the reader may see it.
    #[serde(skip_serializing_if = "Option::is_none")]
    issue: Option<IssueRef>,
    /// The field.
    field: String,
    /// Why it stays.
    kind: UndoSkipKind,
    /// Somebody changed it afterwards, and it was the reader.
    by_you: bool,
    /// When it was changed afterwards.
    #[serde(skip_serializing_if = "Option::is_none")]
    at: Option<String>,
}

impl From<&UndoSkipView> for UndoSkipBody {
    fn from(skip: &UndoSkipView) -> Self {
        Self {
            issue: skip.issue.as_ref().map(IssueRef::from),
            field: skip.field.clone(),
            kind: skip.kind,
            by_you: skip.by_you,
            at: skip.at.map(|at| at.to_string()),
        }
    }
}

/// What *Undo the run* would do, or did.
#[derive(Serialize, ToSchema)]
pub(crate) struct UndoBody {
    /// The version to apply with. An apply with an older version writes nothing and answers with
    /// the newer preview.
    preview_version: String,
    /// How many field changes go back (or went back).
    goes_back: u64,
    /// The first fields that stay, with why.
    skipped: Vec<UndoSkipBody>,
    /// How many fields stay.
    skipped_count: u64,
    /// For an apply: the preview was out of date, nothing was written and this is the newer one.
    changed_since_preview: bool,
}

impl Masked for UndoBody {}

impl From<&UndoView> for UndoBody {
    fn from(view: &UndoView) -> Self {
        Self {
            preview_version: view.version.0.clone(),
            goes_back: view.goes_back,
            skipped: view.skips.iter().map(UndoSkipBody::from).collect(),
            skipped_count: view.skipped,
            changed_since_preview: view.changed_since_preview,
        }
    }
}

const RUN_CHANGES_LIST: &str = "run-changes";

/// The fields of a change of a run that can be asked for.
const RUN_CHANGE_FIELDS: [FieldSpec; 4] = [
    FieldSpec::by_default("event"),
    FieldSpec::by_default("at"),
    FieldSpec::by_default("issue"),
    FieldSpec::by_default("changes"),
];

/// What *Undo the run* is applied with.
#[derive(Deserialize, ToSchema)]
pub(crate) struct UndoRunBody {
    /// The version of the preview the person looked at.
    preview_version: String,
}

/// Shows a run: a session of an AI app, an attempt of an agent or a change to many issues. Only the
/// person it belongs to reads it.
#[utoipa::path(
    get,
    path = "/api/v1/runs/{run}",
    operation_id = "getRun",
    tag = "runs",
    params(("run" = String, Path, description = "The ID of the run.")),
    responses((status = 200, description = "The run.", body = RunBody), NotSignedInAnswer, NotFoundAnswer)
)]
async fn get_run(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(run): ApiPath<String>,
) -> Result<Response, ApiError> {
    let found = auth
        .api()?
        .get_run(
            info,
            RunQuery {
                session_token,
                run: path_id::<RunId>(&run)?,
            },
        )
        .await?;
    answer_authorized(&found.map(|run| RunBody::from(&run)))
}

/// The changes of a run, oldest first, a page at a time: the issues the reader may see, and on them
/// the fields they may see.
#[utoipa::path(
    get,
    path = "/api/v1/runs/{run}/changes",
    operation_id = "listRunChanges",
    tag = "runs",
    params(("run" = String, Path, description = "The ID of the run."), ListParams),
    responses((status = 200, description = "The changes.", body = Page<RunChangeBody>), InvalidAnswer, NotSignedInAnswer, NotFoundAnswer)
)]
async fn list_run_changes(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(run): ApiPath<String>,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    let (position, bad_cursor) = params.start(RUN_CHANGES_LIST);
    let after = position
        .as_deref()
        .and_then(|event| body_id::<EventId>(event).ok());
    let page = auth
        .api()?
        .run_changes(
            info,
            RunChangesQuery {
                session_token,
                run: path_id::<RunId>(&run)?,
                after,
                limit: Some(u64::from(params.limit())),
            },
        )
        .await?;
    bad_cursor?;
    let list = page.map(|page| Listed {
        items: page.items.iter().map(RunChangeBody::from).collect(),
        next: page.next.map(|event| event.to_string()),
    });
    let fields = params.fields_readable(&RUN_CHANGE_FIELDS, &list.hidden_properties())?;
    Ok(answer(Page::of_authorized(
        RUN_CHANGES_LIST,
        &list,
        &fields,
    )?))
}

/// What undoing the run would do now: how many field changes go back and which stay, with the
/// version to apply with.
#[utoipa::path(
    get,
    path = "/api/v1/runs/{run}/undo",
    operation_id = "previewUndoRun",
    tag = "runs",
    params(("run" = String, Path, description = "The ID of the run.")),
    responses((status = 200, description = "What undoing would do.", body = UndoBody), NotSignedInAnswer, NotFoundAnswer)
)]
async fn preview_undo_run(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(run): ApiPath<String>,
) -> Result<Response, ApiError> {
    let preview = auth
        .api()?
        .preview_undo_run(
            info,
            RunQuery {
                session_token,
                run: path_id::<RunId>(&run)?,
            },
        )
        .await?;
    answer_authorized(&preview.map(|view| UndoBody::from(&view)))
}

/// Undoes the run in one step, as the person who undoes it: every change is judged again, and a
/// field somebody changed afterwards stays and is named. History is never deleted; new events put
/// the old values back. With a version that is no longer the current one nothing is written and
/// the newer preview comes back.
#[utoipa::path(
    post,
    path = "/api/v1/runs/{run}/undo",
    operation_id = "undoRun",
    tag = "runs",
    params(("run" = String, Path, description = "The ID of the run.")),
    request_body = UndoRunBody,
    responses((status = 200, description = "What was undone, or the newer preview.", body = UndoBody), InvalidAnswer, NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer, ConflictAnswer)
)]
async fn undo_run(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(run): ApiPath<String>,
    ApiJson(body): ApiJson<UndoRunBody>,
) -> Result<Response, ApiError> {
    let done = auth
        .api()?
        .undo_run(
            info,
            UndoRun {
                session_token,
                run: path_id::<RunId>(&run)?,
                version: UndoVersion(body.preview_version),
            },
        )
        .await?;
    answer_authorized(&done.map(|view| UndoBody::from(&view)))
}

/// The routes of runs.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(get_run))
        .routes(routes!(list_run_changes))
        .routes(routes!(preview_undo_run, undo_run))
}
