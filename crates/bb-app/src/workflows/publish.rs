//! *Publish* (WF-02, ADR-0031 decision 4): the editor sends the whole workflow, and it counts from
//! now on.
//!
//! The workflow is saved under the edit lock the person holds, on the version they looked at. When
//! it drops a status that issues are still in, the person says where those issues go, and they are
//! moved by changes to many issues (ADR-0017) once the workflow is saved, each with its history.
//! Nothing is saved while an issue would be left in a status the workflow no longer has.

use bb_core::ErrorCode;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::fields::FieldKey;
use bb_core::id::WorkflowId;
use bb_core::issue::Issue;
use bb_core::many_changes::MANY_CHANGES_ISSUES_MAX;
use bb_core::own_fields::FieldFor;
use bb_core::version::counted_up;
use bb_core::workflow::{StatusKey, StoredWorkflow, WorkflowContent, check_name};
use bb_policy::{Authorized, Permission};
use serde_json::json;

use super::types::{PublishWorkflow, Published, StatusMoveInput, WorkflowSlot};
use super::write::{summary, what_of};
use crate::auth::{AppError, AuthStore, Ctx, by, entity, internal};
use crate::editing::{TextSave, Written};
use crate::many_changes::{
    ApplyManyChanges, ManyChange, ManyChangeAction, ManyChangeTarget, apply_many_changes,
};
use crate::member::workflow as member;

/// The issues that go to one status, because the status they were in is dropped.
struct Moving {
    target: StatusKey,
    issues: Vec<Issue>,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The workflow the editor sends, when it is one that can be stored: it breaks no rule, and the
    /// own fields its rules name are fields of issues.
    async fn publishable(&self, input: &PublishWorkflow) -> Result<WorkflowContent, AppError> {
        check_name(&input.name).map_err(|_| AppError::from(ErrorCode::WorkflowInvalid))?;
        let content = WorkflowContent::from_drafts(&input.statuses, &input.changes)
            .map_err(|_| AppError::from(ErrorCode::WorkflowInvalid))?;
        let definitions = self.field_catalogue().await?;
        for change in &content.changes {
            for field in change.rules.fields() {
                let FieldKey::Own(id) = field else {
                    continue;
                };
                let for_issues = definitions
                    .iter()
                    .any(|each| each.field.id == id && each.field.applies_to == FieldFor::Issues);
                if !for_issues {
                    return Err(ErrorCode::FieldNotHere.into());
                }
            }
        }
        Ok(content)
    }

    /// Where the issues of the dropped statuses go: for each issue the workflow governs that is in
    /// a dropped status, the status it is sent to.
    async fn plan_moves(
        &self,
        stored: &StoredWorkflow,
        content: &WorkflowContent,
        asked: &[StatusMoveInput],
    ) -> Result<Vec<Moving>, AppError> {
        let dropped = stored.content.dropped_by(content);
        let sensible = asked
            .iter()
            .all(|each| dropped.contains(&each.from) && content.status(&each.to).is_some());
        if !sensible {
            return Err(ErrorCode::InvalidRequest.into());
        }
        if dropped.is_empty() {
            return Ok(Vec::new());
        }
        let directory = self.workflow_directory().await?;
        let issues = self
            .governed_issues(&directory, stored.workflow.id, dropped)
            .await?;
        let mut moving: Vec<Moving> = Vec::new();
        for (issue, _) in issues {
            let target = asked
                .iter()
                .find(|each| each.from == issue.status)
                .map(|each| each.to.clone())
                .ok_or(AppError::Refused(ErrorCode::WorkflowStatusInUse))?;
            match moving.iter_mut().find(|each| each.target == target) {
                Some(found) => found.issues.push(issue),
                None => moving.push(Moving {
                    target,
                    issues: vec![issue],
                }),
            }
        }
        Ok(moving)
    }

    /// Moves the issues as changes to many issues, as the person who published, in steps of the
    /// most issues one change names.
    async fn run_moves(&self, token: &str, moving: Vec<Moving>) -> Result<(), AppError> {
        for Moving { target, issues } in moving {
            for step in issues.chunks(MANY_CHANGES_ISSUES_MAX) {
                apply_many_changes(
                    self,
                    ApplyManyChanges {
                        session_token: token.to_owned(),
                        issues: step
                            .iter()
                            .map(|issue| ManyChangeTarget {
                                issue: issue.id.to_string(),
                                version: issue.version,
                            })
                            .collect(),
                        action: ManyChangeAction::Change {
                            change: ManyChange {
                                status: Some(target.clone()),
                                ..ManyChange::default()
                            },
                        },
                    },
                )
                .await?;
            }
        }
        Ok(())
    }
}

/// Publishes a workflow under the edit lock the person holds: the statuses, the status changes and
/// their rules are replaced by what was sent, and the issues of the statuses that are dropped go
/// where the person said.
///
/// # Errors
/// `resource.not_found` for a workflow that is not there, `access.forbidden` without *Manage
/// workflows* where it is used, `workflow.invalid` for a workflow that breaks a rule,
/// `field.not_here` for a rule that names an own field that issues do not have,
/// `workflow.status_in_use` while an issue is in a dropped status that has no place to go to,
/// `request.invalid` for a move that makes no sense; `edit.lock_lost`, `edit.taken_over` and
/// `edit.version_changed` for a lock that is gone, was taken over, or a workflow that changed
/// since (the workflow is kept as the draft of the person in all three).
pub async fn publish_workflow<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: PublishWorkflow,
) -> Result<Authorized<Published>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let stored = ctx
        .store()
        .stored_workflow(input.workflow)
        .await
        .map_err(|error| internal::<S>(&error))?
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let place = ctx.place_of_workflow(input.workflow).await?;
    let held = ctx
        .require(&actor, Permission::ManageWorkflows, place)
        .await?;
    let content = ctx.publishable(&input).await?;
    let moving = ctx.plan_moves(&stored, &content, &input.moves).await?;
    let id: WorkflowId = input.workflow;
    let record = entity(RecordKind::Workflow, id);
    let name = input.name.trim().to_owned();
    let (before, after) = (summary(&stored.content), summary(&content));
    let mut changes = Vec::new();
    if name != stored.workflow.name {
        changes.push(FieldChange {
            field: member::NAME.to_owned(),
            before: Some(json!(stored.workflow.name)),
            after: Some(json!(name)),
        });
    }
    changes.push(FieldChange {
        field: member::STATUSES.to_owned(),
        before: Some(before.0),
        after: Some(after.0.clone()),
    });
    changes.push(FieldChange {
        field: member::CHANGES.to_owned(),
        before: Some(before.1),
        after: Some(after.1.clone()),
    });
    changes.push(counted_up(input.version));
    let event = ctx.event(by(actor.person.id), record, changes)?;
    let save = TextSave {
        what: what_of(id)?,
        token: input.lock_token,
        // The draft is what the person sent.
        text: json!({"name": name, "statuses": after.0, "changes": after.1}).to_string(),
        version: input.version,
    };
    let (version, written, written_name) = (input.version, content, name);
    ctx.save_text(actor.person.id, place, &save, &event, move |transaction| {
        Box::pin(async move {
            if S::replace_workflow(transaction, id, &written_name, &written, version).await? {
                Ok(Written::Saved(()))
            } else {
                Ok(Written::Stale)
            }
        })
    })
    .await?;
    ctx.forget_workflows();
    ctx.run_moves(&input.session_token, moving).await?;
    Ok(held.authorize(Published {
        workflow: WorkflowSlot {
            id,
            version: version + 1,
        },
    }))
}
