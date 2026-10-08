//! The starting roles (SEC-08, ADR-0025 decision 4, M2-Q5): the one definition of the six roles
//! every organization starts with.
//!
//! A system use case makes editable copies from it once, at the first start (never a migration);
//! after that the definition is read only by *Reset to the starting permissions*. The permissions,
//! field levels, *Who sees people* levels and the order are the tables of architecture §7
//! (*Starting roles*) and are written into the page from this code by `cargo xtask gen roles`
//! (DOC-GENERATED fails when the page differs), so there is no second copy to keep in step.

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

/// The permissions of a Project lead.
const PROJECT_LEAD: [Permission; 7] = [
    Permission::SeeProjects,
    Permission::MakeSubProjects,
    Permission::MoveProjects,
    Permission::ChangeProjectSettings,
    Permission::DecideRoles,
    Permission::DecideWhatRolesSee,
    Permission::CheckAccess,
];

/// Field levels per field, in the order of the roles of [`StartingRole::ALL`].
const FIELD_LEVELS: [(Field, [FieldLevel; 6]); 16] = [
    (Field::Title, [C, C, C, C, S, C]),
    (Field::Description, [C, C, C, C, S, C]),
    (Field::Status, [C, C, C, S, S, S]),
    (Field::Priority, [C, C, C, S, S, S]),
    (Field::AssignedTo, [C, C, C, S, S, S]),
    (Field::ReportedBy, [C, C, S, S, S, S]),
    (Field::DueDate, [C, C, C, S, S, H]),
    (Field::Labels, [C, C, C, S, S, S]),
    (Field::Tags, [C, C, C, C, S, S]),
    (Field::PlannedForRelease, [C, C, C, S, S, S]),
    (Field::FoundInRelease, [C, C, C, C, S, C]),
    (Field::EstimatedTime, [C, C, C, S, S, H]),
    (Field::TimeLeft, [C, C, C, S, S, H]),
    (Field::TimeSpent, [C, C, C, S, S, H]),
    (Field::Activity, [C, C, C, S, S, H]),
    (Field::CostCenter, [C, C, S, S, S, H]),
];

/// *Who sees people* levels per kind, in the order of the roles of [`StartingRole::ALL`].
const PEOPLE_LEVELS: [(PeopleKind, [PeopleLevel; 6]); 10] = [
    (PeopleKind::AssignedTo, [N, N, N, N, N, R]),
    (PeopleKind::ReportedBy, [N, N, N, N, N, O]),
    (PeopleKind::LastChangedBy, [N, N, N, N, N, R]),
    (PeopleKind::Authors, [N, N, N, N, N, R]),
    (PeopleKind::Editing, [N, N, N, N, N, O]),
    (PeopleKind::Followers, [N, N, N, R, R, O]),
    (PeopleKind::TimeEntries, [N, N, N, O, O, O]),
    (PeopleKind::Mentions, [N, N, N, N, N, R]),
    (PeopleKind::Pictures, [N, N, N, N, N, O]),
    (PeopleKind::ProjectMembers, [N, N, N, R, R, O]),
];

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
            Self::ProjectLead => PROJECT_LEAD.to_vec(),
            Self::TeamMember | Self::Contributor | Self::Viewer | Self::Guest => {
                vec![Permission::SeeProjects]
            }
        }
    }

    /// The level of a field for the role.
    #[must_use]
    pub fn field_level(self, field: Field) -> FieldLevel {
        FIELD_LEVELS
            .iter()
            .find(|(candidate, _)| *candidate == field)
            .map_or(FieldLevel::Hidden, |(_, levels)| levels[self.index()])
    }

    /// How the role sees people of a kind.
    #[must_use]
    pub fn people_level(self, kind: PeopleKind) -> PeopleLevel {
        PEOPLE_LEVELS
            .iter()
            .find(|(candidate, _)| *candidate == kind)
            .map_or(PeopleLevel::Hidden, |(_, levels)| levels[self.index()])
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
    fn a_contributor_sees_the_cost_center_at_the_start_so_the_exit_demo_can_hide_it() {
        assert_eq!(
            StartingRole::Contributor.field_level(Field::CostCenter),
            FieldLevel::CanSee
        );
    }
}
