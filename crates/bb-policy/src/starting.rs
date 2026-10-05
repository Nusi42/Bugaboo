//! The starting roles (SEC-08, ADR-0025 decision 4, M2-Q5): the one definition of the six roles
//! every organization starts with.
//!
//! A system use case makes editable copies from it once, at the first start (never a migration);
//! after that the definition is read only by *Reset to the starting permissions*. The permissions,
//! field levels, *Who sees people* levels and the order are the tables of architecture §7
//! (*Starting roles*); the tests below repeat those tables, so a change of one without the other
//! fails.

use bb_core::access::{FieldLevel, PeopleLevel};
use bb_core::fields::Field;
use bb_core::message::{self, MessageKey};
use bb_core::people_kinds::PeopleKind;

use crate::catalogue::Permission;

/// One of the six roles every organization starts with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StartingRole {
    /// Everything.
    Administrator,
    /// Manages their projects, their sub-projects and who has which role there.
    ProjectLead,
    /// Works on issues.
    TeamMember,
    /// Makes issues and changes their own.
    Contributor,
    /// Only looks.
    Viewer,
    /// Sees only what they reported, and sees people by role only.
    Guest,
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
    /// The six roles, in the starting order (which is also the order for role labels).
    pub const ALL: [Self; 6] = [
        Self::Administrator,
        Self::ProjectLead,
        Self::TeamMember,
        Self::Contributor,
        Self::Viewer,
        Self::Guest,
    ];

    /// The place of the role in the starting order, counting from 0.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Administrator => 0,
            Self::ProjectLead => 1,
            Self::TeamMember => 2,
            Self::Contributor => 3,
            Self::Viewer => 4,
            Self::Guest => 5,
        }
    }

    /// The key that a copy keeps in its `starting` mark, also when it is renamed.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Administrator => "administrator",
            Self::ProjectLead => "project-lead",
            Self::TeamMember => "team-member",
            Self::Contributor => "contributor",
            Self::Viewer => "viewer",
            Self::Guest => "guest",
        }
    }

    /// The starting role with this key.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|role| role.key() == key)
    }

    /// The name of the role when it is made (`starting-role.<key>`).
    #[must_use]
    pub const fn name(self) -> MessageKey {
        match self {
            Self::Administrator => message::STARTING_ROLE_ADMINISTRATOR,
            Self::ProjectLead => message::STARTING_ROLE_PROJECT_LEAD,
            Self::TeamMember => message::STARTING_ROLE_TEAM_MEMBER,
            Self::Contributor => message::STARTING_ROLE_CONTRIBUTOR,
            Self::Viewer => message::STARTING_ROLE_VIEWER,
            Self::Guest => message::STARTING_ROLE_GUEST,
        }
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

    /// The rows of the table "Starting roles" in architecture §7, one letter per role:
    /// a tick is a permission the role has.
    const PERMISSION_TABLE: [(Permission, &str); 14] = [
        (Permission::SeeProjects, "YYYYYY"),
        (Permission::MakeProjects, "Y-----"),
        (Permission::MakeSubProjects, "YY----"),
        (Permission::MoveProjects, "YY----"),
        (Permission::ChangeProjectSettings, "YY----"),
        (Permission::DecideRoles, "YY----"),
        (Permission::DecideWhatRolesSee, "YY----"),
        (Permission::CheckAccess, "YY----"),
        (Permission::ManagePeople, "Y-----"),
        (Permission::ManageInvitations, "Y-----"),
        (Permission::ManageGroups, "Y-----"),
        (Permission::ManageRoles, "Y-----"),
        (Permission::ChangeOrganizationSettings, "Y-----"),
        (Permission::SeeStatus, "Y-----"),
    ];

    fn level(letter: char) -> FieldLevel {
        match letter {
            'C' => FieldLevel::CanChange,
            'S' => FieldLevel::CanSee,
            _ => FieldLevel::Hidden,
        }
    }

    fn people(letter: char) -> PeopleLevel {
        match letter {
            'N' => PeopleLevel::Names,
            'R' => PeopleLevel::RoleOnly,
            _ => PeopleLevel::Hidden,
        }
    }

    #[test]
    fn m2_q5_the_permissions_are_the_table_of_architecture_7() {
        let listed: HashSet<Permission> = PERMISSION_TABLE.iter().map(|(p, _)| *p).collect();
        assert_eq!(
            listed.len(),
            Permission::ALL.len(),
            "every permission has a row"
        );
        for (permission, ticks) in PERMISSION_TABLE {
            for (role, tick) in StartingRole::ALL.into_iter().zip(ticks.chars()) {
                assert_eq!(
                    role.permissions().contains(&permission),
                    tick == 'Y',
                    "{} for {}",
                    permission,
                    role.key()
                );
            }
        }
    }

    #[test]
    fn m2_q5_the_field_levels_are_the_table_of_architecture_7() {
        let table = [
            ("title", "CCCCSC"),
            ("description", "CCCCSC"),
            ("status", "CCCSSS"),
            ("priority", "CCCSSS"),
            ("assigned-to", "CCCSSS"),
            ("reported-by", "CCSSSS"),
            ("due-date", "CCCSSH"),
            ("labels", "CCCSSS"),
            ("tags", "CCCCSS"),
            ("planned-for-release", "CCCSSS"),
            ("found-in-release", "CCCCSC"),
            ("estimated-time", "CCCSSH"),
            ("time-left", "CCCSSH"),
            ("time-spent", "CCCSSH"),
            ("activity", "CCCSSH"),
            ("cost-center", "CCSSSH"),
        ];
        assert_eq!(table.len(), Field::ALL.len());
        for (key, letters) in table {
            let field = Field::from_code(key).unwrap();
            for (role, letter) in StartingRole::ALL.into_iter().zip(letters.chars()) {
                assert_eq!(
                    role.field_level(field),
                    level(letter),
                    "{key} for {}",
                    role.key()
                );
            }
        }
    }

    #[test]
    fn m2_q5_the_people_levels_are_the_table_of_architecture_7() {
        let table = [
            ("assigned-to", "NNNNNR"),
            ("reported-by", "NNNNNH"),
            ("last-changed-by", "NNNNNR"),
            ("authors", "NNNNNR"),
            ("editing", "NNNNNH"),
            ("followers", "NNNRRH"),
            ("time-entries", "NNNHHH"),
            ("mentions", "NNNNNR"),
            ("pictures", "NNNNNH"),
            ("project-members", "NNNRRH"),
        ];
        assert_eq!(table.len(), PeopleKind::ALL.len());
        for (key, letters) in table {
            let kind = PeopleKind::from_code(key).unwrap();
            for (role, letter) in StartingRole::ALL.into_iter().zip(letters.chars()) {
                assert_eq!(
                    role.people_level(kind),
                    people(letter),
                    "{key} for {}",
                    role.key()
                );
            }
        }
    }

    #[test]
    fn the_roles_come_in_the_starting_order_with_their_own_keys_and_names() {
        let keys: Vec<&str> = StartingRole::ALL.iter().map(|role| role.key()).collect();
        assert_eq!(
            keys,
            [
                "administrator",
                "project-lead",
                "team-member",
                "contributor",
                "viewer",
                "guest"
            ]
        );
        let mut names = HashSet::new();
        for (at, role) in StartingRole::ALL.into_iter().enumerate() {
            assert_eq!(role.index(), at);
            assert_eq!(StartingRole::from_key(role.key()), Some(role));
            assert_eq!(
                role.name().as_str(),
                format!("starting-role.{}", role.key())
            );
            assert!(names.insert(role.name()));
            assert!(
                !role.two_step(),
                "two-step sign-in is off for every starting role"
            );
        }
        assert_eq!(StartingRole::from_key("owner"), None);
    }

    #[test]
    fn a_contributor_sees_the_cost_center_at_the_start_so_the_exit_demo_can_hide_it() {
        assert_eq!(
            StartingRole::Contributor.field_level(Field::CostCenter),
            FieldLevel::CanSee
        );
    }
}
