//! Changing many issues at once (ISS-09): the limits, why an issue is skipped, and the record of a
//! run that is too big to wait for.
//!
//! Every issue of a selection is judged on its own (the permission, the field levels, the rules of
//! the change), first for the preview and again when the change is made, by the same function.
//! An issue the change cannot touch is skipped with a reason a person can read; an issue that
//! changed after the preview is named and left alone.

use jiff::Timestamp;

use crate::id::{BulkRunId, IssueId, UserId};
use crate::personal::Entity;

/// The most issues one change names, so a preview stays a page and a run stays a run.
pub const BULK_ISSUES_MAX: usize = 5000;

/// A change to more issues than this runs as a background job (ADR-0017), with progress and a note
/// when it is done.
pub const BULK_JOB_ABOVE: usize = 1000;

/// How many issues one step of a background run changes before it hands the rest back to the
/// queue: a worker that stops loses at most one step.
pub const BULK_STEP: usize = 100;

words! {
    /// Why an issue of the selection is left alone.
    pub enum SkipReason {
        /// The issue is not there for this person: it does not exist, is in the trash or is hidden
        /// from them (the three look alike).
        NotFound => "not-found",
        /// The person may not make this change to this issue.
        NotAllowed => "not-allowed",
        /// The issue is as the change wants it already.
        NothingToChange => "nothing-to-change",
        /// The issue changed after the preview.
        ChangedSince => "changed-since",
        /// The project it would move to does not offer the issue's type.
        TypeMissing => "type-missing",
        /// The project it would move to does not use an own field the issue has a value in.
        FieldMissing => "field-missing",
        /// The issue has a tag that is not for the project it would move to.
        TagNotHere => "tag-not-here",
        /// The relation is there already.
        RelationExists => "relation-exists",
        /// An issue cannot be related to itself.
        SameIssue => "same-issue",
        /// The issue holds as many of it as it may.
        LimitReached => "limit-reached",
        /// A value of the issue breaks the rule of its field (it is too long, does not match, is
        /// missing where the field must be filled in).
        FieldRule => "field-rule",
        /// The change cannot be made to this issue, for a reason that has no word of its own.
        NotValid => "not-valid",
    }
    messages name = "bulk-skip."
}

words_in_api!(SkipReason, "Why an issue of the selection is left alone.");

words! {
    /// Where a run of a change to many issues is.
    pub enum BulkStatus {
        /// Waiting for a worker.
        Queued => "queued",
        /// A worker is changing the issues.
        Running => "running",
        /// Every issue was changed or skipped.
        Done => "done",
        /// The run stopped before it was done.
        Failed => "failed",
    }
    messages name = "bulk-status."
}

words_in_api!(BulkStatus, "Where a run of a change to many issues is.");

/// One issue of a selection with the version the person looked at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BulkItem {
    /// The issue.
    pub issue: IssueId,
    /// The version the preview read.
    pub version: i64,
}

impl crate::personal::NotPersonal for BulkItem {}

/// An issue a run left alone, with why.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BulkSkip {
    /// The issue.
    pub issue: IssueId,
    /// Why.
    pub reason: SkipReason,
}

impl crate::personal::NotPersonal for BulkSkip {}

/// A change to many issues that runs in the background.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct BulkRun {
    /// The ID of the run.
    #[not_personal]
    pub id: BulkRunId,
    /// Who asked for it: the changes are made as this person.
    #[personal(category = "activity", basis = "contract")]
    pub person: UserId,
    /// What the change is, as the use case wrote it. It can hold what a person typed (a value).
    #[personal(category = "content", basis = "legitimate_interests")]
    pub action: serde_json::Value,
    /// The issues and the versions the preview read.
    #[not_personal]
    pub items: Vec<BulkItem>,
    /// Where the run is.
    #[not_personal]
    pub status: BulkStatus,
    /// How many issues have been looked at (changed or skipped): the next one is at this place.
    #[not_personal]
    pub done: i64,
    /// How many of them were changed.
    #[not_personal]
    pub changed: i64,
    /// The issues left alone with why.
    #[not_personal]
    pub skipped: Vec<BulkSkip>,
    /// When the person asked.
    #[not_personal]
    pub created_at: Timestamp,
    /// When the run ended.
    #[not_personal]
    pub finished_at: Option<Timestamp>,
}
impl BulkRun {
    /// The reason the event of the change to the issue at `index` carries, so a worker that stops
    /// between the change and counting it can tell, when it goes on, that the change was made
    /// (ADR-0017).
    #[must_use]
    pub fn item_reason(&self, index: i64) -> String {
        format!("bulk-run:{}:{index}", self.id)
    }
}
