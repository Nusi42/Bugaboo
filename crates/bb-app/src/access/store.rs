//! The port of the policy engine and the management use cases: what they need from storage.
//!
//! Like [`AuthStore`](crate::auth::AuthStore), reads take `&self`, and **writes** are associated
//! functions that take the open transaction and are called from inside `record_change` of
//! [`ChangeStore`], so a change and its domain event are committed together (ARCH-EVENTS).
//! `policy_inputs` loads everything the engine needs for one person at one place (inside a change,
//! and for the records that have no request); a request reads `person_policy` and `project_levels`
//! once and the engine chooses what applies at each place (SEC-03, M4-F137). The engine itself
//! does no I/O.

use std::collections::BTreeSet;
use std::future::Future;

use bb_core::access::{Assignment, FieldLevel, Group, GroupMember, PeopleLevel, Role};
use bb_core::fields::FieldKey;
use bb_core::id::{AssignmentId, GroupId, IssueTypeId, ProjectId, RoleId, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_core::project::{Project, ShortName};
use bb_core::version::Version;
use bb_policy::guards::OrganizationState;
use bb_policy::{Inputs, LevelRules, LevelSubject, Permission, Place, Policy, RoleData};
use jiff::Timestamp;

use crate::ports::ChangeStore;
use crate::store_macros::{queries, reads, writes};

/// A role with its permissions, as stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleRecord {
    /// The role.
    pub role: Role,
    /// What it allows.
    pub permissions: BTreeSet<Permission>,
}

impl From<RoleRecord> for RoleData {
    fn from(record: RoleRecord) -> Self {
        Self {
            id: record.role.id,
            name: record.role.name,
            position: record.role.position,
            permissions: record.permissions,
            two_step: record.role.two_step,
        }
    }
}

impl From<&RoleRecord> for RoleData {
    fn from(record: &RoleRecord) -> Self {
        record.clone().into()
    }
}

/// The levels roles have set, for the organization and for the projects from the top down to a
/// place: what the tables of what roles see are made from, with one read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaceLevels {
    /// The projects from the top down to the place; empty for the organization.
    pub chain: Vec<ProjectId>,
    /// The levels of the roles asked about, their own and those set for the projects of the chain.
    pub rules: LevelRules,
}

/// What the policy engine and its management use cases need from storage.
pub trait PolicyStore: ChangeStore + 'static {
    queries! {
        /// Everything the engine needs for `person` at `place`: their rows and their groups' rows
        /// for the organization and for the project and its parents, the roles those rows name
        /// and the levels of those roles. `None` when the project does not exist. Inside a
        /// change, a rule that depends on what a person may do (no escalation) is judged on what
        /// is stored now, after the lock.
        fn policy_inputs(person: UserId, place: Place) -> Option<Inputs> = policy_inputs_in;
        /// The rows for the whole organization that give a role, with the roles and the members
        /// of groups: what the last-administrator rule needs.
        fn organization_state() -> OrganizationState = organization_state_in;
        /// Whether the starting roles were made (they are made once; two servers that start
        /// together must not both make them).
        fn starting_roles_made() -> bool = starting_roles_made_in;
        /// One role with its permissions.
        fn role(id: RoleId) -> Option<RoleRecord> = role_in;
        /// The rows on a project (`Some`) or for the whole organization (`None`), in the order
        /// they were made.
        fn assignments_on(project: Option<ProjectId>) -> Vec<Assignment> = assignments_on_in;
        /// The rows of a group, wherever they are.
        fn assignments_of_group(group: GroupId) -> Vec<Assignment> = assignments_of_group_in;
        /// The rows that give a role, wherever they are.
        fn assignments_of_role(role: RoleId) -> Vec<Assignment> = assignments_of_role_in;
        /// One project.
        fn project(id: ProjectId) -> Option<Project> = project_in;
    }

    reads! {
        /// What the engine needs to decide for `person` at the organization: their rows (their
        /// own and their groups') wherever they are, the roles those name and the levels those
        /// roles have of their own, as one snapshot. Which of it applies at a place is decided
        /// by the engine (`Policy::inputs_from_rows`), so a request reads it once and decides
        /// every place from it.
        fn person_policy(person: UserId) -> Policy;
        /// The levels these roles have set for projects, for all projects: what a decision at a
        /// project needs besides `person_policy`, read once for a request that asks about one.
        fn project_levels(roles: Vec<RoleId>) -> LevelRules;
        /// The people who were administrators in M1 and have not got the role yet.
        fn legacy_administrators() -> Vec<UserId>;
        /// Every role with its permissions, in the order of roles.
        fn roles() -> Vec<RoleRecord>;
        /// Every group, by name.
        fn groups() -> Vec<Group>;
        /// One group.
        fn group(id: GroupId) -> Option<Group>;
        /// The people in a group, in the order they were put there.
        fn group_members(group: GroupId) -> Vec<GroupMember>;
        /// The rows of a person (their own, not their groups'), wherever they are.
        fn assignments_of_person(person: UserId) -> Vec<Assignment>;
        /// The groups a person is in, by name.
        fn groups_of(person: UserId) -> Vec<Group>;
        /// One row.
        fn assignment(id: AssignmentId) -> Option<Assignment>;
        /// The projects from the top down to this one; `None` when it does not exist.
        fn project_chain(id: ProjectId) -> Option<Vec<ProjectId>>;
        /// The project with this short name.
        fn project_by_short_name(name: &ShortName) -> Option<Project>;
        /// Every level the roles have set, their own and those for the projects from the top down
        /// to `project`; `None` when the project does not exist.
        fn level_rules(roles: Vec<RoleId>, project: Option<ProjectId>) -> Option<PlaceLevels>;
        /// Every level these roles have set, for the organization and for all projects: what a
        /// decision over many projects at once is made from, in memory, for a view that was read
        /// without them (the groups of labels in use).
        fn level_rules_of_roles(roles: Vec<RoleId>) -> LevelRules;
    }

    writes! {
        /// Takes the lock that makes changes of roles and rows wait for each other, so two
        /// changes that each leave one administrator cannot leave none. A no-op where the
        /// database has one writer anyway.
        fn lock_policy() -> ();
        /// Marks the starting roles as made.
        fn mark_starting_roles_made(at: Timestamp) -> ();
        /// Forgets a person of the list of M1 administrators.
        fn delete_legacy_administrator(person: UserId) -> ();
        /// Adds a role with its permissions and its levels.
        fn insert_role(
            role: &Role,
            permissions: &[Permission],
            fields: &[(FieldKey, FieldLevel)],
            people: &[(PeopleKind, PeopleLevel)]
        ) -> ();
        /// Changes the name and the two-step switch of a role, counts its version up, and says
        /// whether the role was still at `version`.
        fn update_role(id: RoleId, name: &str, two_step: bool, version: Version) -> bool;
        /// Replaces the permissions of a role.
        fn replace_role_permissions(id: RoleId, permissions: &[Permission]) -> ();
        /// Replaces the levels a role has of its own (not those set for projects).
        fn replace_role_levels(
            id: RoleId,
            fields: &[(FieldKey, FieldLevel)],
            people: &[(PeopleKind, PeopleLevel)]
        ) -> ();
        /// Removes a role, with the rows that give it and the levels it set.
        fn delete_role(id: RoleId) -> bool;
        /// Adds a group.
        fn insert_group(group: &Group) -> ();
        /// Renames a group, counts its version up, and says whether the group was still at
        /// `version`.
        fn rename_group(id: GroupId, name: &str, version: Version) -> bool;
        /// Removes a group, with its members and its rows.
        fn delete_group(id: GroupId) -> bool;
        /// Puts a person into a group.
        fn insert_group_member(member: &GroupMember) -> ();
        /// Takes a person out of a group.
        fn delete_group_member(group: GroupId, person: UserId) -> bool;
        /// Adds a row of *Who has which role*.
        fn insert_assignment(assignment: &Assignment) -> ();
        /// Removes a row.
        fn delete_assignment(id: AssignmentId) -> bool;
        /// Adds a project.
        fn insert_project(project: &Project) -> ();
    }

    /// The levels a role has set for the subjects of one kind (the fields, or the kinds of people
    /// information): in a project (`Some`) or its own (`None`), for all issue types or, for
    /// fields, for one.
    fn levels<K: LevelSubject>(
        &self,
        role: RoleId,
        project: Option<ProjectId>,
        issue_type: Option<IssueTypeId>,
    ) -> impl Future<Output = Result<Vec<(K, K::Level)>, Self::Error>> + Send;

    /// Sets (`Some`) or takes away (`None`) the level of a subject for a role, in a project or
    /// the role's own, for all issue types or, for fields, for one.
    fn put_level<K: LevelSubject>(
        transaction: &mut Self::Transaction,
        role: RoleId,
        project: Option<ProjectId>,
        issue_type: Option<IssueTypeId>,
        subject: K,
        level: Option<K::Level>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}
