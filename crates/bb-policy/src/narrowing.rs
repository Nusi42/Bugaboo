//! Narrowing for access keys and, later, tokens of AI apps (ADR-0025 decision 8, AUTH-07): a
//! key never widens anything. At every use its permissions are the owner's evaluation at that
//! place intersected with the key's choice and projects; the key follows its owner's roles at
//! once because the owner's evaluation is made again for every request.
//!
//! WP-09 builds the keys; the engine already intersects, so a key cannot hold more than its
//! owner whatever scope list it was given.

use std::collections::BTreeSet;
use std::sync::Arc;

use bb_core::access::FieldLevel;
use bb_core::id::{AgentId, ProjectId};

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

/// Who is acting, as the policy tells the kinds apart (ADR-0029, decision 1): one type, so a use
/// case cannot judge an agent as its person by mistake. For an agent and for a person via an app
/// the person is the one whose rights are narrowed; the actor never holds more than they do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActorKind {
    /// A person, by a session or by an access key that narrows them.
    Person,
    /// A person via an AI app (an OAuth grant, or a key an AI app uses): the person, narrowed by
    /// the scope, shown with the app's name.
    PersonViaApp {
        /// The name of the app, as the person saw it at the consent step.
        app: String,
    },
    /// An agent: its responsible person's evaluation, intersected with its work and its limits.
    Agent {
        /// The agent.
        agent: AgentId,
    },
}

impl ActorKind {
    /// Whether the actor is an agent, which judges by its work and never holds a permission in
    /// [`crate::NOT_FOR_AGENTS`].
    #[must_use]
    pub const fn is_agent(&self) -> bool {
        matches!(self, Self::Agent { .. })
    }
}

/// What an access key narrows an evaluation to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Narrowing {
    choice: Choice,
    /// Set for an agent: it keeps no permission of `NOT_FOR_AGENTS`, and counts only where its
    /// person holds *Create agents that work for you*.
    agent: bool,
    /// For an agent: the projects where agents are switched off (the nearest setting in the chain
    /// of the project says so), whatever its work names.
    off: BTreeSet<ProjectId>,
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
            agent: false,
            off: BTreeSet::new(),
            projects: None,
        }
    }

    /// The narrowing of an agent whose work is in these projects (and their sub-projects): what
    /// its person may do there, as *See and change*, without any permission of
    /// [`crate::NOT_FOR_AGENTS`], and only where the person holds *Create agents that work for
    /// you* (M7-Q1), and not in a project (or below it) where agents are switched off (`off`). An
    /// agent with no work counts nowhere (ADR-0029, decision 2).
    #[must_use]
    pub fn agent(work: BTreeSet<ProjectId>, off: BTreeSet<ProjectId>) -> Self {
        Self {
            agent: true,
            off,
            ..Self::see_and_change().in_projects(work)
        }
    }

    /// Whether this narrows an agent.
    #[must_use]
    pub const fn is_agent(&self) -> bool {
        self.agent
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
            && (!self.agent || permission.agents_may_hold())
            && match &self.choice {
                Choice::Kinds(kinds) => kinds.contains(&permission.kind()),
                Choice::Permissions(permissions) => permissions.contains(&permission),
            }
    }

    /// Whether the key can change anything: if not, every field is at most *Can see*, and a use
    /// case that writes something of the owner's own (their settings) refuses the key.
    #[must_use]
    pub fn may_change(&self) -> bool {
        match &self.choice {
            Choice::Kinds(kinds) => kinds.contains(&Kind::Change) || kinds.contains(&Kind::Manage),
            Choice::Permissions(permissions) => permissions
                .iter()
                .any(|permission| permission.keys_may_hold() && permission.kind() != Kind::See),
        }
    }

    /// Whether the key counts at the place of an evaluation.
    fn counts_at(&self, place: Place, chain: &[ProjectId]) -> bool {
        if self.agent && matches!(place, Place::Project(project) if self.off.contains(&project)) {
            return false;
        }
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
    /// The evaluation as the actor has it: narrowed to the access key they came with, if any. The
    /// one place that decides whether to narrow (ADR-0025, decision 8, M2-F27). It takes and gives
    /// a shared evaluation, so that without a key nothing is copied.
    #[must_use]
    pub fn narrowed_by(self: Arc<Self>, key: Option<&Narrowing>) -> Arc<Self> {
        match key {
            Some(key) => Arc::new(self.narrowed(key)),
            None => self,
        }
    }

    /// The evaluation narrowed to an access key: a subset of what the owner may do here, and no
    /// field above *Can see* for a key that may only see.
    #[must_use]
    pub fn narrowed(&self, key: &Narrowing) -> Self {
        // An agent counts only where its person holds *Create agents that work for you*.
        let outside_projects = !key.counts_at(self.place, &self.chain)
            || (key.agent && !self.permissions.contains(&Permission::CreateAgents));
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
            narrowed.fields.close();
            narrowed.people.close();
            narrowed.field_limit = FieldLevel::Hidden;
        } else if !key.may_change() {
            narrowed.fields.cap(FieldLevel::CanSee);
            narrowed.field_limit = narrowed.field_limit.min(FieldLevel::CanSee);
        }
        narrowed.narrowing = Some(Narrowed {
            removed,
            outside_projects,
        });
        narrowed
    }
}
