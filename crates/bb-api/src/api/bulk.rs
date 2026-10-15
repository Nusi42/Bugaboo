//! The routes of changing many issues at once (ISS-09): the preview below `/api/v1/bulk/preview`,
//! the change below `/api/v1/bulk/changes`, and how far a change that goes on in the background is
//! below `/api/v1/bulk/runs/{run}`.
//!
//! The preview says, for each issue of the selection, what will change or why it is left alone,
//! and the version it read; the change takes those versions back. More than 1,000 issues run in
//! the background and name the issues by their internal IDs. The use cases are in `bb-app`, behind
//! [`AuthApi`](bb_app::auth::AuthApi).

use std::collections::BTreeMap;

use axum::extract::State;
use axum::response::Response;
use bb_app::bulk::{
    ApplyBulk, BulkAction, BulkChange, BulkDetail, BulkOutcome, BulkPreview, BulkPreviewQuery,
    BulkResult, BulkRow, BulkRunQuery, BulkRunView, BulkSkipped, BulkTarget,
};
use bb_app::history::{ChangeValue, FieldChangeView};
use bb_core::ErrorCode;
use bb_core::bulk::{BulkStatus, SkipReason};
use bb_core::id::{BulkRunId, LabelId, RelationKindId, UserId};
use bb_core::issue::{Priority, Status};
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
use crate::errors::{ApiError, ApiJson, ApiPath};

/// What a person asks for in the fields of many issues: what is left out stays as it is.
#[derive(Deserialize, ToSchema)]
pub(crate) struct BulkChangeBody {
    /// A new status.
    status: Option<Status>,
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
pub(crate) enum BulkActionBody {
    /// Change fields (*Change fields*, *Change status*).
    Change {
        /// What to change.
        change: BulkChangeBody,
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

impl BulkActionBody {
    /// The action as the use cases take it.
    fn into_action(self) -> Result<BulkAction, ApiError> {
        let invalid = || ApiError::new(ErrorCode::InvalidRequest);
        Ok(match self {
            Self::Change { change } => BulkAction::Change {
                change: BulkChange {
                    status: change.status,
                    priority: change.priority,
                    assigned_to: change
                        .assigned_to
                        .map(|people| {
                            people
                                .iter()
                                .map(|text| text.parse::<UserId>().map_err(|_| invalid()))
                                .collect::<Result<Vec<_>, _>>()
                        })
                        .transpose()?,
                    fields: change.fields,
                },
            },
            Self::Move { project } => BulkAction::Move { project },
            Self::AddLabel { label } => BulkAction::AddLabel {
                label: label.parse::<LabelId>().map_err(|_| invalid())?,
            },
            Self::AddRelation {
                relation_kind,
                direction,
                other,
            } => BulkAction::AddRelation {
                relation_kind: relation_kind
                    .parse::<RelationKindId>()
                    .map_err(|_| invalid())?,
                direction,
                other,
            },
            Self::Delete => BulkAction::Delete,
        })
    }
}

/// The issues to look at and what would be done to them.
#[derive(Deserialize, ToSchema)]
pub(crate) struct PreviewBulkBody {
    /// The issues, by their readable or internal IDs.
    issues: Vec<String>,
    /// What would be done.
    action: BulkActionBody,
}

/// One issue of the change with the version the preview read.
#[derive(Deserialize, ToSchema)]
pub(crate) struct BulkTargetBody {
    /// The issue. A change to more than 1,000 issues names them by their internal IDs.
    issue: String,
    /// The version of the issue the preview read.
    version: i64,
}

/// The issues to change, with the versions the preview read, and what to do.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ApplyBulkBody {
    /// The issues.
    issues: Vec<BulkTargetBody>,
    /// What to do.
    action: BulkActionBody,
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

/// What a row of the preview says will happen to the issue.
#[derive(Serialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum BulkDetailBody {
    /// The fields that would change.
    Changes {
        /// The fields.
        fields: Vec<FieldChangeBody>,
    },
    /// The relation that would be added.
    Relation {
        /// What the issue reads (*blocks*).
        name: String,
        /// The issue at the other end.
        issue: Box<IssueRef>,
    },
}

impl From<&BulkDetail> for BulkDetailBody {
    fn from(detail: &BulkDetail) -> Self {
        match detail {
            BulkDetail::Changes { fields } => Self::Changes {
                fields: fields.iter().map(FieldChangeBody::from).collect(),
            },
            BulkDetail::Relation { name, issue } => Self::Relation {
                name: name.clone(),
                issue: Box::new(IssueRef::from(issue)),
            },
        }
    }
}

/// One issue of the preview: what will happen to it.
#[derive(Serialize, ToSchema)]
pub(crate) struct BulkRowBody {
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
    detail: Option<BulkDetailBody>,
}

impl From<&BulkRow> for BulkRowBody {
    fn from(row: &BulkRow) -> Self {
        Self {
            address: row.address.clone(),
            issue: row.issue.as_ref().map(IssueRef::from),
            version: row.version,
            skipped: row.skipped,
            detail: row.detail.as_ref().map(BulkDetailBody::from),
        }
    }
}

/// What the change to many issues would do, with the sentence on top.
#[derive(Serialize, ToSchema)]
pub(crate) struct BulkPreviewBody {
    /// How many will change and how many are left alone.
    summary: PreviewSummaryBody,
    /// Every issue of the selection, in the order asked.
    rows: Vec<BulkRowBody>,
    /// The change is too big to wait for: it runs in the background and says when it is done.
    runs_in_background: bool,
}

impl Masked for BulkPreviewBody {}

impl From<BulkPreview> for BulkPreviewBody {
    fn from(preview: BulkPreview) -> Self {
        Self {
            summary: PreviewSummaryBody::from(preview.summary),
            rows: preview.rows.iter().map(BulkRowBody::from).collect(),
            runs_in_background: preview.runs_in_background,
        }
    }
}

/// What happened to one issue of the selection.
#[derive(Serialize, ToSchema)]
pub(crate) struct BulkOutcomeBody {
    /// What the person asked for the issue by.
    address: String,
    /// The issue, as the person sees it; left out for an issue that is not there for them.
    #[serde(skip_serializing_if = "Option::is_none")]
    issue: Option<IssueRef>,
    /// Why the issue was left alone; left out when it was changed.
    #[serde(skip_serializing_if = "Option::is_none")]
    skipped: Option<SkipReason>,
}

impl From<&BulkOutcome> for BulkOutcomeBody {
    fn from(outcome: &BulkOutcome) -> Self {
        Self {
            address: outcome.address.clone(),
            issue: outcome.issue.as_ref().map(IssueRef::from),
            skipped: outcome.skipped,
        }
    }
}

/// An issue a run left alone.
#[derive(Serialize, ToSchema)]
pub(crate) struct BulkSkippedBody {
    /// The issue, as the person sees it; left out for an issue that is not there for them.
    #[serde(skip_serializing_if = "Option::is_none")]
    issue: Option<IssueRef>,
    /// Why.
    reason: SkipReason,
}

impl From<&BulkSkipped> for BulkSkippedBody {
    fn from(skipped: &BulkSkipped) -> Self {
        Self {
            issue: skipped.issue.as_ref().map(IssueRef::from),
            reason: skipped.reason,
        }
    }
}

/// How far a change that goes on in the background is.
#[derive(Serialize, ToSchema)]
pub(crate) struct BulkRunBody {
    /// The ID of the run.
    id: String,
    /// Where it is.
    status: BulkStatus,
    /// How many issues it has.
    total: u32,
    /// How many it has looked at.
    done: u32,
    /// How many of those it changed.
    changed: u32,
    /// The first of the issues it left alone, with why.
    skipped: Vec<BulkSkippedBody>,
    /// How many issues it left alone in all.
    skipped_count: u32,
    /// When it was asked for.
    created_at: String,
    /// When it ended; left out while it goes on.
    #[serde(skip_serializing_if = "Option::is_none")]
    finished_at: Option<String>,
}

impl Masked for BulkRunBody {}

impl From<&BulkRunView> for BulkRunBody {
    fn from(run: &BulkRunView) -> Self {
        Self {
            id: run.id.to_string(),
            status: run.status,
            total: run.total,
            done: run.done,
            changed: run.changed,
            skipped: run.skipped.iter().map(BulkSkippedBody::from).collect(),
            skipped_count: run.skipped_count,
            created_at: run.created_at.to_string(),
            finished_at: run.finished_at.map(|at| at.to_string()),
        }
    }
}

/// What a change to many issues did: at once, or the run that goes on in the background.
#[derive(Serialize, ToSchema)]
pub(crate) struct BulkResultBody {
    /// What happened to each issue, when it was done at once.
    outcomes: Vec<BulkOutcomeBody>,
    /// The run, when it goes on in the background.
    #[serde(skip_serializing_if = "Option::is_none")]
    run: Option<BulkRunBody>,
}

impl Masked for BulkResultBody {}

impl From<BulkResult> for BulkResultBody {
    fn from(result: BulkResult) -> Self {
        Self {
            outcomes: result.outcomes.iter().map(BulkOutcomeBody::from).collect(),
            run: result.run.as_ref().map(BulkRunBody::from),
        }
    }
}

/// Says what a change to many issues would do to each issue: what will change, what is left alone
/// and why, and the version of each issue, which the change takes back. Nothing is changed.
#[utoipa::path(
    post,
    path = "/api/v1/bulk/preview",
    operation_id = "previewBulkChange",
    tag = "bulk",
    request_body = PreviewBulkBody,
    responses(
        (status = 200, description = "What would happen to each issue.", body = BulkPreviewBody),
        InvalidAnswer,
        NotSignedInAnswer,
        NotFoundAnswer
    )
)]
async fn preview_bulk_change(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<PreviewBulkBody>,
) -> Result<Response, ApiError> {
    let preview = auth
        .api()?
        .preview_bulk(
            info,
            BulkPreviewQuery {
                session_token,
                issues: body.issues,
                action: body.action.into_action()?,
            },
        )
        .await?;
    answer_authorized(&preview.map(BulkPreviewBody::from))
}

/// Changes many issues at once, each on the version the preview read. Each issue is judged again,
/// and one that changed since the preview is named and left alone. More than 1,000 issues run in
/// the background: the answer holds the run, and `getBulkRun` says how far it is. A script with an
/// access key sends at most 1,000 issues at a time.
#[utoipa::path(
    post,
    path = "/api/v1/bulk/changes",
    operation_id = "applyBulkChange",
    tag = "bulk",
    request_body = ApplyBulkBody,
    responses(
        (status = 200, description = "What happened to each issue, or the run.", body = BulkResultBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn apply_bulk_change(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<ApplyBulkBody>,
) -> Result<Response, ApiError> {
    let result = auth
        .api()?
        .apply_bulk(
            info,
            ApplyBulk {
                session_token,
                issues: body
                    .issues
                    .into_iter()
                    .map(|target| BulkTarget {
                        issue: target.issue,
                        version: target.version,
                    })
                    .collect(),
                action: body.action.into_action()?,
            },
        )
        .await?;
    answer_authorized(&result.map(BulkResultBody::from))
}

/// How far a change to many issues that goes on in the background is. Only the person who asked
/// for it reads it.
#[utoipa::path(
    get,
    path = "/api/v1/bulk/runs/{run}",
    operation_id = "getBulkRun",
    tag = "bulk",
    params(("run" = String, Path, description = "The ID of the run.")),
    responses((status = 200, description = "How far the run is.", body = BulkRunBody), NotSignedInAnswer, NotFoundAnswer)
)]
async fn get_bulk_run(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(run): ApiPath<String>,
) -> Result<Response, ApiError> {
    let run: BulkRunId = run
        .parse()
        .map_err(|_| ApiError::new(ErrorCode::NotFound))?;
    let found = auth
        .api()?
        .bulk_run(info, BulkRunQuery { session_token, run })
        .await?;
    answer_authorized(&found.map(|run| BulkRunBody::from(&run)))
}

/// Stops a change to many issues that goes on in the background: what was changed so far stays and
/// the rest is left alone. The person who asked for it may stop it, and so may an administrator.
#[utoipa::path(
    post,
    path = "/api/v1/bulk/runs/{run}/cancel",
    operation_id = "cancelBulkRun",
    tag = "bulk",
    params(("run" = String, Path, description = "The ID of the run.")),
    responses((status = 200, description = "How far the run was when it stopped.", body = BulkRunBody), NotSignedInAnswer, NotFoundAnswer)
)]
async fn cancel_bulk_run(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(run): ApiPath<String>,
) -> Result<Response, ApiError> {
    let run: BulkRunId = run
        .parse()
        .map_err(|_| ApiError::new(ErrorCode::NotFound))?;
    let found = auth
        .api()?
        .cancel_bulk_run(info, BulkRunQuery { session_token, run })
        .await?;
    answer_authorized(&found.map(|run| BulkRunBody::from(&run)))
}

/// The routes of changing many issues at once.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(preview_bulk_change))
        .routes(routes!(apply_bulk_change))
        .routes(routes!(get_bulk_run))
        .routes(routes!(cancel_bulk_run))
}
