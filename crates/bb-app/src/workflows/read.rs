//! Seeing the statuses a project offers.

use bb_core::ErrorCode;
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

/// The statuses the issues of a project can have: those of the workflow of every issue type that is
/// not retired, in the order the workflows list them. A status that two workflows share with the
/// same key, name and group is listed once; one with the same key and another name or group is
/// listed for each. What a choice of a status for
/// many issues offers; each issue is still judged by its own workflow.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProjectStatuses {
    /// The statuses.
    pub statuses: Vec<StatusView>,
}

impl Masked for ProjectStatuses {}

/// The statuses the issues of a project can have.
///
/// # Errors
/// `resource.not_found` for a project that does not exist or that the person may not see.
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
    let mut statuses: Vec<StatusView> = Vec::new();
    for issue_type in types.iter().filter(|each| each.retired_at.is_none()) {
        let workflow = ctx.workflow_in_force(&chain, Some(issue_type.id)).await?;
        for status in &workflow.statuses {
            let shown = StatusView::from(status);
            if !statuses.contains(&shown) {
                statuses.push(shown);
            }
        }
    }
    Ok(held.authorize(ProjectStatuses { statuses }))
}
