//! *Publish* (WF-02, ADR-0031 decision 4): the editor sends the whole workflow, and it counts from
//! now on.
//!
//! The workflow is saved under the edit lock the person holds, on the version they looked at. When
//! it drops a status that issues are still in, the person says where those issues go: any status
//! of the new workflow, also a new one named like a status change names it (ADR-0031 amendments 2
//! and 11). A move is no status change and no rule of one applies to it; it is judged with the
//! rights of the person who publishes (`mover_of`), and if any issue would be skipped, or there are
//! more than one change to many issues takes, nothing is published. The moves are made inside the
//! change that saves the workflow, each with its history, and the last thing the change does is to
//! check that no live issue is left in a status its workflow does not have (`strand.rs`). So
//! *Publish* either does all of it or none.

use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::access::FieldLevel;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::fields::FieldKey;
use bb_core::id::{GroupId, WorkflowId};
use bb_core::issue::{Issue, StatusGroup};
use bb_core::many_changes::MANY_CHANGES_ISSUES_MAX;
use bb_core::own_fields::FieldFor;
use bb_core::version::counted_up;
use bb_core::workflow::{
    ChangeDraft, StatusDraft, StatusKey, StoredWorkflow, WorkflowContent, check_name, is_new_in,
    status_named,
};
use bb_policy::{Authorized, Evaluation, Permission, Place};
use jiff::Timestamp;
use serde_json::json;

use super::status::StatusCheck;
use super::strand::{count_stranded_in, reach_of_workflow_in};
use super::types::{PublishWorkflow, Published, StatusMoveInput, WorkflowSlot};
use super::write::{summary, what_of};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, EventMaker, by, entity, internal};
use crate::editing::{TextSave, Written};
use crate::issues::{Where, member as issue_member};
use crate::member::workflow as member;

/// What the editor sends of a workflow: what *Publish* and its preview both judge.
#[derive(Clone, Copy)]
pub(super) struct Drafts<'a> {
    pub(super) name: &'a str,
    pub(super) statuses: &'a [StatusDraft],
    pub(super) changes: &'a [ChangeDraft],
}

impl<'a> From<&'a PublishWorkflow> for Drafts<'a> {
    fn from(input: &'a PublishWorkflow) -> Self {
        Self {
            name: &input.name,
            statuses: &input.statuses,
            changes: &input.changes,
        }
    }
}

/// What the person who publishes can do about one issue of a dropped status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mover {
    /// They do not see the issue.
    Unseen,
    /// They see it and may not change its status.
    Blocked,
    /// They may move it.
    Can,
}

/// The live issues that are in the statuses a publish drops, in the workflow as it is stored.
pub(super) struct Dropping {
    /// The statuses that are dropped.
    pub(super) statuses: Vec<StatusKey>,
    /// The issues in them: at most one change to many issues takes, and one more when there are
    /// more of them than that.
    pub(super) issues: Vec<Issue>,
    /// The most issues one publish moves.
    pub(super) cap: usize,
}

impl Dropping {
    /// Whether there are more issues than one change to many issues takes, so nothing is judged.
    pub(super) fn too_many(&self) -> bool {
        self.issues.len() > self.cap
    }
}

/// What a publish does, judged before anything is written (`publish_plan`).
pub(crate) struct PublishPlan {
    actor: Actor,
    stored: StoredWorkflow,
    held: Arc<Evaluation>,
    place: Place,
    content: WorkflowContent,
    moving: Vec<Moving>,
}

#[cfg(test)]
impl PublishPlan {
    /// How many issues the plan moves.
    pub(crate) fn move_count(&self) -> usize {
        self.moving.len()
    }
}

/// An issue that goes to another status because the status it was in is dropped.
pub(crate) struct Moving {
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
    pub(super) async fn publishable(
        &self,
        actor: &Actor,
        held: &Evaluation,
        stored: &StoredWorkflow,
        drafts: Drafts<'_>,
    ) -> Result<WorkflowContent, AppError> {
        check_name(drafts.name).map_err(|_| AppError::from(ErrorCode::WorkflowInvalid))?;
        let content = WorkflowContent::from_drafts(drafts.statuses, drafts.changes)
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
                // A field that does not exist for issues and a field the person may not see get
                // the same answer, so the answer cannot be used to find out which it is.
                let unknown = matches!(field, FieldKey::Own(id) if !definitions.iter().any(|each| {
                    each.field.id == id && each.field.applies_to == FieldFor::Issues
                }));
                let hidden = held.field_level_for(None, field) < FieldLevel::CanSee;
                if !already && (unknown || hidden) {
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

    /// The stored workflow, what the person holds where it is used and that place, when they may
    /// manage it there: the first step of *Publish* and of its preview.
    pub(super) async fn workflow_to_manage(
        &self,
        actor: &Actor,
        id: WorkflowId,
    ) -> Result<(StoredWorkflow, Arc<Evaluation>, Place), AppError> {
        let stored = self
            .store()
            .stored_workflow(id)
            .await
            .map_err(|error| internal::<S>(&error))?
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        let place = self.place_of_workflow(id).await?;
        let held = self
            .require(actor, Permission::ManageWorkflows, place)
            .await?;
        Ok((stored, held, place))
    }

    /// The live issues in the statuses that `content` drops, found from the places the stored
    /// workflow is used at, up to one more than `cap`. The one walk that *Publish* and its preview
    /// share.
    pub(super) async fn dropping(
        &self,
        stored: &StoredWorkflow,
        content: &WorkflowContent,
        cap: usize,
    ) -> Result<Dropping, AppError> {
        let statuses = stored.content.dropped_by(content);
        if statuses.is_empty() {
            return Ok(Dropping {
                statuses,
                issues: Vec::new(),
                cap,
            });
        }
        let directory = self.workflow_directory().await?;
        let issues = self
            .governed_issues(&directory, stored.workflow.id, &statuses, cap)
            .await?;
        Ok(Dropping {
            statuses,
            issues,
            cap,
        })
    }

    /// Whether the person who publishes may move an issue: they see it and may change its status
    /// (`StatusCheck::may_change_status`, which holds the right to change the issue). The one judge
    /// of a move in *Publish* and its preview, with no rule of a status change: the issue has no
    /// status change into the new workflow, and the person chose where it goes.
    pub(super) async fn mover_of(&self, actor: &Actor, issue: &Issue) -> Result<Mover, AppError> {
        let Some(held) = self.seen_in(actor, issue, Where::Live).await? else {
            return Ok(Mover::Unseen);
        };
        let may = StatusCheck::may_change_status(&held, issue.issue_type, issue.reported_by);
        Ok(if may { Mover::Can } else { Mover::Blocked })
    }

    /// What a publish does, judged before anything is written: who publishes, the workflow as it is
    /// stored, the workflow that is sent, and the moves of the issues of the dropped statuses. One
    /// publish moves at most `cap` issues (a test makes it small, as for the preview).
    pub(crate) async fn publish_plan(
        &self,
        input: &PublishWorkflow,
        cap: usize,
    ) -> Result<PublishPlan, AppError> {
        let actor = self.actor(&input.session_token).await?;
        let (stored, held, place) = self.workflow_to_manage(&actor, input.workflow).await?;
        let content = self
            .publishable(&actor, &held, &stored, Drafts::from(input))
            .await?;
        let moving = self
            .plan_moves(
                &actor,
                &stored,
                &content,
                &input.statuses,
                &input.moves,
                cap,
            )
            .await?;
        Ok(PublishPlan {
            actor,
            stored,
            held,
            place,
            content,
            moving,
        })
    }

    /// Where the issues of the dropped statuses go: for each live issue the workflow governs that
    /// is in a dropped status, the status it is sent to. Each move is judged as a change by the
    /// person who publishes (`mover_of`); one they could not make refuses the whole publish.
    async fn plan_moves(
        &self,
        actor: &Actor,
        stored: &StoredWorkflow,
        content: &WorkflowContent,
        drafts: &[StatusDraft],
        asked: &[StatusMoveInput],
        cap: usize,
    ) -> Result<Vec<Moving>, AppError> {
        let dropping = self.dropping(stored, content, cap).await?;
        // A move goes to a status the new workflow has, named by its key or, when it is new, by
        // its name, from a status it drops.
        let mut targets: Vec<(StatusKey, StatusKey)> = Vec::new();
        for each in asked {
            match target_of(drafts, content, &each.to) {
                Some(to) if dropping.statuses.contains(&each.from) => {
                    targets.push((each.from.clone(), to));
                }
                _ => return Err(ErrorCode::InvalidRequest.into()),
            }
        }
        // One publish moves no more issues than one change to many issues takes (the job route of
        // a bigger move is a follow-up).
        if dropping.too_many() {
            return Err(ErrorCode::WorkflowStatusInUse.into());
        }
        let mut planned = Vec::with_capacity(dropping.issues.len());
        for issue in dropping.issues {
            let target = targets
                .iter()
                .find(|(from, _)| *from == issue.status)
                .map(|(_, to)| to.clone())
                .ok_or(AppError::Refused(ErrorCode::WorkflowStatusInUse))?;
            let group = content
                .status(&target)
                .map(|each| each.group)
                .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
            if self.mover_of(actor, &issue).await? != Mover::Can {
                return Err(ErrorCode::WorkflowStatusInUse.into());
            }
            planned.push(Moving {
                issue,
                target,
                group,
            });
        }
        Ok(planned)
    }
}

/// The status of the new workflow that a move names: by its key, or, for a status that is new in
/// the draft (no draft gave it a key), by its name read as the editor reads it (ADR-0031
/// amendment 2).
fn target_of(drafts: &[StatusDraft], content: &WorkflowContent, named: &str) -> Option<StatusKey> {
    status_named(&content.statuses, named, |each| is_new_in(drafts, each))
        .map(|each| each.key.clone())
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
/// names a field or a group the person may not know (an own field that issues do not have is
/// refused in the same words), `workflow.status_in_use` while an issue would be left in a
/// status the workflow no longer has or could not be moved by this person, `request.invalid` for a
/// move that makes no sense; `edit.lock_lost`, `edit.taken_over` and `edit.version_changed` for a
/// lock that is gone, was taken over, or a workflow that changed since (the workflow is kept as the
/// draft of the person in all three).
pub async fn publish_workflow<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: PublishWorkflow,
) -> Result<Authorized<Published>, AppError> {
    let PublishPlan {
        actor,
        stored,
        held,
        place,
        content,
        moving,
    } = ctx.publish_plan(&input, MANY_CHANGES_ISSUES_MAX).await?;
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
