//! The port of the policy engine and the management use cases: what they need from storage.
//!
//! Like [`AuthStore`](crate::auth::AuthStore), reads take `&self`, and **writes** are associated
//! functions that take the open transaction and are called from inside `record_change` of
//! [`ChangeStore`], so a change and its domain event are committed together (ARCH-EVENTS).
//! `policy_inputs` is the one call that loads everything the engine needs for one person at one
//! place (SEC-03); the engine itself does no I/O.

use std::collections::BTreeSet;
use std::future::Future;

use bb_core::access::{Assignment, FieldLevel, Group, GroupMember, PeopleLevel, Role};
use bb_core::fields::Field;
use bb_core::id::{AssignmentId, GroupId, ProjectId, RoleId, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_core::project::{Project, ShortName};
use bb_policy::guards::OrganizationState;
use bb_policy::{Inputs, Permission, Place};
use jiff::Timestamp;

use crate::ports::ChangeStore;
use crate::store_macros::{reads, writes};

/// A role with its permissions, as stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleRecord {
    /// The role.
    pub role: Role,
    /// What it allows.
    pub permissions: BTreeSet<Permission>,
}

/// A person as a name in a list to choose from: the ID and the name, never the e-mail address.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersonName {
    /// The ID of the person.
    pub id: UserId,
    /// The name the person is called.
    pub display_name: String,
}

/// What the policy engine and its management use cases need from storage.
pub trait PolicyStore: ChangeStore + 'static {
    reads! {
        /// Everything the engine needs for `person` at `place`: their rows and their groups' rows
        /// for the organization and for the project and its parents, the roles those rows name
        /// and the levels of those roles. `None` when the project does not exist.
        fn policy_inputs(person: UserId, place: Place) -> Option<Inputs>;
        /// The rows for the whole organization that give a role, with the roles and the members
        /// of groups: what the last-administrator rule needs.
        fn organization_state() -> OrganizationState;
        /// Whether the starting roles were made (they are made once).
        fn starting_roles_made() -> bool;
        /// The people who were administrators in M1 and have not got the role yet.
        fn legacy_administrators() -> Vec<UserId>;
        /// Every role with its permissions, in the order of roles.
        fn roles() -> Vec<RoleRecord>;
        /// One role with its permissions.
        fn role(id: RoleId) -> Option<RoleRecord>;
        /// Everybody who can sign in, with the name they are called, in no particular order: what
        /// *Check access* chooses a person from (M2-F91).
        fn people_names() -> Vec<PersonName>;
        /// Every group, by name.
        fn groups() -> Vec<Group>;
        /// One group.
        fn group(id: GroupId) -> Option<Group>;
        /// The people in a group, in the order they were put there.
        fn group_members(group: GroupId) -> Vec<GroupMember>;
        /// The rows on a project (`Some`) or for the whole organization (`None`), in the order
        /// they were made.
        fn assignments_on(project: Option<ProjectId>) -> Vec<Assignment>;
        /// The rows of a group, wherever they are.
        fn assignments_of_group(group: GroupId) -> Vec<Assignment>;
        /// One row.
        fn assignment(id: AssignmentId) -> Option<Assignment>;
        /// One project.
        fn project(id: ProjectId) -> Option<Project>;
        /// The projects from the top down to this one; `None` when it does not exist.
        fn project_chain(id: ProjectId) -> Option<Vec<ProjectId>>;
        /// The project with this short name.
        fn project_by_short_name(name: &ShortName) -> Option<Project>;
        /// The levels a role has set for the fields of the catalogue: in a project (`Some`) or its
        /// own (`None`).
        fn field_levels(role: RoleId, project: Option<ProjectId>) -> Vec<(Field, FieldLevel)>;
        /// The levels a role has set for the kinds of people information.
        fn people_levels(role: RoleId, project: Option<ProjectId>) -> Vec<(PeopleKind, PeopleLevel)>;
    }

    writes! {
        /// Takes the lock that makes changes of roles and rows wait for each other, so two
        /// changes that each leave one administrator cannot leave none. A no-op where the
        /// database has one writer anyway.
        fn lock_policy() -> ();
        /// The same as [`PolicyStore::organization_state`], inside the change, so it sees what
        /// the change wrote.
        fn organization_state_in() -> OrganizationState;
        /// The same as [`PolicyStore::policy_inputs`], inside the change: a rule that depends on
        /// what a person may do (no escalation) is judged on what is stored now, after the lock.
        fn policy_inputs_in(person: UserId, place: Place) -> Option<Inputs>;
        /// The same as [`PolicyStore::assignments_on`], inside the change.
        fn assignments_on_in(project: Option<ProjectId>) -> Vec<Assignment>;
        /// The same as [`PolicyStore::assignments_of_group`], inside the change.
        fn assignments_of_group_in(group: GroupId) -> Vec<Assignment>;
        /// The same as [`PolicyStore::role`], inside the change.
        fn role_in(id: RoleId) -> Option<RoleRecord>;
        /// Every project below this one, at any depth, in no particular order.
        fn sub_projects_in(project: ProjectId) -> Vec<ProjectId>;
        /// The same as [`PolicyStore::starting_roles_made`], inside the change, so two servers
        /// that start together do not both make the starting roles.
        fn starting_roles_made_in() -> bool;
        /// Marks the starting roles as made.
        fn mark_starting_roles_made(at: Timestamp) -> ();
        /// Forgets a person of the list of M1 administrators.
        fn delete_legacy_administrator(person: UserId) -> ();
        /// Adds a role with its permissions and its levels.
        fn insert_role(
            role: &Role,
            permissions: &[Permission],
            fields: &[(Field, FieldLevel)],
            people: &[(PeopleKind, PeopleLevel)]
        ) -> ();
        /// Changes the name and the two-step switch of a role.
        fn update_role(id: RoleId, name: &str, two_step: bool) -> bool;
        /// Replaces the permissions of a role.
        fn replace_role_permissions(id: RoleId, permissions: &[Permission]) -> ();
        /// Replaces the levels a role has of its own (not those set for projects).
        fn replace_role_levels(
            id: RoleId,
            fields: &[(Field, FieldLevel)],
            people: &[(PeopleKind, PeopleLevel)]
        ) -> ();
        /// Removes a role, with the rows that give it and the levels it set.
        fn delete_role(id: RoleId) -> bool;
        /// Adds a group.
        fn insert_group(group: &Group) -> ();
        /// Renames a group.
        fn rename_group(id: GroupId, name: &str) -> bool;
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
        /// Sets (`Some`) or takes away (`None`) the level of a field for a role, in a project or
        /// the role's own.
        fn put_field_level(
            role: RoleId,
            project: Option<ProjectId>,
            field: Field,
            level: Option<FieldLevel>
        ) -> ();
        /// The same for a kind of people information.
        fn put_people_level(
            role: RoleId,
            project: Option<ProjectId>,
            kind: PeopleKind,
            level: Option<PeopleLevel>
        ) -> ();
        /// Adds a project.
        fn insert_project(project: &Project) -> ();
    }
}
