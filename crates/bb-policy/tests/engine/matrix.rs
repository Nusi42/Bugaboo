//! The rows of the permission matrix that are about the engine itself (SEC-MATRIX, WP-10): every
//! built-in field for every starting role and every level, shown or hidden in the answer through
//! `Authorized<T>` (SEC-06), and every kind of people information for every starting role and level,
//! shown through `PersonRef` (PRIV-12). The routes for every actor are the other half of the
//! matrix, in `bb-server` (`tests/matrix`).
//!
//! The tables below are written by hand from architecture §7 (*Starting field levels*, *Starting
//! Who sees people*): the starting roles of the code are compared with them first, then the engine
//! is asked.

use bb_core::access::{FieldLevel, PeopleLevel};
use bb_core::fields::Field;
use bb_core::id::UserId;
use bb_core::people_kinds::PeopleKind;
use bb_policy::{Masked, PeopleContext, Permission, PersonRef, StartingRole, Target, resolve};
use serde::{Serialize, Serializer};
use serde_json::{Map, Value};

use crate::common::World;

const C: FieldLevel = FieldLevel::CanChange;
const S: FieldLevel = FieldLevel::CanSee;
const H: FieldLevel = FieldLevel::Hidden;

/// The starting field levels: Administrator, Project lead, Team member, Contributor, Viewer, Guest.
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

const N: PeopleLevel = PeopleLevel::Names;
const R: PeopleLevel = PeopleLevel::RoleOnly;
const O: PeopleLevel = PeopleLevel::Hidden;

/// The starting *Who sees people*: Administrator, Project lead, Team member, Contributor, Viewer,
/// Guest.
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

/// An answer with a member for every field of the catalogue, named by the key of the field, as a
/// view of an issue will have them.
struct Answer(Map<String, Value>);

impl Answer {
    fn whole() -> Self {
        Self(
            Field::ALL
                .iter()
                .map(|field| (field.code().to_owned(), Value::from(1)))
                .collect(),
        )
    }
}

impl Serialize for Answer {
    fn serialize<T: Serializer>(&self, serializer: T) -> Result<T::Ok, T::Error> {
        self.0.serialize(serializer)
    }
}

impl Masked for Answer {
    const FIELDS: &'static [(&'static str, Field)] = &[
        ("title", Field::Title),
        ("description", Field::Description),
        ("status", Field::Status),
        ("priority", Field::Priority),
        ("assigned-to", Field::AssignedTo),
        ("reported-by", Field::ReportedBy),
        ("due-date", Field::DueDate),
        ("labels", Field::Labels),
        ("tags", Field::Tags),
        ("planned-for-release", Field::PlannedForRelease),
        ("found-in-release", Field::FoundInRelease),
        ("estimated-time", Field::EstimatedTime),
        ("time-left", Field::TimeLeft),
        ("time-spent", Field::TimeSpent),
        ("activity", Field::Activity),
        ("cost-center", Field::CostCenter),
    ];
}

#[test]
fn the_tables_of_the_matrix_are_the_starting_roles_of_the_code() {
    assert_eq!(FIELD_LEVELS.len(), Field::ALL.len());
    assert_eq!(PEOPLE_LEVELS.len(), PeopleKind::ALL.len());
    assert_eq!(Answer::FIELDS.len(), Field::ALL.len());
    for (field, levels) in FIELD_LEVELS {
        for (role, level) in StartingRole::ALL.iter().zip(levels) {
            assert_eq!(
                role.field_level(field),
                level,
                "{field:?} for {}",
                role.code()
            );
        }
    }
    for (kind, levels) in PEOPLE_LEVELS {
        for (role, level) in StartingRole::ALL.iter().zip(levels) {
            assert_eq!(
                role.people_level(kind),
                level,
                "{kind:?} for {}",
                role.code()
            );
        }
    }
    for (name, field) in Answer::FIELDS {
        assert_eq!(*name, field.code());
    }
}

/// What the answer holds for a mask: the names of the members that are in it.
fn members(answer: &bb_policy::Authorized<Answer>) -> Vec<String> {
    let Ok(Value::Object(shown)) = answer.masked_value() else {
        panic!("an answer is an object");
    };
    shown.keys().cloned().collect()
}

#[test]
fn sec_06_every_field_level_of_every_role_shows_or_hides_the_field_in_answers() {
    let mut rows = 0;
    for (index, role_key) in StartingRole::ALL.iter().enumerate() {
        // The levels the role has of its own, for the whole organization.
        let mut world = World::new();
        let role = world.role(role_key.code(), &[Permission::SeeProjects]);
        world.give_everywhere(role);
        for (field, levels) in FIELD_LEVELS {
            world.field(role, None, field, levels[index]);
        }
        let own = world.at(world.web);
        let answer = own.authorize(Answer::whole());
        let shown = members(&answer);
        for (field, levels) in FIELD_LEVELS {
            assert_eq!(
                own.field_level(field),
                levels[index],
                "{field:?} for {role_key:?}"
            );
            assert_eq!(
                shown.iter().any(|name| name == field.code()),
                levels[index] >= FieldLevel::CanSee,
                "{field:?} for {role_key:?}: in the answer only when it can be seen"
            );
            assert_eq!(
                answer.hidden_properties().contains(&field.code()),
                levels[index] == FieldLevel::Hidden,
                "{field:?} for {role_key:?}: asking for it by `fields` fails only when hidden"
            );
            rows += 1;
        }
        // A project sets any level for the role and its sub-projects inherit it: every field at
        // every level, whatever the role had of its own.
        for level in FieldLevel::ALL.iter().copied() {
            let mut world = World::new();
            let role = world.role(role_key.code(), &[Permission::SeeProjects]);
            world.give_everywhere(role);
            for (field, levels) in FIELD_LEVELS {
                world.field(role, None, field, levels[index]);
                world.field(role, Some(world.acme), field, level);
            }
            for project in [world.acme, world.web, world.api] {
                let held = world.at(project);
                let answer = held.authorize(Answer::whole());
                let shown = members(&answer);
                for (field, _) in FIELD_LEVELS {
                    assert_eq!(held.field_level(field), level, "{field:?} for {role_key:?}");
                    assert_eq!(
                        shown.iter().any(|name| name == field.code()),
                        level >= FieldLevel::CanSee,
                        "{field:?} as {level:?} for {role_key:?}"
                    );
                    assert_eq!(
                        held.field_mask().can_change(field),
                        level == FieldLevel::CanChange
                    );
                    rows += 1;
                }
            }
            // A project beside ACME is not under what ACME set: the role's own level counts there.
            let beside = world.at(world.other);
            for (field, levels) in FIELD_LEVELS {
                assert_eq!(
                    beside.field_level(field),
                    levels[index],
                    "{field:?} for {role_key:?}"
                );
            }
        }
    }
    // 16 fields for 6 roles at their own level, and for 6 roles 3 levels in 3 projects.
    assert_eq!(rows, 16 * 6 + 16 * 6 * 3 * 3);
}

/// How the viewer sees a person of the kind, as the doc says.
fn expected(level: PeopleLevel, id: UserId, role: Option<&str>) -> PersonRef {
    match level {
        PeopleLevel::Names => PersonRef::Name {
            id,
            name: "Bea Builder".to_owned(),
        },
        PeopleLevel::RoleOnly => role.map_or(PersonRef::Someone, |role| PersonRef::Role {
            role: role.to_owned(),
        }),
        PeopleLevel::Hidden => PersonRef::Someone,
    }
}

#[test]
fn priv_12_every_kind_and_level_shows_the_person_as_the_rule_says() {
    let bea = UserId::from_random([2; 16]);
    let target = |role: Option<&'static str>| Target {
        id: bea,
        name: "Bea Builder",
        role,
        removed: None,
    };
    let mut rows = 0;
    for (index, role_key) in StartingRole::ALL.iter().enumerate() {
        for (kind, levels) in PEOPLE_LEVELS {
            // The starting level of the role for the kind, then every level set on a project.
            let mut starting = World::new();
            let role = starting.role(role_key.code(), &[Permission::SeeProjects]);
            starting.give_everywhere(role);
            starting.people(role, None, kind, levels[index]);
            let held = starting.at(starting.web);
            assert_eq!(
                held.kind_level(kind),
                levels[index],
                "{kind:?} for {role_key:?}"
            );
            assert_eq!(
                resolve(
                    &held,
                    kind,
                    PeopleContext::Ordinary,
                    target(Some("a Team member"))
                ),
                expected(levels[index], bea, Some("a Team member"))
            );
            rows += 1;
            for level in PeopleLevel::ALL.iter().copied() {
                let mut world = World::new();
                let role = world.role(role_key.code(), &[Permission::SeeProjects]);
                world.give_everywhere(role);
                world.people(role, None, kind, levels[index]);
                world.people(role, Some(world.acme), kind, level);
                for project in [world.acme, world.web, world.api] {
                    let viewer = world.at(project);
                    // A person with a role, one without, a removed person, and the viewer themselves.
                    for with_role in [Some("a Team member"), None] {
                        assert_eq!(
                            resolve(&viewer, kind, PeopleContext::Ordinary, target(with_role)),
                            expected(level, bea, with_role),
                            "{kind:?} as {level:?} for {role_key:?}"
                        );
                    }
                    let removed = bb_policy::FormerUser {
                        label: "K7Q2".to_owned(),
                    };
                    let gone = resolve(
                        &viewer,
                        kind,
                        PeopleContext::Ordinary,
                        Target {
                            removed: Some(&removed),
                            ..target(Some("a Team member"))
                        },
                    );
                    assert_eq!(
                        gone,
                        if level == PeopleLevel::Names {
                            PersonRef::Former {
                                label: "K7Q2".to_owned(),
                            }
                        } else {
                            PersonRef::Someone
                        },
                        "a removed person as {level:?}"
                    );
                    let own = resolve(
                        &viewer,
                        kind,
                        PeopleContext::Ordinary,
                        Target {
                            id: world.person,
                            ..target(None)
                        },
                    );
                    assert_eq!(
                        own,
                        PersonRef::Name {
                            id: world.person,
                            name: "Bea Builder".to_owned()
                        },
                        "everybody sees themselves by name"
                    );
                    rows += 1;
                }
            }
        }
    }
    // 10 kinds for 6 roles at their starting level, and for 3 levels in 3 projects.
    assert_eq!(rows, 10 * 6 + 10 * 6 * 3 * 3);
}

#[test]
fn priv_12_in_a_management_screen_those_who_manage_see_names_and_the_rest_see_project_members() {
    let bea = UserId::from_random([2; 16]);
    let target = Target {
        id: bea,
        name: "Bea Builder",
        role: Some("a Team member"),
        removed: None,
    };
    // Who manages people or roles (M2-Q3): each of the four permissions makes a viewer see names,
    // whatever the kind says; nobody else is shown by the kind *Project members*.
    let managers = [
        Permission::ManagePeople,
        Permission::ManageGroups,
        Permission::ManageRoles,
        Permission::DecideRoles,
    ];
    for kind in PeopleKind::ALL.iter().copied() {
        for level in PeopleLevel::ALL.iter().copied() {
            let mut plain = World::new();
            let role = plain.role("Viewer", &[Permission::SeeProjects]);
            plain.give_everywhere(role);
            if kind != PeopleKind::ProjectMembers {
                // What the kind says for the viewer does not count in a management screen.
                plain.people(role, None, kind, PeopleLevel::Names);
            }
            plain.people(role, None, PeopleKind::ProjectMembers, level);
            let shown = resolve(
                &plain.at_organization(),
                kind,
                PeopleContext::Management,
                target,
            );
            assert_eq!(
                shown,
                expected(level, bea, Some("a Team member")),
                "{kind:?} as {level:?}"
            );
            for permission in managers {
                let mut world = World::new();
                let role = world.role("Manager", &[Permission::SeeProjects, permission]);
                world.give_everywhere(role);
                world.people(role, None, PeopleKind::ProjectMembers, PeopleLevel::Hidden);
                if kind != PeopleKind::ProjectMembers {
                    world.people(role, None, kind, PeopleLevel::Hidden);
                }
                let shown = resolve(
                    &world.at_organization(),
                    kind,
                    PeopleContext::Management,
                    target,
                );
                assert_eq!(
                    shown,
                    expected(PeopleLevel::Names, bea, None),
                    "{permission:?}"
                );
            }
        }
    }
}
