//! Seeing projects (PRJ-02): one project by its address, and the list of the projects a person
//! sees, as a tree (the projects inside one) or as a search.

use bb_core::ErrorCode;
use bb_core::id::ProjectId;
use bb_core::project::ShortName;
use bb_policy::{Authorized, Permission, Place};

use super::shown::Shown;
use super::store::Found;
use super::types::{
    ProjectDetail, ProjectItem, ProjectPage, ProjectQuery, ProjectView, ProjectsQuery,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};

impl<S: AuthStore> Ctx<'_, S> {
    /// The project an address names: an ID, or a short name (also an old one).
    async fn find_project(&self, address: &str) -> Result<Option<Found>, AppError> {
        if let Ok(id) = address.parse::<ProjectId>() {
            return Ok(self
                .store()
                .project(id)
                .await
                .map_err(|error| internal::<S>(&error))?
                .map(|project| Found {
                    project,
                    old_name: false,
                }));
        }
        let Ok(short_name) = ShortName::new(address) else {
            return Ok(None);
        };
        self.store()
            .project_by_address(&short_name)
            .await
            .map_err(|error| internal::<S>(&error))
    }

    /// Whether the actor may make projects where the project sits: *Make projects* for the whole
    /// organization at the top level, *Make sub-projects* in the parent below it. Whoever may is
    /// told that a short name is taken; nobody else is (ADR-0025, decision 13).
    pub(crate) async fn may_make_projects_beside(
        &self,
        actor: &Actor,
        parent: Option<ProjectId>,
    ) -> Result<bool, AppError> {
        let (permission, place) = parent
            .map_or((Permission::MakeProjects, Place::Organization), |parent| {
                (Permission::MakeSubProjects, Place::Project(parent))
            });
        // A parent that is gone holds nobody back from nothing: it is not one they may make in.
        Ok(self
            .evaluate(actor.person.id, place)
            .await
            .is_ok_and(|held| held.allows(permission)))
    }
}

/// One project by its address: its ID or its short name, also an old one (the answer then holds
/// the current short name, so the page can show the address that is right now).
///
/// # Errors
/// `resource.not_found` for a project that does not exist or that the person may not see: the
/// two cannot be told apart.
pub async fn project<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ProjectQuery,
) -> Result<Authorized<ProjectDetail>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let not_found = || AppError::Refused(ErrorCode::NotFound);
    let found = ctx
        .find_project(&input.address)
        .await?
        .ok_or_else(not_found)?;
    let held = ctx
        .evaluate(actor.person.id, Place::Project(found.project.id))
        .await?;
    if !held.sees_project() {
        return Err(not_found());
    }
    let shown = ctx.shown_to(actor.person.id).await?;
    let (trail, truncated) = shown.trail(&found.project);
    let project = found.project;
    Ok(held.authorize(ProjectDetail {
        trail,
        truncated,
        permissions: held.permissions().iter().copied().collect(),
        may_change_short_name: held.allows(Permission::ChangeProjectSettings)
            && ctx.may_make_projects_beside(&actor, project.parent).await?,
        project: ProjectView {
            id: project.id,
            parent: shown.shown_parent(&project),
            short_name: project.short_name,
            name: project.name,
            version: project.version,
        },
    }))
}

/// Where a page ends, as text: the name the list is sorted by and the ID of the last project.
/// Names hold no control characters, so a tab cannot be part of one.
fn write_cursor(last: &ProjectItem) -> String {
    format!("{}\t{}", last.name.to_lowercase(), last.id)
}

fn read_cursor(text: &str) -> Result<(String, ProjectId), ()> {
    let (name, id) = text.split_once('\t').ok_or(())?;
    Ok((name.to_owned(), id.parse().map_err(|_| ())?))
}

/// Whether every word of `search` is in the name or the short name of the project.
fn matches(search: &[String], name: &str, short_name: &str) -> bool {
    let (name, short_name) = (name.to_lowercase(), short_name.to_lowercase());
    search
        .iter()
        .all(|word| name.contains(word) || short_name.contains(word))
}

/// The projects the person sees, a page at a time, by name: the projects directly inside one
/// (the top of the tree when none is named), or those anywhere that a search finds. A visible
/// project whose parent is hidden is at the top of the tree.
///
/// # Errors
/// `resource.not_found` for a project to look inside that the person may not see (or that does
/// not exist), `request.invalid` for a cursor that is not one.
pub async fn projects<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ProjectsQuery,
) -> Result<Authorized<ProjectPage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    // The projects of the answer are the ones the person sees, from their rows: the answer is
    // wrapped by the evaluation for the whole organization, which holds no field of a project.
    let access = ctx.evaluate(actor.person.id, Place::Organization).await?;
    let shown = ctx.shown_to(actor.person.id).await?;
    let words: Vec<String> = input
        .search
        .as_deref()
        .map(|text| {
            text.to_lowercase()
                .split_whitespace()
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let mut candidates: Vec<&bb_core::project::Project> = if words.is_empty() {
        if let Some(inside) = input.inside
            && !shown.sees(inside)
        {
            return Err(AppError::Refused(ErrorCode::NotFound));
        }
        shown.shown_inside(input.inside)
    } else {
        shown
            .everything()
            .into_iter()
            .filter(|project| matches(&words, &project.name, project.short_name.as_str()))
            .collect()
    };
    if let Some(outside) = input.outside {
        candidates = leave_out(&shown, candidates, outside);
    }
    candidates.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then(a.id.cmp(&b.id))
    });
    let after = input
        .cursor
        .as_deref()
        .map(read_cursor)
        .transpose()
        .map_err(|()| AppError::Refused(ErrorCode::InvalidRequest))?;
    let limit = usize::try_from(bb_core::paging::page_size(Some(input.limit))).unwrap_or(1);
    let mut items: Vec<ProjectItem> = candidates
        .into_iter()
        .filter(|project| {
            after.as_ref().is_none_or(|(name, id)| {
                (project.name.to_lowercase(), project.id) > (name.clone(), *id)
            })
        })
        .take(limit + 1)
        .map(|project| shown.item(project))
        .collect();
    let more = items.len() > limit;
    items.truncate(limit);
    let next_cursor = more.then(|| items.last().map(write_cursor)).flatten();
    Ok(access.authorize(ProjectPage { items, next_cursor }))
}

/// The candidates without `outside` and everything inside it, as the person sees the tree.
fn leave_out<'a>(
    shown: &Shown,
    candidates: Vec<&'a bb_core::project::Project>,
    outside: ProjectId,
) -> Vec<&'a bb_core::project::Project> {
    candidates
        .into_iter()
        .filter(|project| !shown.is_inside(project, outside))
        .collect()
}
