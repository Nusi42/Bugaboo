//! The tree of projects and everything the engine reads, as plain data, so a decision can be made
//! for the tree as it is and for the tree as it would be after a move (PRJ-03, M2-F23).
//!
//! [`evaluate`] decides for one person at one place from the rows of that place. A screen that
//! lists the projects of a person, and the preview of a move, need the same decision for many
//! projects and for a tree that does not exist yet: [`Policy`] holds the rows, roles and levels of
//! the whole organization (or of one person), [`Tree`] the parent of every project, and
//! [`Policy::evaluate_at`] puts the two together into the very same [`Evaluation`] that a request
//! gets, so a list, a preview and a request can never disagree about who may see what.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use bb_core::access::{Assignment, GroupMember};
use bb_core::fields::FieldKey;
use bb_core::id::{GroupId, ProjectId, RoleId, UserId};
use bb_core::offer::reaches;
use bb_core::people_kinds::PeopleKind;
use bb_core::project::{DEPTH_MAX, Project};

use crate::evaluate::{Evaluation, Inputs, Place, RoleData, Row, Who, evaluate};
use crate::levels::LevelRules;
use crate::narrowing::Narrowing;

/// The tree of projects: the parent of each project, or `None` at the top level.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tree {
    parents: BTreeMap<ProjectId, Option<ProjectId>>,
}

impl Tree {
    /// The tree made of `(project, parent)` pairs.
    #[must_use]
    pub fn new(links: impl IntoIterator<Item = (ProjectId, Option<ProjectId>)>) -> Self {
        Self {
            parents: links.into_iter().collect(),
        }
    }

    /// The tree of these stored projects.
    #[must_use]
    pub fn of<'a>(projects: impl IntoIterator<Item = &'a Project>) -> Self {
        Self::new(
            projects
                .into_iter()
                .map(|project| (project.id, project.parent)),
        )
    }

    /// Whether the project is in the tree.
    #[must_use]
    pub fn contains(&self, project: ProjectId) -> bool {
        self.parents.contains_key(&project)
    }

    /// Every project, in the order of their IDs.
    pub fn projects(&self) -> impl Iterator<Item = ProjectId> + '_ {
        self.parents.keys().copied()
    }

    /// The parent of a project: `None` for a project that is not in the tree, `Some(None)` for
    /// one at the top level.
    #[must_use]
    pub fn parent(&self, project: ProjectId) -> Option<Option<ProjectId>> {
        self.parents.get(&project).copied()
    }

    /// The projects from the top down to `project`; `None` when it is not in the tree or the
    /// chain is not a chain (a loop, or deeper than any tree can be).
    #[must_use]
    pub fn chain(&self, project: ProjectId) -> Option<Vec<ProjectId>> {
        let mut chain = vec![project];
        let mut current = self.parent(project)?;
        while let Some(parent) = current {
            if chain.len() >= DEPTH_MAX || chain.contains(&parent) {
                return None;
            }
            chain.push(parent);
            current = self.parent(parent)?;
        }
        chain.reverse();
        Some(chain)
    }

    /// Whether `project` is `ancestor` or inside it, at any depth.
    #[must_use]
    pub fn is_within(&self, project: ProjectId, ancestor: ProjectId) -> bool {
        self.chain(project)
            .is_some_and(|chain| chain.contains(&ancestor))
    }

    /// Whether `project` or something inside it is nested deeper than [`DEPTH_MAX`] levels, or
    /// not in the tree at all.
    #[must_use]
    pub fn too_deep_below(&self, project: ProjectId) -> bool {
        self.subtree(project)
            .into_iter()
            .any(|inside| self.chain(inside).is_none())
    }

    /// `project` and every project inside it, at any depth, the project first.
    #[must_use]
    pub fn subtree(&self, project: ProjectId) -> Vec<ProjectId> {
        let mut found = vec![project];
        let mut at = 0;
        while let Some(current) = found.get(at).copied() {
            at += 1;
            for (child, parent) in &self.parents {
                if *parent == Some(current) && !found.contains(child) {
                    found.push(*child);
                }
            }
        }
        found
    }

    /// The projects directly inside `parent` (the top level for `None`), in the order of their IDs.
    pub fn children(&self, parent: Option<ProjectId>) -> impl Iterator<Item = ProjectId> + '_ {
        self.parents
            .iter()
            .filter(move |(_, known)| **known == parent)
            .map(|(child, _)| *child)
    }

    /// The tree as it would be with `project` inside `parent`.
    #[must_use]
    pub fn moved(&self, project: ProjectId, parent: Option<ProjectId>) -> Self {
        let mut parents = self.parents.clone();
        parents.insert(project, parent);
        Self { parents }
    }
}

/// What the engine reads: who has which role where, the groups, the roles and the levels they
/// set. The store loads it for one person (the rows of the person and of their groups) or for
/// everybody.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Policy {
    /// The rows of *Who has which role*.
    pub assignments: Vec<Assignment>,
    /// Who is in which group.
    pub members: Vec<GroupMember>,
    /// The roles the rows name.
    pub roles: Vec<RoleData>,
    /// The field levels and the people levels of the roles, for the whole organization and for
    /// projects.
    pub rules: LevelRules,
}

impl Policy {
    /// The rows that are the person's own or one of their groups'.
    #[must_use]
    pub fn rows_of(&self, person: UserId) -> Vec<Row> {
        let groups: BTreeSet<GroupId> = self
            .members
            .iter()
            .filter(|member| member.person == person)
            .map(|member| member.group)
            .collect();
        self.assignments
            .iter()
            .filter_map(|row| {
                let who = if row.person == Some(person) {
                    Who::Person
                } else {
                    Who::Group(row.group.filter(|group| groups.contains(group))?)
                };
                Some(Row {
                    id: row.id,
                    who,
                    role: row.role,
                    project: row.project,
                    applies_to: row.applies_to,
                })
            })
            .collect()
    }

    /// The people who hold the rows that `matching` picks: their own, or those of a group they
    /// are in.
    fn holders(&self, matching: impl Fn(&Assignment) -> bool) -> BTreeSet<UserId> {
        let mut people = BTreeSet::new();
        for row in self.assignments.iter().filter(|row| matching(row)) {
            people.extend(row.person);
            if let Some(group) = row.group {
                people.extend(
                    self.members
                        .iter()
                        .filter(|member| member.group == group)
                        .map(|member| member.person),
                );
            }
        }
        people
    }

    /// Every person whose evaluation can change when what is on `projects` is no longer, or is
    /// newly, above a project: those who have a row on one of them (their own or a group's),
    /// and those who hold a role that sets a field level or a people level on one of them, which
    /// the projects below inherit.
    #[must_use]
    pub fn people_affected_by(&self, projects: &BTreeSet<ProjectId>) -> BTreeSet<UserId> {
        let on_projects =
            |project: Option<ProjectId>| project.is_some_and(|project| projects.contains(&project));
        let ruled: BTreeSet<RoleId> = self
            .rules
            .of::<FieldKey>()
            .iter()
            .filter(|rule| on_projects(rule.project))
            .map(|rule| rule.role)
            .chain(
                self.rules
                    .typed
                    .iter()
                    .filter(|rule| on_projects(rule.project))
                    .map(|rule| rule.role),
            )
            .chain(
                self.rules
                    .of::<PeopleKind>()
                    .iter()
                    .filter(|rule| on_projects(rule.project))
                    .map(|rule| rule.role),
            )
            .collect();
        self.holders(|row| {
            row.project
                .is_some_and(|project| projects.contains(&project))
                || row.role.is_some_and(|role| ruled.contains(&role))
        })
    }

    /// Every person who has a row, or is in a group that has one, anywhere.
    #[must_use]
    pub fn people(&self) -> BTreeSet<UserId> {
        self.holders(|_| true)
    }

    /// Everything the engine needs to decide for `person` at `place`, whose projects from the top
    /// down are `chain`: their rows (their own and their groups') on the organization and on the
    /// projects of the chain, the roles those name, and the levels of those roles. The one
    /// definition of "which rows and rules apply at a place"; the store loads its data and asks
    /// it, and so does the test store.
    #[must_use]
    pub fn inputs_for(&self, person: UserId, place: Place, chain: Vec<ProjectId>) -> Inputs {
        self.inputs_from_rows(person, &self.rows_of(person), place, chain)
    }

    /// [`Policy::inputs_for`] with the rows of the person made once by [`Policy::rows_of`], for a
    /// person who is asked about many places.
    #[must_use]
    pub fn inputs_from_rows(
        &self,
        person: UserId,
        rows: &[Row],
        place: Place,
        chain: Vec<ProjectId>,
    ) -> Inputs {
        let rows: Vec<Row> = rows
            .iter()
            .filter(|row| reaches(row.project, &chain))
            .copied()
            .collect();
        let role_ids: BTreeSet<RoleId> = rows.iter().filter_map(|row| row.role).collect();
        Inputs {
            person,
            place,
            rows,
            roles: self
                .roles
                .iter()
                .filter(|role| role_ids.contains(&role.id))
                .cloned()
                .collect(),
            rules: self.rules.at(&role_ids, &chain),
            chain,
        }
    }

    /// What `person` may do at `place` in `tree`, from the rows `rows` (made by
    /// [`Policy::rows_of`] once for a person who is asked about many places). `None` when the
    /// project is not in the tree.
    #[must_use]
    pub fn evaluate_with(
        &self,
        person: UserId,
        rows: &[Row],
        tree: &Tree,
        place: Place,
    ) -> Option<Evaluation> {
        let chain = match place {
            Place::Organization => Vec::new(),
            Place::Project(project) => tree.chain(project)?,
        };
        Some(evaluate(&self.inputs_from_rows(person, rows, place, chain)))
    }

    /// What `person` may do at `place` in `tree`.
    #[must_use]
    pub fn evaluate_at(&self, person: UserId, tree: &Tree, place: Place) -> Option<Evaluation> {
        self.evaluate_with(person, &self.rows_of(person), tree, place)
    }

    /// What the actor `person` may do at `place` in `tree`: their own evaluation, narrowed to the
    /// access key they came with, if they came with one (ADR-0025, decision 8, M2-F27). A key
    /// never widens anything, so this is the only way to judge what **the actor** may see or do;
    /// [`Policy::evaluate_at`] is for judging other people (the preview of a move).
    #[must_use]
    pub fn evaluate_as(
        &self,
        person: UserId,
        key: Option<&Narrowing>,
        tree: &Tree,
        place: Place,
    ) -> Option<Arc<Evaluation>> {
        self.evaluate_at(person, tree, place)
            .map(|held| Arc::new(held).narrowed_by(key))
    }

    /// The projects of the tree that `person` sees, in the order of their IDs.
    #[must_use]
    pub fn visible(&self, person: UserId, tree: &Tree) -> BTreeSet<ProjectId> {
        self.visible_as(person, None, tree)
    }

    /// The projects of the tree that the actor `person` sees, narrowed to the access key they came
    /// with, if any: a key for a project sees that project and what is inside it, nothing above
    /// it and nothing beside it (M2-F27).
    #[must_use]
    pub fn visible_as(
        &self,
        person: UserId,
        key: Option<&Narrowing>,
        tree: &Tree,
    ) -> BTreeSet<ProjectId> {
        let rows = self.rows_of(person);
        tree.projects()
            .filter(|project| {
                self.evaluate_with(person, &rows, tree, Place::Project(*project))
                    .is_some_and(|found| Arc::new(found).narrowed_by(key).sees_project())
            })
            .collect()
    }
}
