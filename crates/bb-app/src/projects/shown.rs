//! The tree as one person sees it: which projects they may see, where a visible project is shown
//! and what its way down is called.
//!
//! The decision of what a person sees is the engine's (`bb_policy::Policy::visible`), made for the
//! whole tree from one read of the tree and one of the person's rows. A project the person does
//! not see is not in the tree they are shown: a visible project whose parent is hidden is shown at
//! the top, and neither its parent, nor an ID, nor the name of the way down to it ever names what
//! is hidden (UX §2.1, ADR-0025 decision 13).

use std::collections::{BTreeMap, BTreeSet};

use bb_core::access::FieldLevel;
use bb_core::fields::Field;
use bb_core::id::{IssueTypeId, ProjectId, UserId};
use bb_core::project::Project;
use bb_core::tags::IssueSight;
use bb_policy::{Evaluation, Narrowing, Permission, Place, Policy, Tree};

use super::types::{ProjectItem, ProjectRef, TRAIL_MAX};
use crate::access::{Actor, ProjectLabel};
use crate::auth::{AppError, AuthStore, Ctx, internal};

/// The projects, and which of them one person sees.
#[derive(Clone)]
pub(crate) struct Shown {
    person: UserId,
    /// What the access key behind the request narrows the person to, if one is in use: it counts
    /// in everything below, so a key for one project sees that project and what is inside it, and
    /// its trail stops there (M2-F27).
    key: Option<Narrowing>,
    policy: Policy,
    tree: Tree,
    projects: BTreeMap<ProjectId, Project>,
    visible: BTreeSet<ProjectId>,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// What the actor sees of the tree, as it is stored now: their own view, narrowed to the access
    /// key they came with, if any.
    pub(crate) async fn shown_to(&self, actor: &Actor) -> Result<Shown, AppError> {
        let (policy, projects) = self
            .store()
            .policy_and_tree_of(actor.person.id)
            .await
            .map_err(|error| internal::<S>(&error))?;
        Ok(Shown::of(
            actor.person.id,
            actor.key_scope().cloned(),
            policy,
            projects,
        ))
    }

    /// The same view as [`Ctx::shown_to`], with the levels ([`Ctx::shown_with_levels`]).
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn shown_with_levels_to(&self, actor: &Actor) -> Result<Leveled, AppError> {
        self.shown_with_levels(actor.person.id, actor.key_scope().cloned())
            .await
    }

    /// The same view of `person` (narrowed to `key`), with the levels of their roles, so that a
    /// decision about a field can be made for every project in memory (the tags to offer, the
    /// rights in the projects of a page). The rows, the levels and the projects come from one
    /// read.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn shown_with_levels(
        &self,
        person: UserId,
        key: Option<Narrowing>,
    ) -> Result<Leveled, AppError> {
        let (policy, projects) = self
            .store()
            .leveled_policy_and_tree_of(person)
            .await
            .map_err(|error| internal::<S>(&error))?;
        Ok(Leveled(Shown::of(person, key, policy, projects)))
    }

    /// The view `shown` with the levels of the roles of its person added: for what is needed only
    /// now and then (the groups of labels in use), so the view is not read again, only the
    /// levels. The levels come from a later transaction than the rows and the tree; that is
    /// acceptable because they only decide whether the name of a group in a project the person
    /// does not see is known to them, never what they see of the tree.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn leveled(&self, shown: &Shown) -> Result<Leveled, AppError> {
        let held = shown.policy.role_ids();
        let levels = self
            .store()
            .level_rules_of_roles(held)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let mut leveled = shown.clone();
        leveled.policy.rules = levels;
        Ok(Leveled(leveled))
    }
}

/// A [`Shown`] whose policy holds the levels of the roles, so that a decision about a field can be
/// made for every project in memory. Only this type can judge it: a view without the levels
/// cannot be given where they are needed.
#[derive(Clone)]
pub(crate) struct Leveled(Shown);

impl std::ops::Deref for Leveled {
    type Target = Shown;

    fn deref(&self) -> &Shown {
        &self.0
    }
}

impl Leveled {
    /// Which issues the person reads where their role lets them see `field` (SEC-06), judged for
    /// every project and issue type in memory: a project where the field is hidden for every type
    /// is left out, and the types for which it is hidden in the others are named in `hidden`.
    pub(crate) fn issue_sight_with(&self, field: Field, types: &[IssueTypeId]) -> IssueSight {
        let mut sight = self.issue_sight();
        let mut kept = Vec::new();
        for list in [&mut sight.all, &mut sight.own] {
            list.retain(|project| {
                let Some(held) = self.held_in(*project) else {
                    return false;
                };
                let hidden: Vec<IssueTypeId> = types
                    .iter()
                    .copied()
                    .filter(|issue_type| {
                        held.field_level_for(Some(*issue_type), field) < FieldLevel::CanSee
                    })
                    .collect();
                if hidden.len() == types.len() {
                    return false;
                }
                kept.extend(hidden.into_iter().map(|issue_type| (*project, issue_type)));
                true
            });
        }
        sight.hidden = kept;
        sight
    }

    /// What the person may do in each project they see, with the projects from the top down to it,
    /// for the projects at or below `within` (all of them without): the places a search looks at,
    /// so the fields it may use are the ones readable in every one of them.
    pub(crate) fn seen_places(
        &self,
        within: Option<ProjectId>,
    ) -> Vec<(Vec<ProjectId>, std::sync::Arc<Evaluation>)> {
        self.visible
            .iter()
            .filter(|project| within.is_none_or(|top| self.tree.is_within(**project, top)))
            .filter_map(|project| Some((self.tree.chain(*project)?, self.held_in(*project)?)))
            .collect()
    }

    /// The projects as a search needs them to read a short name and a place in the tree, for those
    /// among `projects` that the person sees.
    pub(crate) fn links(&self, projects: &[ProjectId]) -> Vec<crate::search::ProjectLink> {
        projects
            .iter()
            .filter_map(|project| self.stored(*project))
            .map(|stored| crate::search::ProjectLink {
                id: stored.id,
                parent: stored.parent,
                short_name: stored.short_name.to_string(),
            })
            .collect()
    }

    /// Which issues the person reads where their role lets them see the labels (SEC-06), judged in
    /// memory; for the groups of labels in use.
    pub(crate) fn label_sight(&self, types: &[IssueTypeId]) -> IssueSight {
        self.issue_sight_with(Field::Labels, types)
    }
}

impl Shown {
    fn of(person: UserId, key: Option<Narrowing>, policy: Policy, projects: Vec<Project>) -> Self {
        let tree = Tree::of(&projects);
        Self {
            visible: policy.visible_as(person, key.as_ref(), &tree),
            person,
            key,
            policy,
            tree,
            projects: projects
                .into_iter()
                .map(|project| (project.id, project))
                .collect(),
        }
    }

    /// The person it is the view of.
    pub(crate) const fn person(&self) -> UserId {
        self.person
    }

    /// What the person themselves may do in each of `projects` (not narrowed to a key), decided in
    /// memory from the rows and the tree read once; a project that is not stored is left out.
    pub(crate) fn evaluations_in(&self, projects: &[ProjectId]) -> Vec<(ProjectId, Evaluation)> {
        self.policy
            .evaluations_in(self.person, &self.tree, projects)
    }

    /// A project as it is stored, whether the person sees it or not: only for what is shown after
    /// the rights were judged, never to be named on its own.
    pub(crate) fn stored(&self, project: ProjectId) -> Option<&Project> {
        self.projects.get(&project)
    }

    /// Which issues the person reads in the projects they see (SEC-06): every issue where they hold
    /// *See issues*, only the ones they reported elsewhere. Decided from the rows already in memory,
    /// so it costs no statement for a project.
    pub(crate) fn issue_sight(&self) -> IssueSight {
        let mut sight = IssueSight {
            all: Vec::new(),
            own: Vec::new(),
            person: self.person,
            with_trashed: false,
            hidden: Vec::new(),
        };
        for project in &self.visible {
            if self.allows(*project, Permission::SeeIssues) {
                sight.all.push(*project);
            } else {
                sight.own.push(*project);
            }
        }
        sight
    }

    /// Every issue of every project of the tree, whether the person sees it or not. Only for asking
    /// whether a tag is used at all, never for showing what was found.
    pub(crate) fn whole_sight(&self) -> IssueSight {
        IssueSight {
            all: self.projects.keys().copied().collect(),
            own: Vec::new(),
            person: self.person,
            with_trashed: true,
            hidden: Vec::new(),
        }
    }

    /// Whether the person sees the project.
    pub(crate) fn sees(&self, project: ProjectId) -> bool {
        self.visible.contains(&project)
    }

    /// Whether the person holds the permission in the project, from the same rows and the same
    /// engine as every other decision.
    pub(crate) fn allows(&self, project: ProjectId, permission: Permission) -> bool {
        self.holds(Place::Project(project), permission)
    }

    /// Whether the person holds the permission at the place, the whole organization included.
    pub(crate) fn holds(&self, place: Place, permission: Permission) -> bool {
        self.held_at(place)
            .is_some_and(|held| held.allows(permission))
    }

    fn held_in(&self, project: ProjectId) -> Option<std::sync::Arc<Evaluation>> {
        self.held_at(Place::Project(project))
    }

    fn held_at(&self, place: Place) -> Option<std::sync::Arc<Evaluation>> {
        self.policy
            .evaluate_as(self.person, self.key.as_ref(), &self.tree, place)
    }

    /// The projects from the top down to the place, the stored ones (empty for the whole
    /// organization): only for comparing places, never to be named.
    pub(crate) fn chain(&self, place: Place) -> Vec<ProjectId> {
        place
            .project()
            .and_then(|project| self.tree.chain(project))
            .unwrap_or_default()
    }

    /// Whether the person may know the project: the one definition of the policy
    /// (`Evaluation::knows_project`), they see it or decide who has which role there.
    pub(crate) fn knows(&self, project: ProjectId) -> bool {
        self.held_in(project)
            .is_some_and(|held| held.knows_project())
    }

    /// Whether the person sees a project that is `outside` or inside it, at any depth: a subject
    /// created in `outside` reaches a project they work in, also when they know nothing above it.
    pub(crate) fn sees_inside(&self, outside: ProjectId) -> bool {
        self.visible
            .iter()
            .any(|project| self.tree.is_within(*project, outside))
    }

    /// Every project, known or not, where the person holds the permission.
    pub(crate) fn holding(&self, permission: Permission) -> Vec<ProjectId> {
        self.projects
            .keys()
            .copied()
            .filter(|project| self.allows(*project, permission))
            .collect()
    }

    /// The place as a name the person may use: the project if they may know it, else *a project
    /// you can't see*; none for the whole organization.
    pub(crate) fn label(&self, place: Place) -> Option<ProjectLabel> {
        place.project().map(|project| {
            self.projects
                .get(&project)
                .filter(|found| self.knows(found.id))
                .map_or(ProjectLabel::Hidden, ProjectLabel::of)
        })
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
        self.get(project).map(ProjectRef::from)
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
        self.tree.is_within(project.id, outside)
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
            trail.push(ProjectRef::from(found));
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
