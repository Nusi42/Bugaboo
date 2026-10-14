//! Changing many issues at once (ISS-09): the change itself, after the preview.
//!
//! The change takes the issues with the versions the preview read. Each issue is judged again, by
//! the same function the preview used (`judge.rs`), and then changed by the use case of a single
//! change, so each issue has its own change and its own event, and the history of every issue
//! shows what happened to it. An issue that changed after the preview is named and left alone.
//!
//! More than [`BULK_JOB_ABOVE`] issues are not waited for: the change is queued as a background
//! job (ADR-0017) that goes on a step at a time, and a person reads how far it is (`run.rs`).

use bb_core::ErrorCode;
use bb_core::bulk::{BULK_ISSUES_MAX, BULK_JOB_ABOVE, BulkItem, BulkRun, BulkStatus, SkipReason};
use bb_core::event::RecordKind;
use bb_core::id::{BulkRunId, JobId};
use bb_policy::{Authorized, Place};
use serde_json::json;

use super::judge::{Judged, Probe, Selection, skip_of};
use super::run::step_payload;
use super::types::{ApplyBulk, BulkAction, BulkOutcome, BulkResult, BulkTarget};
use crate::auth::{AppError, AuthStore, Ctx, by, entity, set_to};
use crate::issues::{ChangeIssue, DeleteIssue, MoveIssue, change_issue, delete_issue, move_issue};
use crate::jobs::{kind, kind_name};
use crate::relations::{AddRelation, LineMaker, add_relation};
use crate::tags::{AddLabel, add_label};

impl<S: AuthStore> Ctx<'_, S> {
    /// Judges one issue of the selection again and changes it, by the use case of a single change:
    /// what happened to it. `target.version` is the version the preview read.
    pub(super) async fn apply_item(
        &self,
        selection: &Selection<'_>,
        target: &BulkTarget,
        maker: &mut LineMaker,
    ) -> Result<BulkOutcome, AppError> {
        let probe = Probe {
            address: &target.issue,
            version: Some(target.version),
        };
        let judgment = self.judge_item(selection, &probe, maker).await?;
        let outcome = |skipped: Option<SkipReason>| BulkOutcome {
            address: target.issue.clone(),
            issue: judgment.issue.clone(),
            skipped,
        };
        if let Judged::Skip(reason) = judgment.judged {
            return Ok(outcome(Some(reason)));
        }
        // Changed at once by the use case of a single change, which stops for the same reasons the
        // judgment gave and, if the issue changed in the instant between, says so.
        let token = selection.token.to_owned();
        let done = match selection.action {
            BulkAction::Change { change } => change_issue(
                self,
                ChangeIssue {
                    session_token: token,
                    issue: target.issue.clone(),
                    version: target.version,
                    issue_type: None,
                    title: None,
                    status: change.status,
                    priority: change.priority,
                    assigned_to: change.assigned_to.clone(),
                    fields: change.fields.clone(),
                },
            )
            .await
            .map(drop),
            BulkAction::Move { project } => move_issue(
                self,
                MoveIssue {
                    session_token: token,
                    issue: target.issue.clone(),
                    version: target.version,
                    project: project.clone(),
                },
            )
            .await
            .map(drop),
            BulkAction::AddLabel { label } => add_label(
                self,
                AddLabel {
                    session_token: token,
                    issue: target.issue.clone(),
                    label: *label,
                },
            )
            .await
            .map(drop),
            BulkAction::AddRelation {
                relation_kind,
                direction,
                other,
            } => add_relation(
                self,
                AddRelation {
                    session_token: token,
                    issue: target.issue.clone(),
                    other: other.clone(),
                    kind: *relation_kind,
                    direction: *direction,
                },
            )
            .await
            .map(drop),
            BulkAction::Delete => delete_issue(
                self,
                DeleteIssue {
                    session_token: token,
                    issue: target.issue.clone(),
                },
            )
            .await
            .map(drop),
        };
        match done {
            Ok(()) => Ok(outcome(None)),
            Err(error) => Ok(outcome(Some(skip_of(error)?))),
        }
    }
}

/// Changes many issues at once, after the preview, each on the version the preview read. Up to
/// [`BULK_JOB_ABOVE`] issues are changed before the answer comes; more are queued as a job that
/// goes on in the background, and the answer holds the run to read how far it is.
///
/// # Errors
/// `request.invalid` for no issue, too many issues or an action that asks for nothing,
/// `resource.not_found` for a project, a label, a kind of relation or an issue named in the action
/// that is not there for the person, `auth.access_key_not_allowed` for a change to more issues than
/// a script may wait for (an access key splits the work into requests that fit).
pub async fn apply_bulk<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ApplyBulk,
) -> Result<Authorized<BulkResult>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    if input.issues.is_empty() || input.issues.len() > BULK_ISSUES_MAX {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let selection = ctx
        .selection(&actor, &input.session_token, &input.action)
        .await?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    if input.issues.len() <= BULK_JOB_ABOVE {
        let mut maker = ctx.line_maker().await?;
        let mut outcomes = Vec::with_capacity(input.issues.len());
        for target in &input.issues {
            outcomes.push(ctx.apply_item(&selection, target, &mut maker).await?);
        }
        return Ok(held.authorize(BulkResult {
            outcomes,
            run: None,
        }));
    }
    // Too many to wait for. A script with an access key has no way to read the run as itself and
    // the job could not act with the key's narrowed rights, so it asks again in smaller requests.
    if actor.key.is_some() {
        return Err(ErrorCode::AccessKeyNotAllowed.into());
    }
    let mut ids = ctx.ids();
    let run = BulkRun {
        id: BulkRunId::generate(&mut ids).map_err(|error| AppError::Internal(error.to_string()))?,
        person: actor.person.id,
        action: serde_json::to_value(&input.action)
            .map_err(|error| AppError::Internal(error.to_string()))?,
        items: input
            .issues
            .iter()
            .map(|target| {
                // The addresses were judged one by one above only for a small change; a big one
                // names the issues by the IDs the preview read.
                target
                    .issue
                    .parse()
                    .map(|issue| BulkItem {
                        issue,
                        version: target.version,
                    })
                    .map_err(|_| AppError::Refused(ErrorCode::InvalidRequest))
            })
            .collect::<Result<Vec<_>, _>>()?,
        status: BulkStatus::Queued,
        done: 0,
        changed: 0,
        skipped: Vec::new(),
        created_at: ctx.now(),
        finished_at: None,
    };
    let job = JobId::generate(&mut ids).map_err(|error| AppError::Internal(error.to_string()))?;
    let payload = step_payload(run.id);
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::BulkRun, run.id),
        vec![
            set_to("action", json!(input.action.word())),
            set_to("issues", json!(run.items.len())),
        ],
    )?;
    let (stored, now) = (run.clone(), ctx.now());
    let name = kind_name(kind::BULK_STEP)?;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::insert_bulk_run(transaction, &stored).await?;
                S::enqueue_job(transaction, job, &name, &payload, now, now).await
            })
        })
        .await
        .map_err(|error| crate::auth::internal::<S>(&error))?;
    let view = ctx.run_view(&actor, &run).await?;
    Ok(held.authorize(BulkResult {
        outcomes: Vec::new(),
        run: Some(view),
    }))
}
