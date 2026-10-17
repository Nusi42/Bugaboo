//! The workflow a project has of its own (WF-01, WF-03, WF-04): the organization's starting workflow
//! copied for one project, renamed, with one status change that *asks for* the person who does the
//! work. It is made the way the editor makes it: *Create*, the edit lock, *Publish*.
//!
//! Every other place keeps the starter workflows (*Simple*, *Bug*, *Development*) that the first
//! start made, so the sample has both: the project with *Ask for* and all the rest without.

use bb_app::access::Masked;
use bb_app::editing::{TakeLockQuery, take_lock};
use bb_app::workflows::{
    CreateWorkflow, EditorChange, PublishWorkflow, StatusView, WorkflowEditor, WorkflowSlot,
    create_workflow, publish_workflow,
};
use bb_core::edit::{EditWhat, WORKFLOW_PART};
use bb_core::event::{EntityRef, RecordKind};
use bb_core::fields::{Field, FieldKey};
use bb_core::id::{EditorId, LockToken, WorkflowId};
use bb_core::technical::TechnicalName;
use bb_core::workflow::starter::{IN_PROGRESS, OPEN};
use bb_core::workflow::{ChangeDraft, StatusDraft, StatusRules};
use bb_dev_words::sample::OwnWorkflow;
use serde::Serialize;
use serde_json::Value;

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
    let editor = json(
        create_workflow(
            &ctx,
            CreateWorkflow {
                session_token: token.to_owned(),
                project: Some(plan.project.clone()),
                issue_type: None,
            },
        )
        .await
        .map(|editor| editor.map(Drafting::from)),
        step,
    )?;
    let workflow: WorkflowId = id_in(&editor["workflow"], "id")?;
    let version = editor["workflow"]["version"]
        .as_i64()
        .ok_or_else(|| SampleError::NotMade(plan.name.clone()))?;

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

    world.tick();
    publish_workflow(
        &ctx,
        PublishWorkflow {
            session_token: token.to_owned(),
            workflow,
            version,
            lock_token,
            name: plan.name.clone(),
            statuses: statuses_of(&editor)?,
            changes: changes_of(&editor)?,
            moves: Vec::new(),
        },
    )
    .await
    .map_err(|error| SampleError::step(step, &error))?;
    Ok(())
}

/// What the sample reads of the editor of a workflow: its place, its statuses and its status
/// changes, as the JSON a client would get.
#[derive(Serialize)]
struct Drafting {
    workflow: Option<WorkflowSlot>,
    statuses: Vec<StatusView>,
    changes: Vec<EditorChange>,
}

impl Masked for Drafting {}

impl From<WorkflowEditor> for Drafting {
    fn from(editor: WorkflowEditor) -> Self {
        Self {
            workflow: editor.workflow,
            statuses: editor.statuses,
            changes: editor.changes,
        }
    }
}

/// The text of a member of an answer.
fn text(value: &Value, member: &str) -> Result<String, SampleError> {
    value[member]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| SampleError::NotMade(member.to_owned()))
}

/// The statuses the editor shows, as the drafts that *Publish* takes back.
fn statuses_of(editor: &Value) -> Result<Vec<StatusDraft>, SampleError> {
    editor["statuses"]
        .as_array()
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .map(|status| {
            let group = text(status, "group")?;
            Ok(StatusDraft {
                key: Some(
                    text(status, "key")?
                        .parse()
                        .map_err(|_| SampleError::NotMade("status".to_owned()))?,
                ),
                name: text(status, "name")?,
                group: bb_core::issue::StatusGroup::from_code(&group)
                    .ok_or(SampleError::NotMade(group))?,
            })
        })
        .collect()
}

/// The status changes the editor shows, with the one that starts work asking for the person.
fn changes_of(editor: &Value) -> Result<Vec<ChangeDraft>, SampleError> {
    editor["changes"]
        .as_array()
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .map(|change| {
            let (from, to) = (text(change, "from")?, text(change, "to")?);
            let mut rules = StatusRules::default();
            if from == OPEN && to == IN_PROGRESS {
                rules.ask_for = vec![FieldKey::BuiltIn(Field::AssignedTo)];
            }
            Ok(ChangeDraft {
                from,
                to,
                button: text(change, "button")?,
                rules,
            })
        })
        .collect()
}
