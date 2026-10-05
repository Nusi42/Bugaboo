//! Projects (PRJ-01): the minimal *make a project* that the policy engine needs, so roles have a
//! tree to be given on. Renaming, moving, short-name history and the screens are WP-08.

use bb_core::ErrorCode;
use bb_core::id::ProjectId;
use bb_core::project::{Project, ShortName};
use bb_core::sign_in::SecurityAction;
use bb_policy::{Authorized, Masked, Permission, Place};
use serde::Serialize;
use serde_json::json;

use crate::auth::{AppError, AuthStore, Ctx, by, change_failed, entity, internal, set_to};
use crate::people::clean_name;

/// Make a project at the top level, or inside another project.
pub struct MakeProject {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project it goes into, or `None` for the top level.
    pub parent: Option<ProjectId>,
    /// The letters of its issue IDs and address, as typed.
    pub short_name: String,
    /// What it is called.
    pub name: String,
}

/// A project as it is shown.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProjectView {
    /// The ID of the project.
    pub id: ProjectId,
    /// The project it is inside, or `None` at the top level.
    pub parent: Option<ProjectId>,
    /// The letters of its issue IDs and address.
    pub short_name: ShortName,
    /// What it is called.
    pub name: String,
}

impl Masked for ProjectView {}

/// Makes a project. Top level takes *Make projects*; a sub-project takes *Make sub-projects* in
/// its parent.
///
/// # Errors
/// `access.forbidden` without the permission (`resource.not_found` for a parent the person may
/// not know about), `request.invalid` for a short name or a name that cannot be one,
/// `name.taken` when the short name is taken (only a person who may make projects there is told).
pub async fn make_project<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: MakeProject,
) -> Result<Authorized<ProjectView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = match input.parent {
        None => {
            ctx.require(&actor, Permission::MakeProjects, Place::Organization)
                .await?
        }
        Some(parent) => {
            ctx.require(&actor, Permission::MakeSubProjects, Place::Project(parent))
                .await?
        }
    };
    let short_name =
        ShortName::new(input.short_name.trim()).map_err(|_| ErrorCode::InvalidRequest)?;
    let name = clean_name(&input.name)?;
    if ctx
        .store()
        .project_by_short_name(&short_name)
        .await
        .map_err(|error| internal::<S>(&error))?
        .is_some()
    {
        return Err(ErrorCode::NameTaken.into());
    }
    let project = Project {
        id: ProjectId::generate(&mut ctx.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?,
        parent: input.parent,
        short_name,
        name,
        version: 1,
    };
    let event = ctx.event(
        by(actor.person.id),
        entity("project", project.id)?,
        vec![
            set_to("short_name", json!(project.short_name.as_str())),
            set_to("name", json!(project.name)),
        ],
    )?;
    let stored = project.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                // A project made while somebody gives a role for sub-projects is seen by them or
                // reached by it: the two wait for each other.
                S::lock_policy(transaction).await?;
                S::insert_project(transaction, &stored).await
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Some(ErrorCode::NameTaken)))?;
    ctx.note(
        SecurityAction::ProjectMade,
        Some(actor.person.id),
        Some(entity("project", project.id)?),
    )
    .await;
    Ok(access.authorize(ProjectView {
        id: project.id,
        parent: project.parent,
        short_name: project.short_name,
        name: project.name,
    }))
}
