//! A project's default list (ADR-0034, decision 6): a saved search that is shared with the project
//! and that a person with *Change project settings* chose. Without one the project shows the
//! built-in default of the list (UX §4.2). A project's list and the *Search* page are the same
//! screen; the list is the search with `within KEY` filled in.

use bb_core::ErrorCode;
use bb_core::event::RecordKind;
use bb_core::id::ProjectId;
use bb_core::saved_search::{ProjectDefaultList, ShareTarget};
use bb_policy::{Authorized, Permission, Place};

use super::events::default_list_change;
use super::types::{ProjectList, ProjectListQuery, SetProjectList};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, Stops, by, change_failed, entity, if_there, internal};

impl<S: AuthStore> Ctx<'_, S> {
    /// The project an address names, as the actor may know it: `resource.not-found` for one that
    /// is not there and for one they may not know.
    async fn known_project(&self, actor: &Actor, address: &str) -> Result<ProjectId, AppError> {
        let found = self.find_project(address).await?;
        let place = found.as_ref().map_or(Place::Organization, |found| {
            Place::Project(found.project.id)
        });
        let held = self.evaluation_of(actor, place).await?;
        match found {
            Some(found) if held.knows_project() => Ok(found.project.id),
            _ => Err(ErrorCode::NotFound.into()),
        }
    }

    /// The list a project shows for the actor: the saved search the project has chosen, when it is
    /// still shared with the project and the actor may open it; else none.
    async fn list_of_project(
        &self,
        actor: &Actor,
        project: ProjectId,
    ) -> Result<ProjectList, AppError> {
        let chosen = self
            .store()
            .project_default_list(project)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let Some(chosen) = chosen else {
            return Ok(ProjectList { search: None });
        };
        let Some(opened) = if_there(self.open_search(actor, chosen.search).await)? else {
            return Ok(ProjectList { search: None });
        };
        let shared = opened
            .stored
            .shares
            .iter()
            .any(|share| share.target == ShareTarget::Project(project));
        if !shared {
            return Ok(ProjectList { search: None });
        }
        let view = self
            .one_view(actor, &opened.reaching, opened.stored)
            .await?;
        Ok(ProjectList { search: Some(view) })
    }
}

/// The list a project shows: its default list when it has one that the person may open, else none
/// and the client shows the built-in default.
///
/// # Errors
/// `resource.not-found` for a project that does not exist or that the person may not know.
pub async fn project_list<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ProjectListQuery,
) -> Result<Authorized<ProjectList>, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let project = ctx.known_project(&actor, &input.project).await?;
    let held = ctx.evaluation_of(&actor, Place::Project(project)).await?;
    // A list is for the people who see the project, not for those who only decide who has a role.
    if !held.sees_project() {
        return Err(ErrorCode::NotFound.into());
    }
    let list = ctx.list_of_project(&actor, project).await?;
    Ok(held.authorize(list))
}

/// Chooses the saved search a project shows as its list, or none for the built-in default. The
/// search has to be shared with the project and the person has to be able to open it.
///
/// # Errors
/// `resource.not-found` for a project the person may not know or a search they may not open,
/// `access.forbidden` without *Change project settings* there, `request.invalid` for a search that
/// is not shared with the project.
pub async fn set_project_list<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SetProjectList,
) -> Result<Authorized<ProjectList>, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let project = ctx.known_project(&actor, &input.project).await?;
    let held = ctx
        .require(
            &actor,
            Permission::ChangeProjectSettings,
            Place::Project(project),
        )
        .await?;
    if let Some(search) = input.search {
        let opened = ctx.open_search(&actor, search).await?;
        let shared = opened
            .stored
            .shares
            .iter()
            .any(|share| share.target == ShareTarget::Project(project));
        if !shared {
            return Err(ErrorCode::InvalidRequest.into());
        }
    }
    let was = ctx
        .store()
        .project_default_list(project)
        .await
        .map_err(|error| internal::<S>(&error))?
        .map(|list| list.search);
    if was != input.search {
        let event = ctx.event(
            by(actor.person.id),
            entity(RecordKind::Project, project),
            vec![default_list_change(was, input.search)],
        )?;
        let chosen = input.search;
        ctx.store()
            .record_change(&event, move |transaction| {
                Box::pin(async move {
                    match chosen {
                        Some(search) => {
                            S::put_project_default_list(
                                transaction,
                                &ProjectDefaultList { project, search },
                            )
                            .await
                        }
                        None => S::clear_project_default_list(transaction, project, None)
                            .await
                            .map(|_| ()),
                    }
                })
            })
            .await
            .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    }
    let list = ctx.list_of_project(&actor, project).await?;
    Ok(held.authorize(list))
}
