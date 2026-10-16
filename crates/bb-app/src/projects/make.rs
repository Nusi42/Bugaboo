//! *Make a project* (PRJ-01): at the top level, or inside another project.

use bb_core::ErrorCode;
use bb_core::event::RecordKind;
use bb_core::id::ProjectId;
use bb_core::project::{Project, ShortName};
use bb_core::sign_in::SecurityAction;
use bb_core::version::FIRST_VERSION;
use bb_policy::{Authorized, Permission, Place, Tree};
use serde_json::json;

use super::Why;
use super::types::{MakeProject, ProjectView};
use crate::auth::{AppError, AuthStore, Ctx, by, entity, internal, set_to};
use crate::input::clean_name;
use crate::member;

/// Makes a project. Top level takes *Create projects*; a sub-project takes *Create sub-projects* in
/// its parent.
///
/// # Errors
/// `access.forbidden` without the permission (`resource.not_found` for a parent the person may
/// not know about), `request.invalid` for a short name or a name that cannot be one,
/// `project.too_deep` when the project would be nested deeper than a tree may go, `name.taken`
/// when the short name is taken, or was used by a project once (only a person who
/// may create projects there is told, with the same answer whether the project is hidden or not).
pub async fn make_project<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: MakeProject,
) -> Result<Authorized<ProjectView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = match input.parent {
        None => {
            ctx.require(&actor, Permission::CreateProjects, Place::Organization)
                .await?
        }
        Some(parent) => {
            let access = ctx
                .require(
                    &actor,
                    Permission::CreateSubProjects,
                    Place::Project(parent),
                )
                .await?;
            // A project the person does not see is not there for them, whatever they may make in
            // it: otherwise making in it would tell them it exists.
            if !access.sees_project() {
                return Err(AppError::Refused(ErrorCode::NotFound));
            }
            access
        }
    };
    let short_name =
        ShortName::new(input.short_name.trim()).map_err(|_| ErrorCode::InvalidRequest)?;
    let name = clean_name(&input.name)?;
    if ctx
        .store()
        .project_by_address(&short_name)
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
        version: FIRST_VERSION,
    };
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Project, project.id),
        vec![
            set_to(
                member::project::SHORT_NAME,
                json!(project.short_name.as_str()),
            ),
            set_to(member::project::NAME, json!(project.name)),
            set_to(
                "parent",
                json!(project.parent.map(|parent| parent.to_string())),
            ),
        ],
    )?;
    let stored = project.clone();
    let why = Why::default();
    let stopped = why.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                // A project made while somebody gives a role for sub-projects is seen by them or
                // reached by it: the two wait for each other.
                S::lock_policy(transaction).await?;
                // A project goes no deeper than the tree may: judged on the tree as it is now.
                if let Some(parent) = stored.parent {
                    let tree = Tree::of(&S::project_tree_in(transaction).await?);
                    if !tree.contains(parent) {
                        return Err(stopped.stop::<S>(ErrorCode::NotFound));
                    }
                    if tree
                        .moved(stored.id, Some(parent))
                        .too_deep_below(stored.id)
                    {
                        return Err(stopped.stop::<S>(ErrorCode::ProjectTooDeep));
                    }
                }
                // Judged again on what is stored now: two people who make a project with the
                // same short name at the same moment cannot both succeed.
                if S::short_name_owner_in(transaction, &stored.short_name)
                    .await?
                    .is_some()
                {
                    return Err(stopped.stop::<S>(ErrorCode::NameTaken));
                }
                S::insert_project(transaction, &stored).await
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::NameTaken))?;
    ctx.note(
        SecurityAction::ProjectMade,
        Some(actor.person.id),
        Some(entity(RecordKind::Project, project.id)),
    )
    .await;
    Ok(access.authorize(ProjectView {
        id: project.id,
        parent: project.parent,
        short_name: project.short_name,
        name: project.name,
        version: project.version,
    }))
}
