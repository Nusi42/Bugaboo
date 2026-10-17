//! The starter workflows of a new organization (WF-06, ADR-0031 decision 5, M4-Q1): *Simple*,
//! *Bug* and *Development*, made once at the first start from the one definition in `bb-core`.
//!
//! *Simple* is used for every issue type that has no workflow of its own, *Bug* for the issue type
//! *Bug* and *Development* for *Feature*. An organization that ran M3 has issues already and
//! keeps the starting workflow *Simple* everywhere (it needs no row), so no issue moves by
//! itself.

use bb_core::event::{Actor, RecordKind};
use bb_core::id::WorkflowId;
use bb_core::message::Message;
use bb_core::version::FIRST_VERSION;
use bb_core::workflow::starter::{SIMPLE, STARTERS};
use bb_core::workflow::{Workflow, WorkflowContent, WorkflowUse};
use serde_json::json;

use crate::auth::{AppError, AuthStore, Ctx, Stops, change_failed, fixed_entity, internal, set_to};
use crate::member::workflow as member;

/// Makes the starter workflows if the organization is new: it has no workflow and no issue yet. It
/// runs at every start and does only what is still to do, so it can be stopped half way and run
/// again.
pub struct CreateStartingWorkflows;

/// Makes the starter workflows once.
///
/// # Errors
/// Fails when the store fails.
pub async fn create_starting_workflows<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    _input: CreateStartingWorkflows,
) -> Result<(), AppError> {
    // New: no workflow was made and no issue was ever written.
    let failed = |error| internal::<S>(&error);
    let none_made = ctx
        .store()
        .workflow_headers()
        .await
        .map_err(failed)?
        .is_empty();
    if !none_made || ctx.store().has_issues().await.map_err(failed)? {
        return Ok(());
    }
    // The job of the first start judges for nobody: it may see all types.
    let types = ctx
        .store()
        .issue_types()
        .await
        .map_err(|error| internal::<S>(&error))?;
    let mut made: Vec<(Workflow, WorkflowContent)> = Vec::with_capacity(STARTERS.len());
    let mut uses: Vec<WorkflowUse> = Vec::new();
    for starter in STARTERS {
        let workflow = Workflow {
            id: WorkflowId::generate(&mut ctx.ids())
                .map_err(|error| AppError::Internal(error.to_string()))?,
            name: starter.title(),
            version: FIRST_VERSION,
        };
        if starter.name == SIMPLE.name {
            // Every issue type that has no workflow of its own, also one made later.
            uses.push(WorkflowUse {
                project: None,
                issue_type: None,
                workflow: workflow.id,
            });
        } else {
            // The issue types it is for, found by the name they were made with.
            for wanted in starter.used_by {
                let name = Message::from(*wanted).render();
                if let Some(found) = types.iter().find(|each| each.name == name) {
                    uses.push(WorkflowUse {
                        project: None,
                        issue_type: Some(found.id),
                        workflow: workflow.id,
                    });
                }
            }
        }
        made.push((workflow, starter.content()));
    }
    let names: Vec<serde_json::Value> = made.iter().map(|(each, _)| json!(each.name)).collect();
    let event = ctx.event(
        Actor::System,
        fixed_entity(RecordKind::Workflow, "starting")?,
        vec![set_to(member::STARTING, serde_json::Value::Array(names))],
    )?;
    let result = ctx
        .store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_policy(transaction).await?;
                // Two servers that start together both saw a new organization above; the one that
                // gets the lock second finds them made and stops, which is not a failure.
                if !S::workflow_headers_in(transaction).await?.is_empty() {
                    return Err(S::aborted());
                }
                for (workflow, content) in &made {
                    S::insert_workflow(transaction, workflow, content).await?;
                }
                for used in &uses {
                    S::put_workflow_use(transaction, used).await?;
                }
                Ok(())
            })
        })
        .await;
    ctx.forget_workflows();
    match result {
        Err(error) if S::is_aborted(&error) => Ok(()),
        Err(error) => Err(change_failed::<S>(&error, Stops::NONE)),
        Ok(()) => Ok(()),
    }
}
