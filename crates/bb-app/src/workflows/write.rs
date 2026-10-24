//! Changing which workflow a place uses: creating the workflow of a place (a copy of the one that
//! counts there), saying that a place uses another one, and taking a workflow away (WF-01,
//! PRJ-02, ADR-0031 decisions 2 and 4).
//!
//! Everything needs *Manage workflows* at the place. Changing what a workflow holds is *Publish*
//! (`publish.rs`).

use bb_core::ErrorCode;
use bb_core::edit::{EditWhat, WORKFLOW_PART};
use bb_core::event::{FieldChange, RecordKind};
use bb_core::id::WorkflowId;
use bb_core::technical::TechnicalName;
use bb_core::version::{FIRST_VERSION, counted_up};
use bb_core::workflow::{Workflow, WorkflowContent, WorkflowPlace, WorkflowUse};
use bb_policy::{Authorized, Permission};
use serde_json::{Value, json};

use super::editor::starting_name;
use super::strand::{count_stranded_in, reach_of_place_in, reach_of_workflow_in};
use super::types::{CreateWorkflow, DeleteWorkflow, UseWorkflow, WorkflowEditor};
use crate::auth::{AppError, AuthStore, Ctx, Done, by, entity, internal, set_to};
use crate::editing::let_go_for_delete;
use crate::member::workflow as member;
use crate::projects::Why;

/// The statuses and the status changes of a workflow as an event writes them.
pub(super) fn summary(content: &WorkflowContent) -> (Value, Value) {
    let statuses = content
        .statuses
        .iter()
        .map(|status| {
            json!({
                "key": status.key.as_str(),
                "name": status.name,
                "group": status.group.code(),
            })
        })
        .collect::<Vec<_>>();
    let changes = content
        .changes
        .iter()
        .map(|change| {
            json!({
                "from": change.from.as_str(),
                "to": change.to.as_str(),
                "button": change.button,
                "rules": change.rules,
            })
        })
        .collect::<Vec<_>>();
    (json!(statuses), json!(changes))
}

/// A place as an event writes it.
pub(super) fn place_value(place: WorkflowPlace) -> Value {
    json!({
        "project": place.project.map(|project| project.to_string()),
        "issue_type": place.issue_type.map(|issue_type| issue_type.to_string()),
    })
}

/// The thing an edit lock on a workflow is about.
pub(super) fn what_of(id: WorkflowId) -> Result<EditWhat, AppError> {
    Ok(EditWhat {
        record: entity(RecordKind::Workflow, id),
        part: TechnicalName::new(WORKFLOW_PART)
            .map_err(|error| AppError::Internal(error.to_string()))?,
    })
}

/// Creates the workflow of a place as a copy of the one that counts there now, so the place can
/// change it (*Customize*). A place that has one is shown as it is.
///
/// # Errors
/// `resource.not-found` for a project the person may not know about, `access.forbidden` without
/// *Manage workflows* at the place, `edit.version-changed` when somebody created the workflow at the
/// same moment.
pub async fn create_workflow<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CreateWorkflow,
) -> Result<Authorized<WorkflowEditor>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    ctx.ensure_type_known(&actor, input.issue_type).await?;
    let at = ctx
        .workflow_where(input.project.as_deref(), input.issue_type)
        .await?;
    let held = ctx
        .require(&actor, Permission::ManageWorkflows, at.place())
        .await?;
    let in_force = ctx
        .workflow_directory()
        .await?
        .detail(&at.chain, at.issue_type);
    if in_force
        .used
        .is_some_and(|used| used.place() == at.workflow_place())
    {
        return Ok(held.authorize(ctx.editor_at(&actor, &held, &at).await?));
    }
    let workflow = Workflow {
        id: WorkflowId::generate(&mut ctx.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?,
        name: in_force
            .workflow
            .as_ref()
            .map_or_else(starting_name, |each| each.name.clone()),
        version: FIRST_VERSION,
    };
    let place = at.workflow_place();
    let used = WorkflowUse {
        project: place.project,
        issue_type: place.issue_type,
        workflow: workflow.id,
    };
    let (statuses, changes) = summary(&in_force.content);
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Workflow, workflow.id),
        vec![
            set_to(member::NAME, json!(workflow.name)),
            set_to(member::USES, json!([place_value(place)])),
            set_to(member::STATUSES, statuses),
            set_to(member::CHANGES, changes),
        ],
    )?;
    let why = Why::default();
    let (stopped, created, copied) = (why.clone(), workflow.clone(), in_force.content.clone());
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_workflows(transaction).await?;
                let placed = S::workflow_uses_in(transaction).await?;
                if placed.iter().any(|each| each.place() == place) {
                    return Err(stopped.stop::<S>(ErrorCode::VersionChanged));
                }
                S::insert_workflow(transaction, &created, &copied).await?;
                S::put_workflow_use(transaction, &used).await
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::VersionChanged))?;
    ctx.forget_workflows();
    Ok(held.authorize(ctx.editor_at(&actor, &held, &at).await?))
}

/// Says which workflow a place uses, or that it follows its parent again (`workflow: None`).
///
/// # Errors
/// `resource.not-found` for a project the person may not know about or a workflow that is not
/// there, `access.forbidden` without *Manage workflows* at the place.
pub async fn use_workflow<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: UseWorkflow,
) -> Result<Authorized<WorkflowEditor>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    ctx.ensure_type_known(&actor, input.issue_type).await?;
    let at = ctx
        .workflow_where(input.project.as_deref(), input.issue_type)
        .await?;
    let held = ctx
        .require(&actor, Permission::ManageWorkflows, at.place())
        .await?;
    let place = at.workflow_place();
    let directory = ctx.workflow_directory().await?;
    let now = directory.uses().iter().find(|each| each.place() == place);
    let chosen = match input.workflow {
        Some(id) => {
            let found = ctx
                .store()
                .stored_workflow(id)
                .await
                .map_err(|error| internal::<S>(&error))?
                .ok_or(AppError::Refused(ErrorCode::NotFound))?
                .workflow
                .id;
            // Only a workflow that sits at the place or above it: its place never moves, and a
            // workflow of a project nobody here manages is not there for this person.
            if !ctx.may_adopt_workflow(&actor, found, &at.chain).await? {
                return Err(AppError::Refused(ErrorCode::NotFound));
            }
            Some(found)
        }
        None => None,
    };
    if now.map(|each| each.workflow) == chosen {
        return Ok(held.authorize(ctx.editor_at(&actor, &held, &at).await?));
    }
    // The event is about the workflow that gains or loses the place.
    let (record, change) = match (chosen, now) {
        (Some(id), before) => (
            id,
            FieldChange {
                field: member::USES.to_owned(),
                before: before.map(|_| place_value(place)),
                after: Some(place_value(place)),
            },
        ),
        (None, Some(each)) => (
            each.workflow,
            FieldChange {
                field: member::USES.to_owned(),
                before: Some(place_value(place)),
                after: None,
            },
        ),
        (None, None) => return Ok(held.authorize(ctx.editor_at(&actor, &held, &at).await?)),
    };
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Workflow, record),
        vec![change],
    )?;
    let why = Why::default();
    let stopped = why.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_workflows(transaction).await?;
                let reach = reach_of_place_in::<S>(transaction, place).await?;
                match chosen {
                    Some(workflow) => {
                        let used = WorkflowUse {
                            project: place.project,
                            issue_type: place.issue_type,
                            workflow,
                        };
                        S::put_workflow_use(transaction, &used).await?;
                    }
                    None => S::remove_workflow_use(transaction, place).await?,
                }
                // No issue of the place is left in a status of a workflow that no longer counts.
                if count_stranded_in::<S>(transaction, &reach).await? > 0 {
                    return Err(stopped.stop::<S>(ErrorCode::WorkflowStatusInUse));
                }
                Ok(())
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::VersionChanged))?;
    ctx.forget_workflows();
    Ok(held.authorize(ctx.editor_at(&actor, &held, &at).await?))
}

/// Takes a workflow away together with the places it was used at: those places follow their
/// parent again, or the starting workflow. The issues it governed keep their statuses; an issue
/// whose status the workflow that counts then does not have is refused here, so *Publish* of the
/// workflow that remains is where such issues are moved.
///
/// # Errors
/// `resource.not-found` for a workflow that is not there, `access.forbidden` without *Manage
/// workflows* where it is used, `workflow.status-in-use` while an issue is in a status that the
/// workflow that would count afterwards does not have, `edit.version-changed` when the workflow
/// changed since the person looked at it, `edit.in-progress` while somebody else holds its edit
/// lock. A lock of the person's own is let go in the same change.
pub async fn delete_workflow<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: DeleteWorkflow,
) -> Result<Done, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let stored = ctx
        .store()
        .stored_workflow(input.workflow)
        .await
        .map_err(|error| internal::<S>(&error))?
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let place = ctx.place_of_workflow(input.workflow).await?;
    ctx.require(&actor, Permission::ManageWorkflows, place)
        .await?;
    let (id, version) = (input.workflow, input.version);
    let record = entity(RecordKind::Workflow, id);
    let (statuses, changes) = summary(&stored.content);
    let event = ctx.event(
        by(actor.person.id),
        record.clone(),
        vec![
            FieldChange {
                field: member::STATUSES.to_owned(),
                before: Some(statuses),
                after: None,
            },
            FieldChange {
                field: member::CHANGES.to_owned(),
                before: Some(changes),
                after: None,
            },
            counted_up(version),
        ],
    )?;
    let why = Why::default();
    let stopped = why.clone();
    let (what, me, now) = (what_of(id)?, actor.person.id, ctx.now());
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_workflows(transaction).await?;
                // The places it is used at are read now, before the use is taken away with it.
                let reach = reach_of_workflow_in::<S>(transaction, id).await?;
                let_go_for_delete::<S>(transaction, &what, me, now, &stopped).await?;
                if !S::delete_workflow(transaction, id, version).await? {
                    return Err(stopped.stop::<S>(ErrorCode::VersionChanged));
                }
                // The places that used it follow their parent now: nothing is left in a status
                // of a workflow that no longer counts.
                if count_stranded_in::<S>(transaction, &reach).await? > 0 {
                    return Err(stopped.stop::<S>(ErrorCode::WorkflowStatusInUse));
                }
                Ok(())
            })
        })
        .await
        .map_err(|error| {
            why.answer::<S>(&error, ErrorCode::VersionChanged).about(
                &record,
                place,
                actor.person.id,
            )
        })?;
    ctx.forget_workflows();
    Ok(Done)
}
