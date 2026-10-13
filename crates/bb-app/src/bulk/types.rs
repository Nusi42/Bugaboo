//! What the use cases of changing many issues at once take and give back (ISS-09). Inputs that
//! change something are named after what they do; inputs that only read end in `Query`
//! (ARCH-EVENTS). Everything that is given back is wrapped in `Authorized<T>` by the use case.
//!
//! The preview and the change take the same action. The preview says, for each issue, what will
//! change or why it is left alone, and the version of the issue it read; the change takes those
//! versions back, so an issue that changed in between is named and left alone.

use std::collections::BTreeMap;

use bb_core::bulk::{BulkState, SkipReason};
use bb_core::id::{BulkRunId, LabelId, RelationKindId, UserId};
use bb_core::issue::{Priority, Status};
use bb_core::relation::Direction;
use bb_policy::Masked;
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::history::FieldChangeView;
use crate::preview::PreviewSummary;
use crate::relations::IssueLine;

/// Fields to change on every issue: what is `Some` is changed. The same fields as changing one
/// issue, except those that only make sense for one (the title, the issue type).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BulkChange {
    /// A new status.
    pub status: Option<Status>,
    /// A new priority.
    pub priority: Option<Priority>,
    /// The people the issues are assigned to from now on (all of them, not the ones to add).
    pub assigned_to: Option<Vec<UserId>>,
    /// The values of own fields to set, by the key of the field; `null` takes a value away.
    pub fields: BTreeMap<String, Value>,
}

/// What is done to every issue of the selection.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BulkAction {
    /// Change fields (*Change fields*, *Change status*).
    Change {
        /// What to change.
        change: BulkChange,
    },
    /// Move the issues into a project.
    Move {
        /// The project, by its ID or its short name (an old one too).
        project: String,
    },
    /// Put a label on the issues.
    AddLabel {
        /// The label.
        label: LabelId,
    },
    /// Relate every issue to one other issue.
    AddRelation {
        /// What the relation means.
        relation_kind: RelationKindId,
        /// Which end the issues of the selection are.
        direction: Direction,
        /// The issue at the other end, by its readable or internal ID.
        other: String,
    },
    /// Put the issues into the trash.
    Delete,
}

impl BulkAction {
    /// What the action is called in the event of a run.
    #[must_use]
    pub const fn word(&self) -> &'static str {
        match self {
            Self::Change { .. } => "change",
            Self::Move { .. } => "move",
            Self::AddLabel { .. } => "add-label",
            Self::AddRelation { .. } => "add-relation",
            Self::Delete => "delete",
        }
    }
}

/// One issue of the selection as the change takes it: the version the preview read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BulkTarget {
    /// The issue, by its readable ID (also an old one) or its internal ID.
    pub issue: String,
    /// The version of the issue the preview read.
    pub version: i64,
}

/// What the change to many issues would do.
pub struct BulkPreviewQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issues, by their readable or internal IDs.
    pub issues: Vec<String>,
    /// What would be done.
    pub action: BulkAction,
}

/// Change many issues, after the preview.
pub struct ApplyBulk {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issues with the versions the preview read.
    pub issues: Vec<BulkTarget>,
    /// What to do.
    pub action: BulkAction,
}

/// Which run of a change to many issues.
pub struct BulkRunQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The run.
    pub run: BulkRunId,
}

/// What a row of the preview says about the issue.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BulkDetail {
    /// The fields that would change, as the history will show them.
    Changes {
        /// The fields.
        fields: Vec<FieldChangeView>,
    },
    /// The relation that would be added, as the issue reads it.
    Relation {
        /// What the issue reads (*blocks*).
        name: String,
        /// The issue at the other end.
        issue: IssueLine,
    },
}

/// One issue of the preview: what will happen to it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BulkRow {
    /// What the person asked for the issue by.
    pub address: String,
    /// The issue, as the person sees it; none for an issue that is not there for them.
    pub issue: Option<IssueLine>,
    /// The version the preview read; the change takes it back.
    pub version: Option<i64>,
    /// Why the issue is left alone; none when it will change.
    pub skipped: Option<SkipReason>,
    /// What will change.
    pub detail: Option<BulkDetail>,
}

/// What the change to many issues would do, per issue, with the sentence on top.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BulkPreview {
    /// How many will change and how many are left alone.
    pub summary: PreviewSummary,
    /// Every issue of the selection, in the order asked.
    pub rows: Vec<BulkRow>,
    /// The change is too big to wait for: it runs in the background and says when it is done.
    pub runs_in_background: bool,
}

impl Masked for BulkPreview {}

/// What happened to one issue of the selection.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BulkOutcome {
    /// What the person asked for the issue by.
    pub address: String,
    /// The issue, as the person sees it; none for an issue that is not there for them.
    pub issue: Option<IssueLine>,
    /// Why the issue was left alone; none when it was changed.
    pub skipped: Option<SkipReason>,
}

/// An issue a run left alone.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BulkSkipped {
    /// The issue, as the person sees it; none for an issue that is not there for them.
    pub issue: Option<IssueLine>,
    /// Why.
    pub reason: SkipReason,
}

/// How far a run in the background is.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BulkRunView {
    /// The run.
    pub id: BulkRunId,
    /// Where it is.
    pub state: BulkState,
    /// How many issues it has.
    pub total: u32,
    /// How many it has looked at.
    pub done: u32,
    /// How many of those it changed.
    pub changed: u32,
    /// The first of the issues it left alone, with why.
    pub skipped: Vec<BulkSkipped>,
    /// How many issues it left alone in all.
    pub skipped_count: u32,
    /// When it was asked for.
    pub created_at: Timestamp,
    /// When it ended.
    pub finished_at: Option<Timestamp>,
}

impl Masked for BulkRunView {}

/// What a change to many issues did: at once, or the run that goes on in the background.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BulkResult {
    /// What happened to each issue, when it was done at once.
    pub outcomes: Vec<BulkOutcome>,
    /// The run, when it goes on in the background.
    pub run: Option<BulkRunView>,
}

impl Masked for BulkResult {}

/// The most issues a run lists as left alone.
pub const RUN_SKIPPED_MAX: usize = 50;
