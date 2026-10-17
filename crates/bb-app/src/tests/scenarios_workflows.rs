//! The scenarios of the workflow use cases (ADR-0007, M1-F18), and the preparation the tests of
//! workflows share: a workflow of a project, its edit lock, and what *Publish* is sent from an
//! editor. `events.rs` names each use case once and runs them.

use bb_core::edit::EditLock;
use bb_core::event::{EntityRef, RecordKind};
use bb_core::id::{EditorId, IssueTypeId, LockToken};
use bb_core::technical::TechnicalName;
use bb_core::workflow::{ChangeDraft, StatusDraft};
use jiff::Timestamp;

use super::scenarios::{Added, a_project, measured, text, token};
use super::scenarios_issues::a_type;
use super::world::World;
use crate::workflows::{
    CreateStartingWorkflows, CreateWorkflow, DeleteWorkflow, PublishWorkflow, UseWorkflow,
    WorkflowEditor, WorkflowSlot, create_starting_workflows, create_workflow, delete_workflow,
    publish_workflow, use_workflow,
};

/// The input that creates the workflow of a project for all issue types.
pub(crate) fn new_workflow(world: &World, project: Option<&str>) -> CreateWorkflow {
    CreateWorkflow {
        session_token: token(world),
        project: project.map(str::to_owned),
        issue_type: None,
    }
}

/// Creates the workflow of a project and says what the editor shows.
pub(crate) async fn a_workflow(world: &World, project: &str) -> Result<WorkflowEditor, String> {
    Ok(
        create_workflow(&world.ctx(), new_workflow(world, Some(project)))
            .await
            .map_err(text)?
            .into_value(),
    )
}

/// The slot of an editor that has a workflow of its own.
pub(crate) fn slot_of(editor: &WorkflowEditor) -> Result<WorkflowSlot, String> {
    editor
        .workflow
        .ok_or_else(|| "the place has no workflow of its own".to_owned())
}

/// The lock Ada holds on a workflow, to the end of the century.
pub(crate) fn lock_the_workflow(world: &World, slot: WorkflowSlot) -> LockToken {
    let token = LockToken::from_random([7; 16]);
    world.store.lock().state.locks.push(EditLock {
        record: EntityRef {
            kind: RecordKind::Workflow.into(),
            id: slot.id.into(),
        },
        part: TechnicalName::new(bb_core::edit::WORKFLOW_PART).unwrap_or_else(|_| unreachable!()),
        holder: world.admin.person.id,
        editor: EditorId::from_random([6; 16]),
        token,
        taken_at: Timestamp::UNIX_EPOCH,
        expires_at: Timestamp::MAX,
    });
    token
}

/// What *Publish* is sent for a workflow just as the editor shows it; a test changes the parts it
/// is about.
pub(crate) fn publish_of(
    world: &World,
    editor: &WorkflowEditor,
    lock_token: LockToken,
) -> Result<PublishWorkflow, String> {
    let slot = slot_of(editor)?;
    Ok(PublishWorkflow {
        session_token: token(world),
        workflow: slot.id,
        version: slot.version,
        lock_token,
        name: editor.name.clone(),
        statuses: editor
            .statuses
            .iter()
            .map(|status| StatusDraft {
                key: Some(status.key.clone()),
                name: status.name.clone(),
                group: status.group,
            })
            .collect(),
        changes: editor
            .changes
            .iter()
            .map(|change| ChangeDraft {
                from: change.from.to_string(),
                to: change.to.to_string(),
                button: change.button.clone(),
                rules: change.rules.clone(),
            })
            .collect(),
        moves: Vec::new(),
    })
}

/// Publishes the workflow of a project after `change` changed what the editor showed; the lock is
/// taken for the test. Says what the person got.
pub(crate) async fn published_with(
    world: &World,
    editor: &WorkflowEditor,
    change: impl FnOnce(&mut PublishWorkflow),
) -> Result<(), crate::auth::AppError> {
    let slot = editor
        .workflow
        .ok_or_else(|| crate::auth::AppError::Internal("no workflow of its own".into()))?;
    let lock = lock_the_workflow(world, slot);
    let mut sent = publish_of(world, editor, lock).map_err(crate::auth::AppError::Internal)?;
    change(&mut sent);
    publish_workflow(&world.ctx(), sent).await?;
    Ok(())
}

/// A server that has not made its starter workflows: the first start.
pub(crate) async fn create_starting_workflows_scenario(_world: &World) -> Added {
    let bare = World::new_without_workflows().await?;
    measured(
        &bare.store,
        create_starting_workflows(&bare.ctx(), CreateStartingWorkflows),
    )
    .await
}

pub(crate) async fn create_workflow_scenario(world: &World) -> Added {
    a_project(world, "ACME").await?;
    measured(
        &world.store,
        create_workflow(&world.ctx(), new_workflow(world, Some("ACME"))),
    )
    .await
}

pub(crate) async fn use_workflow_scenario(world: &World) -> Added {
    a_project(world, "OTHER").await?;
    // The workflow of the whole organization: a project may put it to use for one issue type.
    let editor = create_workflow(&world.ctx(), new_workflow(world, None))
        .await
        .map_err(text)?
        .into_value();
    let slot = slot_of(&editor)?;
    let bugs: IssueTypeId = a_type(world).await?;
    measured(
        &world.store,
        use_workflow(
            &world.ctx(),
            UseWorkflow {
                session_token: token(world),
                project: Some("OTHER".to_owned()),
                issue_type: Some(bugs),
                workflow: Some(slot.id),
            },
        ),
    )
    .await
}

pub(crate) async fn publish_workflow_scenario(world: &World) -> Added {
    a_project(world, "ACME").await?;
    let editor = a_workflow(world, "ACME").await?;
    let lock = lock_the_workflow(world, slot_of(&editor)?);
    let mut sent = publish_of(world, &editor, lock)?;
    "Simple, renamed".clone_into(&mut sent.name);
    measured(&world.store, Box::pin(publish_workflow(&world.ctx(), sent))).await
}

pub(crate) async fn delete_workflow_scenario(world: &World) -> Added {
    a_project(world, "ACME").await?;
    let editor = a_workflow(world, "ACME").await?;
    let slot = slot_of(&editor)?;
    measured(
        &world.store,
        delete_workflow(
            &world.ctx(),
            DeleteWorkflow {
                session_token: token(world),
                workflow: slot.id,
                version: slot.version,
            },
        ),
    )
    .await
}
