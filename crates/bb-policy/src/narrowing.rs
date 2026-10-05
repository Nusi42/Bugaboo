//! Narrowing for access keys and, later, tokens of AI apps (ADR-0025 decision 8, AUTH-07): a
//! key never widens anything. At every use its permissions are the owner's evaluation at that
//! place intersected with the key's choice and projects; the key follows its owner's roles at
//! once because the owner's evaluation is made again for every request.
//!
//! WP-09 builds the keys; the engine already intersects, so a key cannot hold more than its
//! owner whatever scope list it was given.

use std::collections::BTreeSet;

use bb_core::access::FieldLevel;
use bb_core::id::ProjectId;

use crate::catalogue::{Kind, Permission};
use crate::evaluate::{Evaluation, Place};

/// *What it may do* of an access key (M2-Q6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Choice {
    /// The permissions of these kinds: *See only* is `See`, *See and change* is `See` and
    /// `Change`, *Everything I can do* is all three.
    Kinds(BTreeSet<Kind>),
    /// Single permissions, under *More options*.
    Permissions(BTreeSet<Permission>),
}

/// What an access key narrows an evaluation to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Narrowing {
    choice: Choice,
    /// The projects the key counts in (and their sub-projects), by ID so a move changes nothing;
    /// `None` for no list. A key with projects gives nothing for the whole organization.
    projects: Option<BTreeSet<ProjectId>>,
}

/// What narrowing took away, for *Check access*.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Narrowed {
    /// The permissions of the owner that the key does not keep.
    pub(crate) removed: BTreeSet<Permission>,
    /// The key does not count at this place at all (outside its projects).
    pub(crate) outside_projects: bool,
}

impl Narrowing {
    /// A key with this choice, for every project of its owner.
    #[must_use]
    pub const fn new(choice: Choice) -> Self {
        Self {
            choice,
            projects: None,
        }
    }

    /// *See only*.
    #[must_use]
    pub fn see_only() -> Self {
        Self::new(Choice::Kinds(BTreeSet::from([Kind::See])))
    }

    /// *See and change*.
    #[must_use]
    pub fn see_and_change() -> Self {
        Self::new(Choice::Kinds(BTreeSet::from([Kind::See, Kind::Change])))
    }

    /// *Everything I can do*.
    #[must_use]
    pub fn everything() -> Self {
        Self::new(Choice::Kinds(Kind::ALL.iter().copied().collect()))
    }

    /// The same key, counting only in these projects and their sub-projects.
    #[must_use]
    pub fn in_projects(mut self, projects: BTreeSet<ProjectId>) -> Self {
        self.projects = Some(projects);
        self
    }

    /// Whether the key keeps the permission: never one that keys do not hold (`NOT_FOR_KEYS`),
    /// whatever the choice, and otherwise what the choice says.
    fn keeps(&self, permission: Permission) -> bool {
        permission.keys_may_hold()
            && match &self.choice {
                Choice::Kinds(kinds) => kinds.contains(&permission.kind()),
                Choice::Permissions(permissions) => permissions.contains(&permission),
            }
    }

    /// Whether the key can change anything: if not, every field is at most *Can see*.
    fn may_change(&self) -> bool {
        match &self.choice {
            Choice::Kinds(kinds) => kinds.contains(&Kind::Change) || kinds.contains(&Kind::Manage),
            Choice::Permissions(permissions) => permissions
                .iter()
                .any(|permission| permission.keys_may_hold() && permission.kind() != Kind::See),
        }
    }

    /// Whether the key counts at the place of an evaluation.
    fn counts_at(&self, place: Place, chain: &[ProjectId]) -> bool {
        match (&self.projects, place) {
            (None, _) => true,
            (Some(_), Place::Organization) => false,
            (Some(projects), Place::Project(_)) => {
                chain.iter().any(|project| projects.contains(project))
            }
        }
    }
}

impl Evaluation {
    /// The evaluation narrowed to an access key: a subset of what the owner may do here, and no
    /// field above *Can see* for a key that may only see.
    #[must_use]
    pub fn narrowed(&self, key: &Narrowing) -> Self {
        let outside_projects = !key.counts_at(self.place, &self.chain);
        let kept: BTreeSet<Permission> = if outside_projects {
            BTreeSet::new()
        } else {
            self.permissions
                .iter()
                .copied()
                .filter(|permission| key.keeps(*permission))
                .collect()
        };
        let removed = self.permissions.difference(&kept).copied().collect();
        let mut narrowed = self.clone();
        narrowed.permissions = kept;
        if outside_projects {
            // A key outside its projects is not here at all: nothing is shown either.
            for level in narrowed.fields.values_mut() {
                *level = FieldLevel::Hidden;
            }
            for level in narrowed.people.values_mut() {
                *level = bb_core::access::PeopleLevel::Hidden;
            }
        } else if !key.may_change() {
            for level in narrowed.fields.values_mut() {
                *level = (*level).min(FieldLevel::CanSee);
            }
        }
        narrowed.narrowing = Some(Narrowed {
            removed,
            outside_projects,
        });
        narrowed
    }
}
