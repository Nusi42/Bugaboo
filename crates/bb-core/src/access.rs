//! What roles, groups and assignments store (SEC-01, SEC-02, SEC-08, ADR-0025), and the levels
//! of field permissions and *Who sees people*.
//!
//! The permissions themselves are the enum `Permission` in `bb-policy` (the one catalogue); a
//! stored role names them by their keys. Every field carries its personal-data mark (ADR-0015):
//! who belongs to which group and who has which role where is personal data of the category
//! `organization`.

use jiff::Timestamp;

use crate::fields::Field;
use crate::id::{AssignmentId, GroupId, ProjectId, RoleId, UserId};
use crate::people_kinds::PeopleKind;
use crate::personal::Entity;
use crate::technical::TechnicalName;

words! {
    /// What a role allows to do with a field: *Hidden*, *Can see*, *Can change*. The order is
    /// the order of the words: the level of several roles is the highest one.
    pub enum FieldLevel {
        /// The field is not in any answer, and asking for it fails like asking for a field that
        /// does not exist.
        Hidden => "hidden",
        /// The field is shown.
        CanSee => "can-see",
        /// The field is shown and can be changed, where the person can change the item at all.
        CanChange => "can-change",
    }
}

words! {
    /// How a viewer sees people of one kind: *Names*, *Role only* (*"a Team member"*) or *Hidden*
    /// (*"Someone"*). The order is the order of the words: the level of several roles is the
    /// most open one.
    pub enum PeopleLevel {
        /// Shown as *Someone*.
        Hidden => "hidden",
        /// Shown by the label of their role.
        RoleOnly => "role-only",
        /// Shown by name.
        Names => "names",
    }
}

words! {
    /// Where a role given to someone counts (*Where it counts*).
    pub enum AppliesTo {
        /// In the whole organization. Rows like this exist only in *Organization settings*.
        Organization => "organization",
        /// Only in the project the row is on.
        Project => "project",
        /// In the project the row is on and its sub-projects.
        ProjectAndSubProjects => "project-and-sub-projects",
    }
}

/// A group of people (SEC-02). Groups are flat in M2: a group holds people, never groups.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Group {
    /// The ID of the group.
    #[not_personal]
    pub id: GroupId,
    /// What the group is called. Free text that could name a person.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub name: String,
}

/// A person in a group.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct GroupMember {
    /// The group.
    #[not_personal]
    pub group: GroupId,
    /// Who is in it.
    #[personal(category = "organization", basis = "contract")]
    pub person: UserId,
    /// When the person was put into the group.
    #[personal(category = "activity", basis = "contract")]
    pub added_at: Timestamp,
}

/// A role: a named set of permissions, with a switch for two-step sign-in (SEC-08, ADR-0025).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Role {
    /// The ID of the role.
    #[not_personal]
    pub id: RoleId,
    /// What the role is called. Free text that could name a person.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub name: String,
    /// The starting role this one is a copy of (`administrator`, …), kept when the role is
    /// renamed so that *Reset to the starting permissions* still finds its definition.
    #[not_personal]
    pub starting: Option<TechnicalName>,
    /// The place in the order of roles. The first role of a person in the order is their label
    /// when they are shown as *Role only*.
    #[not_personal]
    pub position: i64,
    /// Whether everybody with this role needs two-step sign-in (AUTH-03, M1-F45).
    #[not_personal]
    pub two_step: bool,
}

/// One permission of a role, by the key of the permission catalogue.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct RolePermission {
    /// The role.
    #[not_personal]
    pub role: RoleId,
    /// The key of the permission (`see-projects`).
    #[not_personal]
    pub permission: TechnicalName,
}

/// One row of *Who has which role*: a person or a group, a role or *No access*, and where it
/// counts (ADR-0025, decision 2).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Assignment {
    /// The ID of the row.
    #[not_personal]
    pub id: AssignmentId,
    /// The person, when the row is for a person.
    #[personal(category = "organization", basis = "contract")]
    pub person: Option<UserId>,
    /// The group, when the row is for a group.
    #[not_personal]
    pub group: Option<GroupId>,
    /// The role, or `None` for *No access*.
    #[not_personal]
    pub role: Option<RoleId>,
    /// The project the row is on, or `None` for the whole organization.
    #[not_personal]
    pub project: Option<ProjectId>,
    /// Where it counts.
    #[not_personal]
    pub applies_to: AppliesTo,
    /// When the row was made.
    #[personal(category = "activity", basis = "contract")]
    pub given_at: Timestamp,
}

/// The level of a field for a role in the whole organization (the role's own level).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct RoleFieldLevel {
    /// The role.
    #[not_personal]
    pub role: RoleId,
    /// The field.
    #[not_personal]
    pub field: Field,
    /// Its level.
    #[not_personal]
    pub level: FieldLevel,
}

/// The level of a field for a role in one project and its sub-projects, until one of them sets
/// its own.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct ProjectFieldLevel {
    /// The project.
    #[not_personal]
    pub project: ProjectId,
    /// The role.
    #[not_personal]
    pub role: RoleId,
    /// The field.
    #[not_personal]
    pub field: Field,
    /// Its level.
    #[not_personal]
    pub level: FieldLevel,
}

/// How a role sees people of one kind in the whole organization (the role's own level).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct RolePeopleLevel {
    /// The role.
    #[not_personal]
    pub role: RoleId,
    /// The kind of people information.
    #[not_personal]
    pub kind: PeopleKind,
    /// Its level.
    #[not_personal]
    pub level: PeopleLevel,
}

/// How a role sees people of one kind in one project and its sub-projects, until one of them
/// sets its own.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct ProjectPeopleLevel {
    /// The project.
    #[not_personal]
    pub project: ProjectId,
    /// The role.
    #[not_personal]
    pub role: RoleId,
    /// The kind of people information.
    #[not_personal]
    pub kind: PeopleKind,
    /// Its level.
    #[not_personal]
    pub level: PeopleLevel,
}

/// A person who was an administrator in M1 (the flag is gone). The migration to M2 keeps the
/// list until the first start gives these people the Administrator role; then the rows go.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct LegacyAdministrator {
    /// The person.
    #[personal(category = "organization", basis = "contract")]
    pub person: UserId,
}

/// The one row that says the starting roles were made, so they are made once and never again
/// (ADR-0025, decision 4).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct StartingRolesMade {
    /// When they were made.
    #[not_personal]
    pub made_at: Timestamp,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_levels_are_ordered_from_closed_to_open() {
        assert!(FieldLevel::Hidden < FieldLevel::CanSee);
        assert!(FieldLevel::CanSee < FieldLevel::CanChange);
        assert!(PeopleLevel::Hidden < PeopleLevel::RoleOnly);
        assert!(PeopleLevel::RoleOnly < PeopleLevel::Names);
        assert_eq!(FieldLevel::ALL.len(), 3);
        assert_eq!(PeopleLevel::ALL.len(), 3);
    }

    #[test]
    fn where_it_counts_has_the_three_words_of_the_api() {
        let words: Vec<&str> = AppliesTo::ALL.iter().map(|place| place.code()).collect();
        assert_eq!(
            words,
            ["organization", "project", "project-and-sub-projects"]
        );
    }
}
