//! Seeing the statuses a project offers.

use bb_core::ErrorCode;
use bb_core::offer::reaches;
use bb_policy::{Authorized, Masked};
use serde::Serialize;

use super::types::StatusView;
use crate::auth::{AppError, AuthStore, Ctx, internal};

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
    let types = ctx
        .store()
        .issue_types()
        .await
        .map_err(|error| internal::<S>(&error))?;
    // Only the types the person may know (`offers/known.rs`).
    let shown = ctx.shown_to(&actor).await?;
    let types = ctx.known_types(&shown, types).await?;
    let mut changes: Vec<ProjectChange> = Vec::new();
    // Only the types that can reach the project: the statuses of a type created in another project
    // are no business of this one (and nothing about that type is told).
    for issue_type in types
        .iter()
        .filter(|each| each.retired_at.is_none() && reaches(each.project, &chain))
    {
        let workflow = ctx.workflow_in_force(&chain, Some(issue_type.id)).await?;
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
