//! The workflow a project has of its own (WF-01, WF-03, WF-04): the organization's starting workflow
//! copied for one project, renamed, with one status change that *asks for* the person who does the
//! work. It is made the way the editor makes it: *Create*, the edit lock, *Publish*.
//!
//! Every other place keeps the starter workflows (*Simple*, *Bug*, *Development*) that the first
//! start made, so the sample has both: the project with *Ask for* and all the rest without.

use bb_app::editing::{TakeLockQuery, take_lock};
use bb_app::workflows::{CreateWorkflow, PublishWorkflow, create_workflow, publish_workflow};
use bb_core::edit::{EditWhat, WORKFLOW_PART};
use bb_core::event::{EntityRef, RecordKind};
use bb_core::fields::{Field, FieldKey};
use bb_core::id::{EditorId, LockToken, WorkflowId};
use bb_core::technical::TechnicalName;
use bb_core::workflow::StatusRules;
use bb_core::workflow::starter::{IN_PROGRESS, OPEN};
use bb_dev_words::sample::OwnWorkflow;

use crate::error::{SampleError, Step};
use crate::generate::World;
use crate::issues_setup::{id_in, json};

/// Gives the project of the plan a workflow of its own, as the administrator whose session is
/// `token`: the workflow that counted there, with the status change from *Open* to *In progress*
/// asking for *Assigned to*.
pub(crate) async fn create(
    world: &World<'_>,
    token: &str,
    plan: &OwnWorkflow,
) -> Result<(), SampleError> {
    let ctx = world.services.ctx(world.request());
    let step = Step::Workflows;
    world.tick();
    // The administrator's own editor, as the use case answered it (the sample is one process
    // without an attacker, so the mask is not looked at).
    let mut shown = None;
    let _ = create_workflow(
        &ctx,
        CreateWorkflow {
            session_token: token.to_owned(),
            project: Some(plan.project.clone()),
            issue_type: None,
        },
    )
    .await
    .map_err(|error| SampleError::step(step, &error))?
    .map(|editor| shown = Some(editor));
    let editor = shown.ok_or_else(|| SampleError::NotMade("workflow".to_owned()))?;
    let workflow: WorkflowId = editor
        .workflow
        .map(|slot| slot.id)
        .ok_or_else(|| SampleError::NotMade("workflow".to_owned()))?;

    world.tick();
    let lock = json(
        take_lock(
            &ctx,
            TakeLockQuery {
                session_token: token.to_owned(),
                what: EditWhat {
                    record: EntityRef {
                        kind: RecordKind::Workflow.into(),
                        id: workflow.into(),
                    },
                    part: TechnicalName::new(WORKFLOW_PART)
                        .map_err(|error| SampleError::step(step, &error))?,
                },
                editor: EditorId::generate(&mut bb_app::context::SystemIds),
            },
        )
        .await,
        step,
    )?;
    let lock_token: LockToken = id_in(&lock, "token")?;

    // What the editor shows, with the name and with the one status change that starts work asking
    // for the person; every other rule stays as it is.
    let mut publish = PublishWorkflow::from_editor(token.to_owned(), &editor, lock_token)
        .map_err(|error| SampleError::step(step, &error))?;
    plan.name.clone_into(&mut publish.name);
    for change in &mut publish.changes {
        if change.from == OPEN && change.to == IN_PROGRESS {
            change.rules = StatusRules {
                ask_for: vec![FieldKey::BuiltIn(Field::AssignedTo)],
                ..change.rules.clone()
            };
        }
    }
    world.tick();
    publish_workflow(&ctx, publish)
        .await
        .map_err(|error| SampleError::step(step, &error))?;
    Ok(())
}
