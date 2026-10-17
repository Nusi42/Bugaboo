//! *Publish* (WF-02, ADR-0031 decision 4): the editor sends the whole workflow, and it counts from
//! now on.
//!
//! The workflow is saved under the edit lock the person holds, on the version they looked at. When
//! it drops a status that issues are still in, the person says where those issues go. Every such
//! move is judged first, on the same path as the preview of a change to many issues and in the
//! workflow as it will be; if any issue would be skipped, nothing is published. The moves are made
//! inside the change that saves the workflow, each with its history, and the last thing the change
//! does is to check that no live issue is left in a status its workflow does not have
//! (`strand.rs`). So *Publish* either does all of it or none.

use std::collections::BTreeMap;

use bb_core::ErrorCode;
use bb_core::access::FieldLevel;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::fields::FieldKey;
use bb_core::id::{GroupId, WorkflowId};
use bb_core::issue::{Issue, StatusGroup};
use bb_core::many_changes::MANY_CHANGES_ISSUES_MAX;
use bb_core::own_fields::FieldFor;
use bb_core::version::counted_up;
use bb_core::workflow::{StatusKey, StoredWorkflow, WorkflowContent, check_name};
use bb_policy::{Authorized, Evaluation, Permission};
use jiff::Timestamp;
use serde_json::json;

use super::strand::{count_stranded_in, reach_of_workflow_in};
use super::types::{PublishWorkflow, Published, StatusMoveInput, WorkflowSlot};
use super::write::{summary, what_of};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, EventMaker, by, entity, internal};
use crate::editing::{TextSave, Written};
use crate::issues::{ChangeIssue, member as issue_member};
use crate::member::workflow as member;

/// An issue that goes to another status because the status it was in is dropped.
struct Moving {
    issue: Issue,
    target: StatusKey,
    /// The status group of the target: it decides the closed date.
    group: StatusGroup,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The workflow the editor sends, when it is one that can be stored: it breaks no rule, the own
    /// fields its rules name are fields of issues, and every rule that is new or changed names only
    /// fields the person may see at the place and groups they may know. Rules that are there
    /// already stay as they are, whatever they name.
    async fn publishable(
        &self,
        actor: &Actor,
        held: &Evaluation,
        stored: &StoredWorkflow,
        input: &PublishWorkflow,
    ) -> Result<WorkflowContent, AppError> {
        check_name(&input.name).map_err(|_| AppError::from(ErrorCode::WorkflowInvalid))?;
        let content = WorkflowContent::from_drafts(&input.statuses, &input.changes)
            .map_err(|_| AppError::from(ErrorCode::WorkflowInvalid))?;
        let definitions = self.field_catalogue().await?;
        let known_groups: Vec<GroupId> = self
            .groups_offered(actor, &stored.content)
            .await?
            .into_iter()
            .map(|group| group.id)
            .collect();
        for change in &content.changes {
            let already = stored
                .content
                .change(&change.from, &change.to)
                .is_some_and(|each| each.rules == change.rules);
            for field in change.rules.fields() {
                if let FieldKey::Own(id) = field {
                    let for_issues = definitions.iter().any(|each| {
                        each.field.id == id && each.field.applies_to == FieldFor::Issues
                    });
                    if !for_issues {
                        return Err(ErrorCode::FieldNotHere.into());
                    }
                }
                if !already && held.field_level_for(None, field) < FieldLevel::CanSee {
                    return Err(ErrorCode::WorkflowInvalid.into());
                }
            }
            let groups = &change.rules.who.groups;
            if !already && groups.iter().any(|group| !known_groups.contains(group)) {
                return Err(ErrorCode::WorkflowInvalid.into());
            }
        }
        Ok(content)
    }

    /// Where the issues of the dropped statuses go: for each live issue the workflow governs that
    /// is in a dropped status, the status it is sent to. Each move is judged as a change by the
    /// person who publishes, in the workflow as it will be; one that would be skipped (they may not
    /// change the issue, the status is not allowed, or the issue changed since) refuses the whole
    /// publish.
    async fn plan_moves(
        &self,
        actor: &Actor,
        stored: &StoredWorkflow,
        content: &WorkflowContent,
        asked: &[StatusMoveInput],
    ) -> Result<Vec<Moving>, AppError> {
        let dropped = stored.content.dropped_by(content);
        // A move goes only to the starting status of the workflow: nothing else is allowed to an
        // issue outside its workflow (ADR-0031 amendment 3).
        let sensible = asked.iter().all(|each| {
            dropped.contains(&each.from)
                && content.status(&each.to).is_some()
                && content.initial() == Some(&each.to)
        });
        if !sensible {
            return Err(ErrorCode::InvalidRequest.into());
        }
        if dropped.is_empty() {
            return Ok(Vec::new());
        }
        let directory = self.workflow_directory().await?;
        let issues = self
            .governed_issues(&directory, stored.workflow.id, &dropped)
            .await?;
        // One publish moves no more issues than one change to many issues takes (the job route of
        // a bigger move is a follow-up).
        if issues.len() > MANY_CHANGES_ISSUES_MAX {
            return Err(ErrorCode::WorkflowStatusInUse.into());
        }
        let mut planned = Vec::with_capacity(issues.len());
        for issue in issues {
            let target = asked
                .iter()
                .find(|each| each.from == issue.status)
                .map(|each| each.to.clone())
                .ok_or(AppError::Refused(ErrorCode::WorkflowStatusInUse))?;
            let group = content
                .status(&target)
                .map(|each| each.group)
                .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
            planned.push(Moving {
                issue,
                target,
                group,
            });
        }
        // From here on the request sees the workflow as it will be.
        self.see_directory(directory.with_content(stored.workflow.id, content.clone()));
        let mut skipped = 0_usize;
        for each in &planned {
            let asked = ChangeIssue {
                session_token: String::new(),
                issue: each.issue.id.to_string(),
                version: each.issue.version,
                issue_type: None,
                title: None,
                status: Some(each.target.clone()),
                priority: None,
                assigned_to: None,
                fields: BTreeMap::new(),
            };
            match self.plan_issue_change(actor, &asked).await {
                Ok(_) => {}
                // A failure of the server is not a skipped issue.
                Err(error @ AppError::Internal(_)) => {
                    self.forget_workflows();
                    return Err(error);
                }
                Err(_) => skipped += 1,
            }
        }
        self.forget_workflows();
        if skipped > 0 {
            return Err(ErrorCode::WorkflowStatusInUse.into());
        }
        Ok(planned)
    }
}

/// Carries out the moves of the issues of the dropped statuses inside the change, each with its history.
/// Says why not when one cannot be made as it was planned: the issue changed since.
async fn carry_out_moves<S: AuthStore>(
    transaction: &mut S::Transaction,
    moving: &[Moving],
    events: &EventMaker,
    now: Timestamp,
) -> Result<Option<ErrorCode>, S::Error> {
    for each in moving {
        let Some(stored) = S::issue_in(transaction, each.issue.id).await? else {
            return Ok(Some(ErrorCode::WorkflowStatusInUse));
        };
        if stored.deleted_at.is_some() {
            // In the trash an issue keeps its status; it starts again when it comes back.
            continue;
        }
        if stored.version != each.issue.version || stored.status != each.issue.status {
            return Ok(Some(ErrorCode::WorkflowStatusInUse));
        }
        let next = Issue {
            status: each.target.clone(),
            closed_at: each.group.closed_after(stored.closed_at, now),
            updated_at: now,
            ..stored.clone()
        };
        if !S::update_issue(transaction, &next, stored.version).await? {
            return Ok(Some(ErrorCode::WorkflowStatusInUse));
        }
        let followed = events
            .create(
                entity(RecordKind::Issue, stored.id),
                vec![
                    issue_member::status_change(&stored.status, &each.target),
                    counted_up(stored.version),
                ],
            )
            .map_err(|_| S::aborted())?;
        S::append_follow_up_event(transaction, &followed).await?;
    }
    Ok(None)
}

/// Publishes a workflow under the edit lock the person holds: the statuses, the status changes and
/// their rules are replaced by what was sent, and the issues of the statuses that are dropped go
/// where the person said, in the same change.
///
/// # Errors
/// `resource.not_found` for a workflow that is not there, `access.forbidden` without *Manage
/// workflows* where it is used, `workflow.invalid` for a workflow that breaks a rule or a rule that
/// names a field or a group the person may not know, `field.not_here` for a rule that names an own
/// field that issues do not have, `workflow.status_in_use` while an issue would be left in a
/// status the workflow no longer has or could not be moved by this person, `request.invalid` for a
/// move that makes no sense; `edit.lock_lost`, `edit.taken_over` and `edit.version_changed` for a
/// lock that is gone, was taken over, or a workflow that changed since (the workflow is kept as the
/// draft of the person in all three).
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
    let content = ctx.publishable(&actor, &held, &stored, &input).await?;
    let moving = ctx
        .plan_moves(&actor, &stored, &content, &input.moves)
        .await?;
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
    let (events, now) = (ctx.event_maker(by(actor.person.id)), ctx.now());
    ctx.save_text(actor.person.id, place, &save, &event, move |transaction| {
        Box::pin(async move {
            S::lock_workflows(transaction).await?;
            // The places are read now, with the lock: they are what is stored when the check runs.
            let reach = reach_of_workflow_in::<S>(transaction, id).await?;
            if !S::replace_workflow(transaction, id, &written_name, &written, version).await? {
                return Ok(Written::Stale);
            }
            if let Some(refused) = carry_out_moves::<S>(transaction, &moving, &events, now).await? {
                return Ok(Written::Refused(refused));
            }
            if count_stranded_in::<S>(transaction, &reach).await? > 0 {
                return Ok(Written::Refused(ErrorCode::WorkflowStatusInUse));
            }
            Ok(Written::Saved(()))
        })
    })
    .await?;
    ctx.forget_workflows();
    Ok(held.authorize(Published {
        workflow: WorkflowSlot {
            id,
            version: version + 1,
        },
    }))
}
