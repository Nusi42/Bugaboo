//! The starting roles (SEC-08, ADR-0025 decision 4, M2-Q5): the one definition of the six roles
//! every organization starts with.
//!
//! A system use case makes editable copies from it once, at the first start (never a migration);
//! after that the definition is read only by *Reset to the starting permissions*. The permissions,
//! field levels, *Who sees people* levels and the order are the tables of architecture §7
//! (*Starting roles*) and are written into the page from this code by `cargo xtask gen roles`
//! (DOC-GENERATED fails when the page differs), so there is no second copy to keep in step.

use std::collections::BTreeSet;

use bb_core::access::{FieldLevel, PeopleLevel};
use bb_core::fields::Field;
use bb_core::people_kinds::PeopleKind;
use bb_core::words;

use crate::catalogue::Permission;

words! {
    /// One of the six roles every organization starts with, in the starting order (which is also
    /// the order for role labels). The word is the key that a copy keeps in its `starting` mark,
    /// also when it is renamed.
    pub enum StartingRole {
        /// Everything.
        Administrator => "administrator",
        /// Manages their projects, their sub-projects and who has which role there.
        ProjectLead => "project-lead",
        /// Works on issues.
        TeamMember => "team-member",
        /// Makes issues and changes their own.
        Contributor => "contributor",
        /// Only looks.
        Viewer => "viewer",
        /// Sees only what they reported, and sees people by role only.
        Guest => "guest",
    }
    messages name = "starting-role.";
}

const C: FieldLevel = FieldLevel::CanChange;
const S: FieldLevel = FieldLevel::CanSee;
const H: FieldLevel = FieldLevel::Hidden;

const N: PeopleLevel = PeopleLevel::Names;
const R: PeopleLevel = PeopleLevel::RoleOnly;
const O: PeopleLevel = PeopleLevel::Hidden;

/// The permissions of a Contributor, which every role above it has too (M3-Q6).
const CONTRIBUTOR: [Permission; 5] = [
    Permission::SeeProjects,
    Permission::SeeIssues,
    Permission::MakeIssues,
    Permission::ChangeOwnIssues,
    Permission::Comment,
];

/// What a Team member has beyond a Contributor.
const TEAM_MEMBER: [Permission; 3] = [
    Permission::ChangeIssues,
    Permission::DeleteIssues,
    Permission::MoveIssues,
];

/// What a Project lead has beyond a Team member.
const PROJECT_LEAD: [Permission; 9] = [
    Permission::MakeSubProjects,
    Permission::MoveProjects,
    Permission::ChangeProjectSettings,
    Permission::DecideRoles,
    Permission::DecideWhatRolesSee,
    Permission::CheckAccess,
    Permission::EditAnyway,
    Permission::ManageIssueTypes,
    Permission::ManageLabels,
];

/// A Viewer only looks.
const VIEWER: [Permission; 2] = [Permission::SeeProjects, Permission::SeeIssues];

/// A Guest sees only the issues they reported (no *See issues*), makes issues, changes their own
/// and comments on them.
const GUEST: [Permission; 4] = [
    Permission::SeeProjects,
    Permission::MakeIssues,
    Permission::ChangeOwnIssues,
    Permission::Comment,
];

/// Field levels of a field, in the order of the roles of [`StartingRole::ALL`]. A `match` with no
/// catch-all arm: a field without a row does not compile, so no field falls back to hidden.
const fn field_levels(field: Field) -> [FieldLevel; 6] {
    match field {
        Field::Title | Field::Description | Field::FoundInRelease => [C, C, C, C, S, C],
        Field::Status
        | Field::Priority
        | Field::AssignedTo
        | Field::Labels
        | Field::PlannedForRelease => [C, C, C, S, S, S],
        Field::ReportedBy => [C, C, S, S, S, S],
        Field::DueDate
        | Field::EstimatedTime
        | Field::TimeLeft
        | Field::TimeSpent
        | Field::Activity => [C, C, C, S, S, H],
        Field::Tags => [C, C, C, C, S, S],
        Field::CostCenter => [C, C, S, S, S, H],
    }
}

/// *Who sees people* levels of a kind, in the order of the roles of [`StartingRole::ALL`]. A
/// `match` with no catch-all arm, like [`field_levels`].
const fn people_levels(kind: PeopleKind) -> [PeopleLevel; 6] {
    match kind {
        PeopleKind::AssignedTo
        | PeopleKind::LastChangedBy
        | PeopleKind::Authors
        | PeopleKind::Mentions => [N, N, N, N, N, R],
        PeopleKind::ReportedBy | PeopleKind::Editing | PeopleKind::Pictures => [N, N, N, N, N, O],
        PeopleKind::Followers | PeopleKind::ProjectMembers => [N, N, N, R, R, O],
        PeopleKind::TimeEntries => [N, N, N, O, O, O],
    }
}

/// The level a new own field starts with for a role (architecture section 7, *The field
/// catalogue*): *Can see* for every role that sees projects, *Can change* for the roles that also
/// hold `changes_with` (*Change issues* for a field of issues). Whoever makes the field can change
/// it at once. The same rule gives a custom role its level when the field is made, and a starting
/// role its level again when it is reset.
#[must_use]
pub fn starting_own_field_level(
    permissions: &BTreeSet<Permission>,
    changes_with: Permission,
) -> FieldLevel {
    if !permissions.contains(&Permission::SeeProjects) {
        FieldLevel::Hidden
    } else if permissions.contains(&changes_with) {
        FieldLevel::CanChange
    } else {
        FieldLevel::CanSee
    }
}

impl StartingRole {
    /// The place of the role in the starting order, counting from 0.
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// The permissions of the role.
    #[must_use]
    pub fn permissions(self) -> Vec<Permission> {
        match self {
            Self::Administrator => Permission::ALL.to_vec(),
            Self::ProjectLead => [&CONTRIBUTOR[..], &TEAM_MEMBER, &PROJECT_LEAD].concat(),
            Self::TeamMember => [&CONTRIBUTOR[..], &TEAM_MEMBER].concat(),
            Self::Contributor => CONTRIBUTOR.to_vec(),
            Self::Viewer => VIEWER.to_vec(),
            Self::Guest => GUEST.to_vec(),
        }
    }

    /// The level of a field for the role.
    #[must_use]
    pub fn field_level(self, field: Field) -> FieldLevel {
        field_levels(field)[self.index()]
    }

    /// How the role sees people of a kind.
    #[must_use]
    pub fn people_level(self, kind: PeopleKind) -> PeopleLevel {
        people_levels(kind)[self.index()]
    }

    /// Whether everybody with the role needs two-step sign-in at the start: off for all.
    #[must_use]
    pub const fn two_step(self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn the_roles_come_in_the_starting_order_with_their_own_keys_and_names() {
        let mut names = HashSet::new();
        for (at, role) in StartingRole::ALL.iter().copied().enumerate() {
            assert_eq!(role.index(), at);
            assert_eq!(StartingRole::from_code(role.code()), Some(role));
            assert_eq!(
                role.name().as_str(),
                format!("starting-role.{}", role.code())
            );
            assert!(names.insert(role.name()));
            assert!(
                !role.two_step(),
                "two-step sign-in is off for every starting role"
            );
        }
        assert_eq!(StartingRole::from_code("owner"), None);
    }

    #[test]
    fn cf_06_a_new_own_field_starts_can_see_for_all_and_can_change_from_team_member_up() {
        let level = |role: StartingRole| {
            starting_own_field_level(
                &role.permissions().into_iter().collect(),
                Permission::ChangeIssues,
            )
        };
        for role in [
            StartingRole::Administrator,
            StartingRole::ProjectLead,
            StartingRole::TeamMember,
        ] {
            assert_eq!(level(role), FieldLevel::CanChange, "{role:?}");
        }
        for role in [
            StartingRole::Contributor,
            StartingRole::Viewer,
            StartingRole::Guest,
        ] {
            assert_eq!(level(role), FieldLevel::CanSee, "{role:?}");
        }
        assert_eq!(
            starting_own_field_level(&BTreeSet::new(), Permission::ChangeIssues),
            FieldLevel::Hidden
        );
    }

    #[test]
    fn a_contributor_sees_the_cost_center_at_the_start_so_the_exit_demo_can_hide_it() {
        assert_eq!(
            StartingRole::Contributor.field_level(Field::CostCenter),
            FieldLevel::CanSee
        );
    }
}
