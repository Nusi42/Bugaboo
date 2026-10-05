//! The tree as one person sees it: which projects they may see, where a visible project is shown
//! and what its way down is called.
//!
//! The decision of what a person sees is the engine's (`bb_policy::Policy::visible`), made for the
//! whole tree from one read of the tree and one of the person's rows. A project the person does
//! not see is not in the tree they are shown: a visible project whose parent is hidden is shown at
//! the top, and neither its parent, nor an ID, nor the name of the way down to it ever names what
//! is hidden (UX §2.1, ADR-0025 decision 13).

use std::collections::{BTreeMap, BTreeSet};

use bb_core::id::{ProjectId, UserId};
use bb_core::project::{DEPTH_MAX, Project};
use bb_policy::{Permission, Place, Policy, Tree};

use super::types::{ProjectItem, ProjectRef, TRAIL_MAX};
use crate::auth::{AppError, AuthStore, Ctx, internal};

/// The projects, and which of them one person sees.
pub(crate) struct Shown {
    person: UserId,
    policy: Policy,
    tree: Tree,
    projects: BTreeMap<ProjectId, Project>,
    visible: BTreeSet<ProjectId>,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// What `person` sees of the tree, as it is stored now.
    pub(crate) async fn shown_to(&self, person: UserId) -> Result<Shown, AppError> {
        let projects = self
            .store()
            .project_tree()
            .await
            .map_err(|error| internal::<S>(&error))?;
        let policy = self
            .store()
            .policy_of(person)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let tree = Tree::new(projects.iter().map(|project| (project.id, project.parent)));
        Ok(Shown {
            visible: policy.visible(person, &tree),
            person,
            policy,
            tree,
            projects: projects
                .into_iter()
                .map(|project| (project.id, project))
                .collect(),
        })
    }
}

impl Shown {
    /// Whether the person sees the project.
    pub(crate) fn sees(&self, project: ProjectId) -> bool {
        self.visible.contains(&project)
    }

    /// Whether the person holds the permission in the project, from the same rows and the same
    /// engine as every other decision.
    pub(crate) fn allows(&self, project: ProjectId, permission: Permission) -> bool {
        self.policy
            .evaluate_at(self.person, &self.tree, Place::Project(project))
            .is_some_and(|held| held.allows(permission))
    }

    /// The parent a stored project has, whether the person sees it or not: only for comparing
    /// places, never to be named.
    pub(crate) fn parent_of(&self, project: ProjectId) -> Option<ProjectId> {
        self.projects.get(&project).and_then(|found| found.parent)
    }

    /// The project, if the person sees it.
    pub(crate) fn get(&self, project: ProjectId) -> Option<&Project> {
        self.projects
            .get(&project)
            .filter(|found| self.sees(found.id))
    }

    /// The project as a name in a sentence, if the person sees it.
    pub(crate) fn reference(&self, project: ProjectId) -> Option<ProjectRef> {
        self.get(project).map(reference_of)
    }

    /// The parent of the project, when the person sees the parent too.
    pub(crate) fn shown_parent(&self, project: &Project) -> Option<ProjectId> {
        project.parent.filter(|parent| self.sees(*parent))
    }

    /// The projects the person sees that are shown directly inside `parent`: the top of their
    /// tree for `None`, which holds the projects at the top level and the visible projects whose
    /// parent is hidden.
    pub(crate) fn shown_inside(&self, parent: Option<ProjectId>) -> Vec<&Project> {
        self.visible
            .iter()
            .filter_map(|id| self.projects.get(id))
            .filter(|project| self.shown_parent(project) == parent)
            .collect()
    }

    /// Every project the person sees.
    pub(crate) fn everything(&self) -> Vec<&Project> {
        self.visible
            .iter()
            .filter_map(|id| self.projects.get(id))
            .collect()
    }

    /// Whether `project` is `outside` or inside it, at any depth (also through projects the person
    /// does not see: the tree is the stored one).
    pub(crate) fn is_inside(&self, project: &Project, outside: ProjectId) -> bool {
        let mut current = Some(project.id);
        for _ in 0..DEPTH_MAX {
            let Some(id) = current else {
                return false;
            };
            if id == outside {
                return true;
            }
            current = self.projects.get(&id).and_then(|known| known.parent);
        }
        false
    }

    /// The projects above `project` that the person sees, from the top down, stopping at the
    /// first one they do not see; and whether the list was cut at [`TRAIL_MAX`].
    pub(crate) fn trail(&self, project: &Project) -> (Vec<ProjectRef>, bool) {
        let mut trail = Vec::new();
        let mut current = self.shown_parent(project);
        let mut truncated = false;
        while let Some(parent) = current {
            if trail.len() >= TRAIL_MAX {
                truncated = true;
                break;
            }
            let Some(found) = self.get(parent) else {
                break;
            };
            trail.push(reference_of(found));
            current = self.shown_parent(found);
        }
        trail.reverse();
        (trail, truncated)
    }

    /// The project as a line of a list.
    pub(crate) fn item(&self, project: &Project) -> ProjectItem {
        ProjectItem {
            id: project.id,
            short_name: project.short_name.clone(),
            name: project.name.clone(),
            has_sub_projects: !self.shown_inside(Some(project.id)).is_empty(),
            parent: self
                .shown_parent(project)
                .and_then(|parent| self.reference(parent)),
        }
    }
}

fn reference_of(project: &Project) -> ProjectRef {
    ProjectRef {
        id: project.id,
        short_name: project.short_name.clone(),
        name: project.name.clone(),
    }
}
