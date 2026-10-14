//! Judging one issue of a selection (ISS-09): the one evaluation that the preview shows and the
//! change acts on.
//!
//! Each issue is judged on its own, by the same functions the single changes use (the permission,
//! the field levels, the rules of the change, the rules of a move), so what the preview says about
//! an issue is what the change then does to it. An issue that cannot be changed is skipped with a
//! reason a person can read; an issue that changed after the preview is named and left alone.

use bb_core::ErrorCode;
use bb_core::bulk::SkipReason;
use bb_core::event::FieldChange;
use bb_core::id::{IssueId, LabelId};
use bb_core::issue::Issue;
use bb_policy::Evaluation;
use serde_json::json;
use std::sync::Arc;

use super::types::{BulkAction, BulkChange};
use crate::access::Actor;
use crate::auth::AppError;
use crate::auth::{AuthStore, Ctx};
use crate::issues::{ChangeIssue, member};
use crate::relations::{AddRelation, IssueLine, LineMaker};

/// What one issue of the selection is asked.
pub(super) struct Probe<'a> {
    /// What the person asked for the issue by.
    pub(super) address: &'a str,
    /// The version the preview read; none while the preview itself is made.
    pub(super) version: Option<i64>,
}

/// The fields of an issue that a change would change.
pub(super) struct FieldsPlan {
    pub(super) issue: Issue,
    pub(super) held: Arc<Evaluation>,
    pub(super) changes: Vec<FieldChange>,
}

/// What a change to an issue would do, before it is shown or made.
pub(super) enum Planned {
    /// Fields of the issue change, as this person holds them in the project of the issue.
    Fields(Box<FieldsPlan>),
    /// A relation to another issue is added.
    Relation { name: String, other: Box<Issue> },
    /// The issue is put into the trash.
    Trash,
}

/// What the judgment of one issue says.
pub(super) enum Judged {
    /// The change would be made.
    Change(Planned),
    /// The issue is left alone.
    Skip(SkipReason),
}

/// An issue judged: as the person sees it, the version read, and what will happen.
pub(super) struct Judgment {
    pub(super) issue: Option<IssueLine>,
    pub(super) version: Option<i64>,
    pub(super) judged: Judged,
}

impl Judgment {
    fn skipped(issue: Option<IssueLine>, version: Option<i64>, reason: SkipReason) -> Self {
        Self {
            issue,
            version,
            judged: Judged::Skip(reason),
        }
    }
}

/// What the whole selection shares: who asks, with what session, and what is done.
pub(super) struct Selection<'a> {
    pub(super) actor: &'a Actor,
    /// The secret of the session of the request (for the use cases that are run for each issue).
    pub(super) token: &'a str,
    pub(super) action: &'a BulkAction,
    /// The issue at the other end of a relation, found once.
    pub(super) other: Option<IssueId>,
}

/// The reason an issue is left alone for the error a single change gives, or the error itself when
/// it is no reason (the store broke): that stops the whole change.
///
/// # Errors
/// The error, when it is no reason to skip one issue.
pub(super) fn skip_of(error: AppError) -> Result<SkipReason, AppError> {
    match error {
        AppError::Stale(_) | AppError::Refused(ErrorCode::VersionChanged) => {
            Ok(SkipReason::ChangedSince)
        }
        AppError::Refused(code) => Ok(match code {
            ErrorCode::NotFound => SkipReason::NotFound,
            ErrorCode::Forbidden => SkipReason::NotAllowed,
            ErrorCode::IssueTypeRetired => SkipReason::TypeMissing,
            ErrorCode::FieldNotHere => SkipReason::FieldMissing,
            ErrorCode::TagNotHere => SkipReason::TagNotHere,
            ErrorCode::RelationExists => SkipReason::RelationExists,
            ErrorCode::LimitReached => SkipReason::LimitReached,
            _ => SkipReason::NotValid,
        }),
        other => Err(other),
    }
}

/// A judged change turned into a reason to skip the issue, when the single change would have
/// refused it.
///
/// # Errors
/// The error, when it is no reason to skip one issue.
fn or_skip(result: Result<Judged, AppError>) -> Result<Judged, AppError> {
    match result {
        Err(error) => Ok(Judged::Skip(skip_of(error)?)),
        judged => judged,
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Judges one issue of the selection. `maker` makes the line of the issue.
    pub(super) async fn judge_item(
        &self,
        selection: &Selection<'_>,
        probe: &Probe<'_>,
        maker: &mut LineMaker,
    ) -> Result<Judgment, AppError> {
        let actor = selection.actor;
        let (issue, held) = match self.visible_issue(actor, probe.address).await {
            Ok(found) => found,
            Err(AppError::Refused(ErrorCode::NotFound)) => {
                return Ok(Judgment::skipped(None, None, SkipReason::NotFound));
            }
            Err(other) => return Err(other),
        };
        let line = self.line_of(actor, maker, &issue).await?;
        let version = Some(issue.version);
        if probe.version.is_some_and(|read| read != issue.version) {
            return Ok(Judgment::skipped(line, version, SkipReason::ChangedSince));
        }
        let judged = match selection.action {
            BulkAction::Change { change } => {
                or_skip(self.judge_change(selection, change, &issue).await)?
            }
            BulkAction::Move { project } => {
                or_skip(self.judge_move(actor, project, &issue, held).await)?
            }
            BulkAction::AddLabel { label } => {
                or_skip(self.judge_label(actor, *label, &issue, held).await)?
            }
            BulkAction::AddRelation { .. } => {
                or_skip(self.judge_relation(selection, &issue).await)?
            }
            BulkAction::Delete => or_skip(
                self.issue_to_delete(actor, &issue.id.to_string())
                    .await
                    .map(|_| Judged::Change(Planned::Trash)),
            )?,
        };
        Ok(Judgment {
            issue: line,
            version,
            judged,
        })
    }

    async fn judge_change(
        &self,
        selection: &Selection<'_>,
        change: &BulkChange,
        issue: &Issue,
    ) -> Result<Judged, AppError> {
        let asked = ChangeIssue {
            session_token: selection.token.to_owned(),
            issue: issue.id.to_string(),
            version: issue.version,
            issue_type: None,
            title: None,
            status: change.status,
            priority: change.priority,
            assigned_to: change.assigned_to.clone(),
            fields: change.fields.clone(),
        };
        let (held, plan) = self.plan_issue_change(selection.actor, &asked).await?;
        let changes = plan.field_changes();
        Ok(if changes.is_empty() {
            Judged::Skip(SkipReason::NothingToChange)
        } else {
            Judged::Change(Planned::Fields(Box::new(FieldsPlan {
                issue: plan.current,
                held,
                changes,
            })))
        })
    }

    async fn judge_move(
        &self,
        actor: &Actor,
        project: &str,
        issue: &Issue,
        held: Arc<Evaluation>,
    ) -> Result<Judged, AppError> {
        let plan = self
            .plan_move(actor, &issue.id.to_string(), project)
            .await?;
        if plan.already_there {
            return Ok(Judged::Skip(SkipReason::NothingToChange));
        }
        Ok(Judged::Change(Planned::Fields(Box::new(FieldsPlan {
            changes: vec![FieldChange {
                field: member::PROJECT.to_owned(),
                before: Some(json!(plan.issue.project.to_string())),
                after: Some(json!(plan.target_id().to_string())),
            }],
            issue: plan.issue,
            held,
        }))))
    }

    async fn judge_label(
        &self,
        actor: &Actor,
        label: LabelId,
        issue: &Issue,
        held: Arc<Evaluation>,
    ) -> Result<Judged, AppError> {
        let plan = self.plan_label(actor, &issue.id.to_string(), label).await?;
        if plan.is_done_already() {
            return Ok(Judged::Skip(SkipReason::NothingToChange));
        }
        Ok(Judged::Change(Planned::Fields(Box::new(FieldsPlan {
            changes: vec![FieldChange {
                field: member::LABELS.to_owned(),
                before: Some(json!(plan.before)),
                after: Some(json!(plan.after)),
            }],
            issue: plan.issue,
            held,
        }))))
    }

    async fn judge_relation(
        &self,
        selection: &Selection<'_>,
        issue: &Issue,
    ) -> Result<Judged, AppError> {
        let BulkAction::AddRelation {
            relation_kind,
            direction,
            other,
        } = selection.action
        else {
            return Err(AppError::Refused(ErrorCode::InvalidRequest));
        };
        if selection.other == Some(issue.id) {
            return Ok(Judged::Skip(SkipReason::SameIssue));
        }
        let asked = AddRelation {
            session_token: selection.token.to_owned(),
            issue: issue.id.to_string(),
            other: other.clone(),
            kind: *relation_kind,
            direction: *direction,
        };
        let plan = self.plan_relation(selection.actor, &asked).await?;
        let (name, second) = plan.describe();
        Ok(Judged::Change(Planned::Relation {
            name: name.to_owned(),
            other: Box::new(second.clone()),
        }))
    }
}
