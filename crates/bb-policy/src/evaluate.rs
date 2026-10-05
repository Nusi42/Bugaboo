//! The evaluation (SEC-03, ADR-0025 decision 3, architecture §7 *Evaluation*): from the rows of
//! *Who has which role* that apply to a person at a place, work out what they may do there, how
//! they see fields and how they see people. No I/O: the rows come in as plain data, loaded by the
//! store through one port call (`bb-app`), and the result lives for one request.
//!
//! The steps, in the order of the architecture:
//! 1. nobody signed in reaches only public routes (the caller never builds an evaluation then),
//!    and a person without an applicable row can do nothing (the result is empty);
//! 2. the rows that apply: the actor's own rows and their groups' for the whole organization,
//!    those on a parent with *This project and its sub-projects*, and those on the project;
//! 3. every row has a rank: the depth of its project (the whole organization lowest), then the
//!    person's own row above a row of one of their groups;
//! 4. the highest-ranked *No access* removes every role row of equal or lower rank, but
//!    *Decide who has which role* from a removed row strictly above it (a project higher up, or
//!    the whole organization) is kept;
//! 5. the permissions are the union of the remaining roles, filtered by where they count;
//! 6. narrowing for an access key is [`Evaluation::narrowed`] (see `narrowing.rs`);
//! 7. the field mask and 8. the *Who sees people* levels come from the same remaining roles.

use std::collections::{BTreeMap, BTreeSet};

use bb_core::access::{AppliesTo, FieldLevel, PeopleLevel};
use bb_core::fields::Field;
use bb_core::id::{AssignmentId, GroupId, ProjectId, RoleId, UserId};
use bb_core::people_kinds::PeopleKind;

use crate::catalogue::{Counts, Permission};

/// Where something is decided: for the whole organization, or in a project.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Place {
    /// The whole organization (*Organization settings*).
    Organization,
    /// One project and, through its parents, everything above it.
    Project(ProjectId),
}

impl Place {
    /// The project, if the place is one.
    #[must_use]
    pub const fn project(self) -> Option<ProjectId> {
        match self {
            Self::Organization => None,
            Self::Project(project) => Some(project),
        }
    }
}

/// Whom a row is for, seen from the actor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Who {
    /// The actor themselves.
    Person,
    /// A group the actor is in.
    Group(GroupId),
}

/// One row of *Who has which role* that is the actor's own or one of their groups'.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    /// The ID of the row.
    pub id: AssignmentId,
    /// The actor or a group of the actor.
    pub who: Who,
    /// The role, or `None` for *No access*.
    pub role: Option<RoleId>,
    /// The project the row is on, or `None` for the whole organization.
    pub project: Option<ProjectId>,
    /// Where it counts.
    pub applies_to: AppliesTo,
}

/// A role with what the evaluation needs of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleData {
    /// The ID of the role.
    pub id: RoleId,
    /// Its name; the label of a person shown as *Role only*.
    pub name: String,
    /// The place in the order of roles.
    pub position: i64,
    /// What it allows.
    pub permissions: BTreeSet<Permission>,
    /// Whether everybody with it needs two-step sign-in.
    pub two_step: bool,
}

/// The level of a field for a role: in a project (`Some`) or the role's own (`None`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldRule {
    /// The project, or `None` for the role's own level.
    pub project: Option<ProjectId>,
    /// The role.
    pub role: RoleId,
    /// The field.
    pub field: Field,
    /// The level.
    pub level: FieldLevel,
}

/// How a role sees people of a kind: in a project (`Some`) or the role's own (`None`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeopleRule {
    /// The project, or `None` for the role's own level.
    pub project: Option<ProjectId>,
    /// The role.
    pub role: RoleId,
    /// The kind of people information.
    pub kind: PeopleKind,
    /// The level.
    pub level: PeopleLevel,
}

/// Everything the engine needs to decide for one person at one place; the store loads it with one
/// call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inputs {
    /// The actor.
    pub person: UserId,
    /// The place.
    pub place: Place,
    /// The projects from the top down to the place itself; empty for the organization.
    pub chain: Vec<ProjectId>,
    /// The rows of the actor and of their groups for the organization and for the projects of
    /// the chain (rows on other projects are left out by the store).
    pub rows: Vec<Row>,
    /// The roles the rows (and the level rules) name.
    pub roles: Vec<RoleData>,
    /// The field levels of those roles for the organization and for the projects of the chain.
    pub field_rules: Vec<FieldRule>,
    /// The people levels of those roles for the organization and for the projects of the chain.
    pub people_rules: Vec<PeopleRule>,
}

/// The rank of a row (step 3): the depth first, then the person above a group.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Rank {
    depth: usize,
    personal: bool,
}

/// What happened to a row in the evaluation, for *Check access*.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowStatus {
    /// The role of the row counts.
    Counts,
    /// A *No access* row of equal or higher rank removed the role, and a *No access* at the depth
    /// of the row removes it by itself: nothing is kept from it.
    RemovedBy(AssignmentId),
    /// A *No access* row removed the role, but the row is strictly above every *No access* that
    /// removes it by itself: *Decide who has which role* is kept from it (ADR-0025, decision 6).
    RemovedFromAbove(AssignmentId),
    /// The row is *No access* and it is the one that decides.
    NoAccessDecides,
    /// The row is *No access* but a higher-ranked one decides.
    NoAccessCovered,
}

/// A row with what the engine made of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowTrace {
    /// The row.
    pub row: Row,
    /// The depth of its project: 0 for the whole organization, 1 for a top-level project.
    pub depth: usize,
    /// Whether the row is on a project above the place (or on the organization, when the place is
    /// a project) and so reaches the place through inheritance.
    pub inherited: bool,
    /// What the engine made of it.
    pub status: RowStatus,
}

/// A role that counts at the place, with its place in the order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeldRole {
    /// The ID of the role.
    pub id: RoleId,
    /// Its name.
    pub name: String,
    /// The place in the order of roles.
    pub position: i64,
}

/// Where the level of a field or of a kind came from, for one role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LevelSource {
    /// Set for a project (the place itself or the nearest parent that set one).
    Project(ProjectId),
    /// The role's own level, for the whole organization.
    RoleOwn,
    /// Nothing is set: the level is the closed one (default deny).
    Unset,
}

/// What one role says about one field or kind of people information.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoleLevel<L> {
    /// The role.
    pub role: RoleId,
    /// Its level.
    pub level: L,
    /// Where it was set.
    pub source: LevelSource,
}

/// The result: what a person may do at a place, and how they see fields and people.
///
/// Only [`evaluate`] makes one (and [`Evaluation::none`] for nobody), so everything built from an
/// evaluation (`Authorized<T>`, `PersonRef`) rests on a decision of this crate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evaluation {
    pub(crate) person: Option<UserId>,
    pub(crate) place: Place,
    pub(crate) chain: Vec<ProjectId>,
    pub(crate) permissions: BTreeSet<Permission>,
    /// *Decide who has which role* that came only from a row that a *No access* below it removed.
    pub(crate) kept_decide_roles: bool,
    pub(crate) roles: Vec<HeldRole>,
    pub(crate) fields: BTreeMap<Field, FieldLevel>,
    pub(crate) people: BTreeMap<PeopleKind, PeopleLevel>,
    pub(crate) field_sources: BTreeMap<Field, Vec<RoleLevel<FieldLevel>>>,
    pub(crate) people_sources: BTreeMap<PeopleKind, Vec<RoleLevel<PeopleLevel>>>,
    pub(crate) trace: Vec<RowTrace>,
    /// The permissions of every role the rows name, for *Check access*.
    pub(crate) all_role_permissions: BTreeMap<RoleId, BTreeSet<Permission>>,
    pub(crate) narrowing: Option<crate::narrowing::Narrowed>,
}

/// Works out what the person may do at the place of `inputs`.
#[must_use]
pub fn evaluate(inputs: &Inputs) -> Evaluation {
    let mut traced: Vec<(Rank, RowTrace)> = inputs
        .rows
        .iter()
        .filter_map(|row| applicable(inputs, row))
        .collect();
    traced.sort_by_key(|(rank, trace)| (std::cmp::Reverse(*rank), trace.row.id));

    // Step 4: the highest-ranked *No access* removes every role row of equal or lower rank.
    let top_no_access = traced
        .iter()
        .find(|(_, trace)| trace.row.role.is_none())
        .map(|(rank, trace)| (*rank, trace.row.id));
    let no_access_ranks: Vec<Rank> = traced
        .iter()
        .filter(|(_, trace)| trace.row.role.is_none())
        .map(|(rank, _)| *rank)
        .collect();
    for (rank, trace) in &mut traced {
        match (trace.row.role, top_no_access) {
            (None, Some((_, decides))) => {
                trace.status = if trace.row.id == decides {
                    RowStatus::NoAccessDecides
                } else {
                    RowStatus::NoAccessCovered
                };
            }
            (Some(_), Some((top, decides))) if *rank <= top => {
                // A *No access* at the depth of the row that ranks as high removes it by itself,
                // so nothing is kept from it: a group's role next to the person's own *No
                // access* would otherwise let them lift it. A *No access* that ranks lower (a
                // group's, next to the person's own role) does not remove it at all.
                let removed_at_its_depth = no_access_ranks
                    .iter()
                    .any(|other| other.depth == rank.depth && other >= rank);
                trace.status = if removed_at_its_depth {
                    RowStatus::RemovedBy(decides)
                } else {
                    RowStatus::RemovedFromAbove(decides)
                };
            }
            _ => {}
        }
    }

    let roles: BTreeMap<RoleId, &RoleData> =
        inputs.roles.iter().map(|role| (role.id, role)).collect();
    let mut held: Vec<&RoleData> = Vec::new();
    let mut permissions = BTreeSet::new();
    let mut kept_decide_roles = false;
    for (_, trace) in &traced {
        let Some(role) = trace.row.role.and_then(|id| roles.get(&id).copied()) else {
            continue;
        };
        match trace.status {
            RowStatus::Counts => {
                if !held.iter().any(|already| already.id == role.id) {
                    held.push(role);
                }
                permissions.extend(
                    role.permissions
                        .iter()
                        .copied()
                        .filter(|permission| counts_here(*permission, inputs.place)),
                );
            }
            // Nobody is locked out by a project below them (ADR-0025, decision 6): the right to
            // decide is kept from a row strictly above the *No access* that removed it, the whole
            // organization included.
            RowStatus::RemovedFromAbove(_) => {
                if role.permissions.contains(&Permission::DecideRoles) {
                    kept_decide_roles = true;
                }
            }
            RowStatus::RemovedBy(_) | RowStatus::NoAccessDecides | RowStatus::NoAccessCovered => {}
        }
    }
    if kept_decide_roles {
        permissions.insert(Permission::DecideRoles);
    }
    held.sort_by_key(|role| (role.position, role.id));

    let all_role_permissions = inputs
        .roles
        .iter()
        .map(|role| (role.id, role.permissions.clone()))
        .collect();
    let (fields, field_sources) = field_mask(inputs, &held);
    let (people, people_sources) = people_levels(inputs, &held);
    Evaluation {
        person: Some(inputs.person),
        place: inputs.place,
        chain: inputs.chain.clone(),
        permissions,
        kept_decide_roles,
        roles: held
            .iter()
            .map(|role| HeldRole {
                id: role.id,
                name: role.name.clone(),
                position: role.position,
            })
            .collect(),
        fields,
        people,
        field_sources,
        people_sources,
        trace: traced.into_iter().map(|(_, trace)| trace).collect(),
        all_role_permissions,
        narrowing: None,
    }
}

/// The row with its rank, when it applies at the place of `inputs` (step 2).
fn applicable(inputs: &Inputs, row: &Row) -> Option<(Rank, RowTrace)> {
    let depth = match row.project {
        None => 0,
        Some(project) => inputs.chain.iter().position(|step| *step == project)? + 1,
    };
    let place_depth = inputs.chain.len();
    let reaches = match row.project {
        None => true,
        Some(_) if depth == place_depth => true,
        Some(_) => row.applies_to == AppliesTo::ProjectAndSubProjects,
    };
    if !reaches {
        return None;
    }
    let rank = Rank {
        depth,
        personal: row.who == Who::Person,
    };
    Some((
        rank,
        RowTrace {
            row: *row,
            depth,
            inherited: depth != place_depth || (row.project.is_none() && place_depth != 0),
            status: RowStatus::Counts,
        },
    ))
}

/// Whether a permission of a role counts at the place: an organization permission only at the
/// organization (step 5), a project permission everywhere.
pub(crate) const fn counts_here(permission: Permission, place: Place) -> bool {
    match (permission.counts(), place) {
        (Counts::Project, _) | (Counts::Organization, Place::Organization) => true,
        (Counts::Organization, Place::Project(_)) => false,
    }
}

/// The depth of a project of the chain, from 1; `None` for another project.
fn chain_depth(chain: &[ProjectId], project: ProjectId) -> Option<usize> {
    chain
        .iter()
        .position(|step| *step == project)
        .map(|at| at + 1)
}

/// The nearest rule of a role: the project of the chain closest to the place that has one, else
/// the role's own.
fn nearest<R: Copy>(
    chain: &[ProjectId],
    rules: impl Iterator<Item = (Option<ProjectId>, R)>,
) -> Option<(LevelSource, R)> {
    let mut best: Option<(usize, LevelSource, R)> = None;
    for (project, rule) in rules {
        let (depth, source) = match project {
            None => (0, LevelSource::RoleOwn),
            Some(project) => match chain_depth(chain, project) {
                Some(depth) => (depth, LevelSource::Project(project)),
                None => continue,
            },
        };
        if best.as_ref().is_none_or(|(deepest, _, _)| depth > *deepest) {
            best = Some((depth, source, rule));
        }
    }
    best.map(|(_, source, rule)| (source, rule))
}

type FieldSources = BTreeMap<Field, Vec<RoleLevel<FieldLevel>>>;
type PeopleSources = BTreeMap<PeopleKind, Vec<RoleLevel<PeopleLevel>>>;

/// Step 7: for each field, the level each remaining role sets on the nearest project of the
/// chain, else its own; the mask is the highest of them.
fn field_mask(inputs: &Inputs, held: &[&RoleData]) -> (BTreeMap<Field, FieldLevel>, FieldSources) {
    let mut levels = BTreeMap::new();
    let mut sources: FieldSources = BTreeMap::new();
    for field in Field::ALL.iter().copied() {
        let mut highest = FieldLevel::Hidden;
        let mut per_role = Vec::new();
        for role in held {
            let rules = inputs
                .field_rules
                .iter()
                .filter(|rule| rule.role == role.id && rule.field == field)
                .map(|rule| (rule.project, rule.level));
            let (source, level) =
                nearest(&inputs.chain, rules).unwrap_or((LevelSource::Unset, FieldLevel::Hidden));
            highest = highest.max(level);
            per_role.push(RoleLevel {
                role: role.id,
                level,
                source,
            });
        }
        levels.insert(field, highest);
        sources.insert(field, per_role);
    }
    (levels, sources)
}

/// Step 8: the same for *Who sees people*; the most open level of the remaining roles counts.
fn people_levels(
    inputs: &Inputs,
    held: &[&RoleData],
) -> (BTreeMap<PeopleKind, PeopleLevel>, PeopleSources) {
    let mut levels = BTreeMap::new();
    let mut sources: PeopleSources = BTreeMap::new();
    for kind in PeopleKind::ALL.iter().copied() {
        let mut highest = PeopleLevel::Hidden;
        let mut per_role = Vec::new();
        for role in held {
            let rules = inputs
                .people_rules
                .iter()
                .filter(|rule| rule.role == role.id && rule.kind == kind)
                .map(|rule| (rule.project, rule.level));
            let (source, level) =
                nearest(&inputs.chain, rules).unwrap_or((LevelSource::Unset, PeopleLevel::Hidden));
            highest = highest.max(level);
            per_role.push(RoleLevel {
                role: role.id,
                level,
                source,
            });
        }
        levels.insert(kind, highest);
        sources.insert(kind, per_role);
    }
    (levels, sources)
}

impl Evaluation {
    /// Whether the permission counts at the place of this evaluation at all (an organization
    /// permission counts only at the organization).
    #[must_use]
    pub const fn counts_in_this_place(&self, permission: Permission) -> bool {
        counts_here(permission, self.place)
    }

    /// The person the evaluation is for.
    #[must_use]
    pub const fn person(&self) -> Option<UserId> {
        self.person
    }

    /// The place.
    #[must_use]
    pub const fn place(&self) -> Place {
        self.place
    }

    /// The permissions that count at the place.
    #[must_use]
    pub const fn permissions(&self) -> &BTreeSet<Permission> {
        &self.permissions
    }

    /// Whether the permission counts at the place.
    #[must_use]
    pub fn allows(&self, permission: Permission) -> bool {
        self.permissions.contains(&permission)
    }

    /// Whether the person sees the project (and what is in it).
    #[must_use]
    pub fn sees_project(&self) -> bool {
        self.allows(Permission::SeeProjects)
    }

    /// Whether the person knows that the project exists and may see its name and short name:
    /// they see it, or they decide who has which role there (and so can take a *No access*
    /// away again).
    #[must_use]
    pub fn knows_project(&self) -> bool {
        self.sees_project() || self.allows(Permission::DecideRoles)
    }

    /// Whether *Decide who has which role* is only here because it was kept from a row that
    /// *No access* removed (ADR-0025, decision 6).
    #[must_use]
    pub const fn decide_roles_was_kept(&self) -> bool {
        self.kept_decide_roles
    }

    /// The roles that count at the place, in the order of roles.
    #[must_use]
    pub fn roles(&self) -> &[HeldRole] {
        &self.roles
    }

    /// The label of the first role in the order of roles: how the person is shown as
    /// *Role only*.
    #[must_use]
    pub fn first_role_name(&self) -> Option<&str> {
        self.roles.first().map(|role| role.name.as_str())
    }

    /// The level of a field (the highest of the roles; at most *Can see* for an access key that
    /// may only see).
    #[must_use]
    pub fn field_level(&self, field: Field) -> FieldLevel {
        self.fields
            .get(&field)
            .copied()
            .unwrap_or(FieldLevel::Hidden)
    }

    /// The raw level for a kind of people information (the most open of the roles).
    #[must_use]
    pub fn kind_level(&self, kind: PeopleKind) -> PeopleLevel {
        self.people
            .get(&kind)
            .copied()
            .unwrap_or(PeopleLevel::Hidden)
    }

    /// The rows of the evaluation with what was made of each, strongest first.
    #[must_use]
    pub fn trace(&self) -> &[RowTrace] {
        &self.trace
    }

    /// The projects from the top down to the place.
    #[must_use]
    pub fn chain(&self) -> &[ProjectId] {
        &self.chain
    }
}
