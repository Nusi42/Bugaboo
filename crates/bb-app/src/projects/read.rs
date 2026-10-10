//! Seeing projects (PRJ-02): one project by its address, and the list of the projects a person
//! sees, as a tree (the projects inside one) or as a search.

use bb_core::ErrorCode;
use bb_core::event::RecordKind;
use bb_core::id::ProjectId;
use bb_core::paging::{Cut, page_len, read_count};
use bb_core::project::ShortName;
use bb_policy::{Authorized, Permission, Place};
use jiff::Timestamp;

use super::shown::Shown;
use super::store::Found;
use super::types::{
    MoveRight, ProjectDetail, ProjectItem, ProjectPage, ProjectQuery, ProjectView, ProjectsQuery,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, if_there, internal};
use crate::fields::Item;

impl<S: AuthStore> Ctx<'_, S> {
    /// The project an address names: an ID, or a short name (also an old one). An address that is
    /// neither asks the store for a project that is not there, so every address, good or not,
    /// costs one question.
    pub(crate) async fn find_project(&self, address: &str) -> Result<Option<Found>, AppError> {
        let nowhere = ProjectId::from_parts(Timestamp::UNIX_EPOCH, [0; 10])
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let by_name = if address.parse::<ProjectId>().is_ok() {
            None
        } else {
            ShortName::new(address).ok()
        };
        if let Some(short_name) = by_name {
            return self
                .store()
                .project_by_address(&short_name)
                .await
                .map_err(|error| internal::<S>(&error));
        }
        let id = address.parse::<ProjectId>().unwrap_or(nowhere);
        Ok(self
            .store()
            .project(id)
            .await
            .map_err(|error| internal::<S>(&error))?
            .map(|project| Found {
                project,
                old_name: false,
            }))
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
        Ok(if_there(self.evaluation_of(actor, place).await)?
            .is_some_and(|held| held.allows(permission)))
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
    // A project that is hidden and one that is missing take the same reads and the same
    // evaluation before either is answered, so the answer is not the only thing that is equal.
    let shown = ctx.shown_to(&actor).await?;
    let found = ctx.find_project(&input.address).await?;
    let place = found.as_ref().map_or(Place::Organization, |found| {
        Place::Project(found.project.id)
    });
    let held = ctx.evaluation_of(&actor, place).await?;
    let Some(found) = found.filter(|_| held.sees_project()) else {
        return Err(AppError::Refused(ErrorCode::NotFound));
    };
    let (trail, truncated) = shown.trail(&found.project);
    let project = found.project;
    let old_parent = ctx
        .evaluation_of(&actor, Place::from(project.parent))
        .await?;
    let organization = ctx.evaluation_of(&actor, Place::Organization).await?;
    let may_move = if !old_parent.allows(Permission::MoveProjects) {
        MoveRight::Nowhere
    } else if project.parent.is_some()
        && organization.allows(Permission::MoveProjects)
        && organization.allows(Permission::MakeProjects)
    {
        MoveRight::IntoProjectsOrTop
    } else {
        MoveRight::IntoProjects
    };
    let chain = ctx.chain_of(project.id).await?;
    let item = Item {
        kind: RecordKind::Project,
        id: Some(project.id.into()),
        chain,
        issue_type: None,
    };
    let values = ctx
        .load_values(RecordKind::Project, &[project.id.into()])
        .await?;
    let changeable_fields = if held.allows(Permission::ChangeProjectSettings) {
        ctx.changeable_own_fields(&held.field_mask(), &item).await?
    } else {
        Vec::new()
    };
    let mut own_fields = values.shown(&held, None, project.id.into());
    let mut people = std::collections::HashMap::new();
    for (key, value) in &mut own_fields.0 {
        if let Some(kind) = values.kind_of(key) {
            *value = ctx
                .display_value(&actor, &held, &mut people, kind, std::mem::take(value))
                .await?;
        }
    }
    own_fields.0.retain(|_, value| !value.is_null());
    Ok(held.authorize(ProjectDetail {
        own_fields,
        changeable_fields,
        trail,
        truncated,
        permissions: held.permissions().iter().copied().collect(),
        may_change_short_name: held.allows(Permission::ChangeProjectSettings)
            && ctx.may_make_projects_beside(&actor, project.parent).await?,
        may_move,
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
/// (the top of the tree when none is named), or those anywhere that a search finds (a search for
/// nothing finds them all). A visible
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
    let access = ctx.evaluation_of(&actor, Place::Organization).await?;
    let shown = ctx.shown_to(&actor).await?;
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
    let mut candidates: Vec<&bb_core::project::Project> = if input.search.is_none() {
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
    // Leaving a project out is only done for one the person sees: naming one they do not see
    // would show, by what drops out of the list, that it is there and what is inside it.
    if let Some(outside) = input.outside.filter(|outside| shown.sees(*outside)) {
        candidates = leave_out(&shown, candidates, outside);
    }
    if let Some(moving) = input.movable.filter(|moving| shown.sees(*moving)) {
        candidates = movable_into(&shown, candidates, moving);
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
    let limit = page_len(Some(input.limit));
    let items: Vec<ProjectItem> = candidates
        .into_iter()
        .filter(|project| {
            after.as_ref().is_none_or(|(name, id)| {
                (project.name.to_lowercase(), project.id) > (name.clone(), *id)
            })
        })
        .take(read_count(limit))
        .map(|project| shown.item(project))
        .collect();
    let page = Cut::of(items, limit);
    let next_cursor = page.next_cursor(write_cursor);
    let items = page.items;
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

/// The candidates this project may be moved into by the person: not itself, not what is inside it,
/// not the place it sits in already, and only where they may move projects.
fn movable_into<'a>(
    shown: &Shown,
    candidates: Vec<&'a bb_core::project::Project>,
    moving: ProjectId,
) -> Vec<&'a bb_core::project::Project> {
    let from = shown.parent_of(moving);
    leave_out(shown, candidates, moving)
        .into_iter()
        .filter(|place| from != Some(place.id) && shown.allows(place.id, Permission::MoveProjects))
        .collect()
}
