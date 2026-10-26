//! Seeing the statuses a project offers.

use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::id::ProjectId;
use bb_core::issue::IssueType;
use bb_core::offer::reaches;
use bb_core::workflow::WorkflowContent;
use bb_policy::{Authorized, Masked};
use serde::Serialize;

use super::types::StatusView;
use crate::auth::{AppError, AuthStore, Ctx};
use crate::projects::Shown;

/// Ask which statuses the issues of a project can have.
pub struct ProjectStatusesQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project, by its ID or its short name.
    pub project: String,
}

/// The status changes the issues of a project can have: those of the workflow of every issue type
/// that is not retired, in the order the workflows list them. What a choice of a status change for
/// many issues offers; each issue is still judged by its own workflow.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProjectStatuses {
    /// The status changes by their words: each status a change ends in with the text of its
    /// button, once for the same status and text in several workflows, in the order the workflows
    /// list them. What a choice of a status change for many issues offers.
    pub changes: Vec<ProjectChange>,
}

/// A status change of some workflow of a project: where it leads and what its button says.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProjectChange {
    /// The status it ends in.
    pub to: StatusView,
    /// The text of the button.
    pub button: String,
}

impl Masked for ProjectStatuses {}

/// The statuses the issues of a project can have.
///
/// # Errors
/// `resource.not-found` for a project that does not exist or that the person may not see.
pub async fn project_statuses<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ProjectStatusesQuery,
) -> Result<Authorized<ProjectStatuses>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let found = ctx.find_project(&input.project).await?;
    let place = bb_policy::Place::from(found.as_ref().map(|found| found.project.id));
    let held = ctx.evaluation_of(&actor, place).await?;
    let Some(found) = found.filter(|_| held.sees_project()) else {
        return Err(AppError::Refused(ErrorCode::NotFound));
    };
    let chain = ctx.chain_of(found.project.id).await?;
    let shown = ctx.shown_to(&actor).await?;
    let mut changes: Vec<ProjectChange> = Vec::new();
    // Only the types the person may know that can reach the project: the statuses of a type created
    // in another project are no business of this one (and nothing about that type is told).
    for (_, workflow) in ctx
        .workflows_reaching(&shown, &chain)
        .await?
        .iter()
        .filter(|(issue_type, _)| issue_type.retired_at.is_none())
    {
        for change in &workflow.changes {
            let Some(to) = workflow.status(&change.to) else {
                continue;
            };
            let shown = ProjectChange {
                to: StatusView::from(to),
                button: change.button.clone(),
            };
            if !changes.contains(&shown) {
                changes.push(shown);
            }
        }
    }
    Ok(held.authorize(ProjectStatuses { changes }))
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The issue types that the person may know and that can reach a place whose projects from the
    /// top down are `chain`, each with the workflow in force there: the one answer to "which
    /// statuses can an issue here have", for the statuses of a project and for the choices of a
    /// search.
    pub(crate) async fn workflows_reaching(
        &self,
        shown: &Shown,
        chain: &[ProjectId],
    ) -> Result<Vec<(IssueType, Arc<WorkflowContent>)>, AppError> {
        let directory = self.workflow_directory().await?;
        let types = self
            .known_types_reaching(shown, self.all_issue_types().await?.to_vec(), |owner| {
                reaches(owner, chain)
            })
            .await?;
        Ok(types
            .into_iter()
            .map(|found| {
                let workflow = directory.in_force(chain, Some(found.id));
                (found, workflow)
            })
            .collect())
    }
}
