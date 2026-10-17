//! The routes of changing many issues at once (ISS-09): the preview below `/api/v1/many-changes/preview`,
//! the change below `/api/v1/many-changes`, and how far a change that goes on in the background is
//! below `/api/v1/many-changes/runs/{run}`.
//!
//! The preview says, for each issue of the selection, what will change or why it is left alone,
//! and the version it read; the change takes those versions back. More than 1,000 issues run in
//! the background and name the issues by their internal IDs. The use cases are in `bb-app`, behind
//! [`AuthApi`](bb_app::auth::AuthApi).

use std::collections::BTreeMap;

use axum::extract::State;
use axum::response::Response;
use bb_app::history::{ChangeValue, FieldChangeView};
use bb_app::many_changes::{
    ApplyManyChanges, ManyChange, ManyChangeAction, ManyChangeDetail, ManyChangeNote,
    ManyChangeOutcome, ManyChangePreview, ManyChangePreviewQuery, ManyChangeResult, ManyChangeRow,
    ManyChangeRunQuery, ManyChangeRunStatus, ManyChangeRunView, ManyChangeSkipped,
    ManyChangeTarget,
};
use bb_core::id::{LabelId, ManyChangeRunId, RelationKindId, UserId};
use bb_core::issue::Priority;
use bb_core::many_changes::{ManyChangeStatus, SkipReason};
use bb_core::own_fields::FieldType;
use bb_core::relation::Direction;
use bb_policy::{Masked, PersonRef};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer_authorized,
};
use super::previews::PreviewSummaryBody;
use super::projects::ProjectRef;
use super::relations::IssueRef;
use super::workflows::status_in;
use super::{body_id, path_id};
use crate::errors::{ApiError, ApiJson, ApiPath};

/// What a person asks for in the fields of many issues: what is left out stays as it is.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ManyChangeBody {
    /// The key of a new status. Each issue is judged by its own workflow, and an issue whose
    /// workflow does not allow the change is skipped.
    status: Option<String>,
    /// A new priority.
    priority: Option<Priority>,
    /// The IDs of the people the issues are assigned to from now on: all of them, not the ones to
    /// add.
    assigned_to: Option<Vec<String>>,
    /// The values of own fields to set, by the key (ID) of the field; `null` takes a value away.
    #[serde(default)]
    fields: BTreeMap<String, Value>,
}

/// What is done to every issue of the selection.
#[derive(Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum ManyChangeActionBody {
    /// Change fields (*Change fields*, *Change status*).
    Change {
        /// What to change.
        change: ManyChangeBody,
    },
    /// Move the issues into a project.
    Move {
        /// The project, by its ID or its short name.
        project: String,
    },
    /// Put a label on the issues.
    AddLabel {
        /// The ID of the label.
        label: String,
    },
    /// Relate every issue to one other issue.
    AddRelation {
        /// The ID of the kind of relation.
        relation_kind: String,
        /// Which end the issues of the selection are.
        direction: Direction,
        /// The issue at the other end, by its readable or internal ID.
        other: String,
    },
    /// Put the issues into the trash.
    Delete,
}

impl ManyChangeActionBody {
    /// The action as the use cases take it.
    fn into_action(self) -> Result<ManyChangeAction, ApiError> {
        Ok(match self {
            Self::Change { change } => ManyChangeAction::Change {
                change: ManyChange {
                    status: status_in(change.status.as_deref())?,
                    priority: change.priority,
                    assigned_to: change
                        .assigned_to
                        .map(|people| {
                            people
                                .iter()
                                .map(|text| body_id::<UserId>(text))
                                .collect::<Result<Vec<_>, _>>()
                        })
                        .transpose()?,
                    fields: change.fields,
                },
            },
            Self::Move { project } => ManyChangeAction::Move { project },
            Self::AddLabel { label } => ManyChangeAction::AddLabel {
                label: body_id::<LabelId>(&label)?,
            },
            Self::AddRelation {
                relation_kind,
                direction,
                other,
            } => ManyChangeAction::AddRelation {
                relation_kind: body_id::<RelationKindId>(&relation_kind)?,
                direction,
                other,
            },
            Self::Delete => ManyChangeAction::Delete,
        })
    }
}

/// The issues to look at and what would be done to them.
#[derive(Deserialize, ToSchema)]
pub(crate) struct PreviewManyChangesBody {
    /// The issues, by their readable or internal IDs.
    issues: Vec<String>,
    /// What would be done.
    action: ManyChangeActionBody,
}

/// One issue of the change with the version the preview read.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ManyChangeTargetBody {
    /// The issue. A change to more than 1,000 issues names them by their internal IDs.
    issue: String,
    /// The version of the issue the preview read.
    version: i64,
}

/// The issues to change, with the versions the preview read, and what to do.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ApplyManyChangesBody {
    /// The issues.
    issues: Vec<ManyChangeTargetBody>,
    /// What to do.
    action: ManyChangeActionBody,
}

/// A value in a change, as the person may see it.
#[derive(Serialize, ToSchema)]
#[serde(tag = "shown", rename_all = "snake_case")]
pub(crate) enum ChangeValueBody {
    /// A word, a name or a number.
    Text {
        /// The text.
        text: String,
    },
    /// The names of the choices of a pick field, or of tags and labels.
    Names {
        /// The names, in order.
        names: Vec<String>,
    },
    /// People, as the kind of the field says.
    People {
        /// The people.
        people: Vec<PersonRef>,
    },
    /// A value that a screen words itself: yes or no, a length of time, a number, a day.
    Typed {
        /// The kind of the field.
        kind: FieldType,
        /// The value.
        value: Value,
    },
    /// An issue the person may see.
    Issue {
        /// The issue.
        issue: IssueRef,
    },
    /// A project the person may see.
    Project {
        /// The project.
        project: ProjectRef,
    },
}

impl From<&ChangeValue> for ChangeValueBody {
    fn from(value: &ChangeValue) -> Self {
        match value {
            ChangeValue::Text { text } => Self::Text { text: text.clone() },
            ChangeValue::Names { names } => Self::Names {
                names: names.clone(),
            },
            ChangeValue::People { people } => Self::People {
                people: people.clone(),
            },
            ChangeValue::Typed { kind, value } => Self::Typed {
                kind: *kind,
                value: value.clone(),
            },
            ChangeValue::Issue { issue } => Self::Issue {
                issue: IssueRef::from(issue),
            },
            ChangeValue::Project { project } => Self::Project {
                project: ProjectRef::from(project),
            },
        }
    }
}

/// One field that would change, with the values the person may see.
#[derive(Serialize, ToSchema)]
pub(crate) struct FieldChangeBody {
    /// The key of the field.
    field: String,
    /// The name of an own field.
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    /// What it is.
    #[serde(skip_serializing_if = "Option::is_none")]
    before: Option<ChangeValueBody>,
    /// What it will be.
    #[serde(skip_serializing_if = "Option::is_none")]
    after: Option<ChangeValueBody>,
}

impl From<&FieldChangeView> for FieldChangeBody {
    fn from(change: &FieldChangeView) -> Self {
        Self {
            field: change.field.clone(),
            name: change.name.clone(),
            before: change.before.as_ref().map(ChangeValueBody::from),
            after: change.after.as_ref().map(ChangeValueBody::from),
        }
    }
}

/// Something a person should know about a change that will be made, beside what changes.
#[derive(Serialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum ManyChangeNoteBody {
    /// The issue moves to a project that does not use its issue type. It keeps the type.
    TypeNotUsed {
        /// The name of the issue type.
        issue_type: String,
        /// The name of the project the issue moves to.
        project: String,
    },
}

impl From<&ManyChangeNote> for ManyChangeNoteBody {
    fn from(note: &ManyChangeNote) -> Self {
        match note {
            ManyChangeNote::TypeNotUsed {
                issue_type,
                project,
            } => Self::TypeNotUsed {
                issue_type: issue_type.clone(),
                project: project.clone(),
            },
        }
    }
}

/// What a row of the preview says will happen to the issue.
#[derive(Serialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum ManyChangeDetailBody {
    /// The fields that would change.
    Changes {
        /// The fields.
        fields: Vec<FieldChangeBody>,
        /// What else the person should know about this change.
        notes: Vec<ManyChangeNoteBody>,
    },
    /// The relation that would be added.
    Relation {
        /// What the issue reads (*blocks*).
        name: String,
        /// The issue at the other end.
        issue: Box<IssueRef>,
    },
}

impl From<&ManyChangeDetail> for ManyChangeDetailBody {
    fn from(detail: &ManyChangeDetail) -> Self {
        match detail {
            ManyChangeDetail::Changes { fields, notes } => Self::Changes {
                fields: fields.iter().map(FieldChangeBody::from).collect(),
                notes: notes.iter().map(ManyChangeNoteBody::from).collect(),
            },
            ManyChangeDetail::Relation { name, issue } => Self::Relation {
                name: name.clone(),
                issue: Box::new(IssueRef::from(&**issue)),
            },
        }
    }
}

/// One issue of the preview: what will happen to it.
#[derive(Serialize, ToSchema)]
pub(crate) struct ManyChangeRowBody {
    /// What the person asked for the issue by.
    address: String,
    /// The issue, as the person sees it; left out for an issue that is not there for them.
    #[serde(skip_serializing_if = "Option::is_none")]
    issue: Option<IssueRef>,
    /// The version the preview read; the change takes it back.
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<i64>,
    /// Why the issue is left alone; left out when it will change.
    #[serde(skip_serializing_if = "Option::is_none")]
    skipped: Option<SkipReason>,
    /// What will change.
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<ManyChangeDetailBody>,
}

impl From<&ManyChangeRow> for ManyChangeRowBody {
    fn from(row: &ManyChangeRow) -> Self {
        Self {
            address: row.address.clone(),
            issue: row.issue.as_ref().map(IssueRef::from),
            version: row.version,
            skipped: row.skipped,
            detail: row.detail.as_ref().map(ManyChangeDetailBody::from),
        }
    }
}

/// What the change to many issues would do, with the sentence on top.
#[derive(Serialize, ToSchema)]
pub(crate) struct ManyChangePreviewBody {
    /// How many will change and how many are left alone.
    summary: PreviewSummaryBody,
    /// Every issue of the selection, in the order asked.
    rows: Vec<ManyChangeRowBody>,
    /// The change is too big to wait for: it runs in the background and says when it is done.
    runs_in_background: bool,
}

impl Masked for ManyChangePreviewBody {}

impl From<ManyChangePreview> for ManyChangePreviewBody {
    fn from(preview: ManyChangePreview) -> Self {
        Self {
            summary: PreviewSummaryBody::from(preview.summary),
            rows: preview.rows.iter().map(ManyChangeRowBody::from).collect(),
            runs_in_background: preview.runs_in_background,
        }
    }
}

/// What happened to one issue of the selection.
#[derive(Serialize, ToSchema)]
pub(crate) struct ManyChangeOutcomeBody {
    /// What the person asked for the issue by.
    address: String,
    /// The issue, as the person sees it; left out for an issue that is not there for them.
    #[serde(skip_serializing_if = "Option::is_none")]
    issue: Option<IssueRef>,
    /// Why the issue was left alone; left out when it was changed.
    #[serde(skip_serializing_if = "Option::is_none")]
    skipped: Option<SkipReason>,
}

impl From<&ManyChangeOutcome> for ManyChangeOutcomeBody {
    fn from(outcome: &ManyChangeOutcome) -> Self {
        Self {
            address: outcome.address.clone(),
            issue: outcome.issue.as_ref().map(IssueRef::from),
            skipped: outcome.skipped,
        }
    }
}

/// An issue a run left alone.
#[derive(Serialize, ToSchema)]
pub(crate) struct ManyChangeSkippedBody {
    /// The issue, as the person sees it; left out for an issue that is not there for them.
    #[serde(skip_serializing_if = "Option::is_none")]
    issue: Option<IssueRef>,
    /// Why.
    reason: SkipReason,
}

impl From<&ManyChangeSkipped> for ManyChangeSkippedBody {
    fn from(skipped: &ManyChangeSkipped) -> Self {
        Self {
            issue: skipped.issue.as_ref().map(IssueRef::from),
            reason: skipped.reason,
        }
    }
}

/// Where a change that goes on in the background is, and nothing more.
#[derive(Serialize, ToSchema)]
pub(crate) struct ManyChangeRunStatusBody {
    /// The ID of the run.
    id: String,
    /// Where it is.
    status: ManyChangeStatus,
}

impl Masked for ManyChangeRunStatusBody {}

impl From<&ManyChangeRunStatus> for ManyChangeRunStatusBody {
    fn from(run: &ManyChangeRunStatus) -> Self {
        Self {
            id: run.id.to_string(),
            status: run.status,
        }
    }
}

/// How far a change that goes on in the background is.
#[derive(Serialize, ToSchema)]
pub(crate) struct ManyChangeRunBody {
    /// The ID of the run.
    id: String,
    /// Where it is.
    status: ManyChangeStatus,
    /// How many issues it has.
    total: u32,
    /// How many it has looked at.
    done: u32,
    /// How many of those it changed.
    changed: u32,
    /// The first of the issues it left alone, with why.
    skipped: Vec<ManyChangeSkippedBody>,
    /// How many issues it left alone in all.
    skipped_count: u32,
    /// When it was asked for.
    created_at: String,
    /// When it ended; left out while it goes on.
    #[serde(skip_serializing_if = "Option::is_none")]
    finished_at: Option<String>,
}

impl Masked for ManyChangeRunBody {}

impl From<&ManyChangeRunView> for ManyChangeRunBody {
    fn from(run: &ManyChangeRunView) -> Self {
        Self {
            id: run.id.to_string(),
            status: run.status,
            total: run.total,
            done: run.done,
            changed: run.changed,
            skipped: run
                .skipped
                .iter()
                .map(ManyChangeSkippedBody::from)
                .collect(),
            skipped_count: run.skipped_count,
            created_at: run.created_at.to_string(),
            finished_at: run.finished_at.map(|at| at.to_string()),
        }
    }
}

/// What a change to many issues did: at once, or the run that goes on in the background.
#[derive(Serialize, ToSchema)]
pub(crate) struct ManyChangeResultBody {
    /// What happened to each issue, when it was done at once.
    outcomes: Vec<ManyChangeOutcomeBody>,
    /// The run, when it goes on in the background.
    #[serde(skip_serializing_if = "Option::is_none")]
    run: Option<ManyChangeRunBody>,
}

impl Masked for ManyChangeResultBody {}

impl From<ManyChangeResult> for ManyChangeResultBody {
    fn from(result: ManyChangeResult) -> Self {
        Self {
            outcomes: result
                .outcomes
                .iter()
                .map(ManyChangeOutcomeBody::from)
                .collect(),
            run: result.run.as_ref().map(ManyChangeRunBody::from),
        }
    }
}

/// Says what a change to many issues would do to each issue: what will change, what is left alone
/// and why, and the version of each issue, which the change takes back. Nothing is changed.
#[utoipa::path(
    post,
    path = "/api/v1/many-changes/preview",
    operation_id = "previewManyChanges",
    tag = "many-changes",
    request_body = PreviewManyChangesBody,
    responses(
        (status = 200, description = "What would happen to each issue.", body = ManyChangePreviewBody),
        InvalidAnswer,
        NotSignedInAnswer,
        NotFoundAnswer
    )
)]
async fn preview_many_changes(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<PreviewManyChangesBody>,
) -> Result<Response, ApiError> {
    let preview = auth
        .api()?
        .preview_many_changes(
            info,
            ManyChangePreviewQuery {
                session_token,
                issues: body.issues,
                action: body.action.into_action()?,
            },
        )
        .await?;
    answer_authorized(&preview.map(ManyChangePreviewBody::from))
}

/// Changes many issues at once, each on the version the preview read. Each issue is judged again,
/// and one that changed since the preview is named and left alone. More than 1,000 issues run in
/// the background: the answer holds the run, and `getManyChangeRun` says how far it is. A script with an
/// access key sends at most 1,000 issues at a time.
#[utoipa::path(
    post,
    path = "/api/v1/many-changes",
    operation_id = "applyManyChanges",
    tag = "many-changes",
    request_body = ApplyManyChangesBody,
    responses(
        (status = 200, description = "What happened to each issue, or the run.", body = ManyChangeResultBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn apply_many_changes(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<ApplyManyChangesBody>,
) -> Result<Response, ApiError> {
    let result = auth
        .api()?
        .apply_many_changes(
            info,
            ApplyManyChanges {
                session_token,
                issues: body
                    .issues
                    .into_iter()
                    .map(|target| ManyChangeTarget {
                        issue: target.issue,
                        version: target.version,
                    })
                    .collect(),
                action: body.action.into_action()?,
            },
        )
        .await?;
    answer_authorized(&result.map(ManyChangeResultBody::from))
}

/// How far a change to many issues that goes on in the background is. Only the person who asked
/// for it reads it.
#[utoipa::path(
    get,
    path = "/api/v1/many-changes/runs/{run}",
    operation_id = "getManyChangeRun",
    tag = "many-changes",
    params(("run" = String, Path, description = "The ID of the run.")),
    responses((status = 200, description = "How far the run is.", body = ManyChangeRunBody), NotSignedInAnswer, NotFoundAnswer)
)]
async fn get_many_changes_run(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(run): ApiPath<String>,
) -> Result<Response, ApiError> {
    let run: ManyChangeRunId = path_id(&run)?;
    let found = auth
        .api()?
        .many_changes_run(info, ManyChangeRunQuery { session_token, run })
        .await?;
    answer_authorized(&found.map(|run| ManyChangeRunBody::from(&run)))
}

/// Stops a change to many issues that goes on in the background: what was changed so far stays and
/// the rest is left alone. The person who asked for it may stop it, and so may whoever holds
/// *Manage people*. The answer holds the status only.
#[utoipa::path(
    post,
    path = "/api/v1/many-changes/runs/{run}/stop",
    operation_id = "stopManyChangeRun",
    tag = "many-changes",
    params(("run" = String, Path, description = "The ID of the run.")),
    responses((status = 200, description = "Where the run is after the stop.", body = ManyChangeRunStatusBody), NotSignedInAnswer, NotFoundAnswer)
)]
async fn stop_many_changes_run(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(run): ApiPath<String>,
) -> Result<Response, ApiError> {
    let run: ManyChangeRunId = path_id(&run)?;
    let found = auth
        .api()?
        .stop_many_changes_run(info, ManyChangeRunQuery { session_token, run })
        .await?;
    answer_authorized(&found.map(|run| ManyChangeRunStatusBody::from(&run)))
}

/// The routes of changing many issues at once.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(preview_many_changes))
        .routes(routes!(apply_many_changes))
        .routes(routes!(get_many_changes_run))
        .routes(routes!(stop_many_changes_run))
}
