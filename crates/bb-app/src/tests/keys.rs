//! What an access key narrows its owner to (AUTH-07, ADR-0025 decision 8), from the key as it is
//! stored: a property test over random roles, rows, choices, permissions and projects.
//!
//! The engine has its own property test of narrowing on plain data (`bb-policy`); this one starts
//! where the use cases do, with an `AccessKeyRecord`, and checks the whole mapping from what is
//! stored to what the key may do.

use std::collections::BTreeSet;

use bb_core::access::{AppliesTo, FieldLevel, PeopleLevel};
use bb_core::access_key::{AccessKey, KeyChoice};
use bb_core::fields::Field;
use bb_core::id::{AccessKeyId, AssignmentId, ProjectId, RoleId, SequentialIds, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_core::security_log::Digest;
use bb_policy::{
    FieldRule, Inputs, Kind, LevelRules, NOT_FOR_KEYS, PeopleRule, Permission, Place, RoleData,
    Row, Who, evaluate,
};
use jiff::Timestamp;
use proptest::prelude::*;

use crate::auth::{AccessKeyRecord, narrowing_of};

/// The roles, the projects (ACME, WEB inside it, API inside that) and the person of the property.
struct World {
    person: UserId,
    projects: [ProjectId; 3],
    roles: [RoleId; 2],
}

impl World {
    fn new() -> Self {
        let mut ids = SequentialIds::default();
        Self {
            person: UserId::generate(&mut ids),
            projects: [
                ProjectId::generate(&mut ids).unwrap(),
                ProjectId::generate(&mut ids).unwrap(),
                ProjectId::generate(&mut ids).unwrap(),
            ],
            roles: [
                RoleId::generate(&mut ids).unwrap(),
                RoleId::generate(&mut ids).unwrap(),
            ],
        }
    }
}

/// A row of the owner: a role (or *No access*) on a project (or for the whole organization).
#[derive(Clone, Debug)]
struct RowSpec {
    role: Option<usize>,
    project: Option<usize>,
    here_only: bool,
}

fn row_strategy() -> impl Strategy<Value = RowSpec> {
    (
        prop::option::of(0..2_usize),
        prop::option::of(0..3_usize),
        any::<bool>(),
    )
        .prop_map(|(role, project, here_only)| RowSpec {
            role,
            // *No access* is only ever set on a project.
            project: if role.is_none() {
                project.or(Some(0))
            } else {
                project
            },
            here_only,
        })
}

fn permissions_of(mask: u16) -> BTreeSet<Permission> {
    Permission::ALL
        .iter()
        .enumerate()
        .filter(|(at, _)| mask & (1 << at) != 0)
        .map(|(_, permission)| *permission)
        .collect()
}

/// What the owner holds at the place: `place` 0 is the whole organization, 1 to 3 the projects.
fn owner_inputs(world: &World, masks: [u16; 2], rows: &[RowSpec], place: usize) -> Inputs {
    let rows = rows
        .iter()
        .zip(1_u64..)
        .map(|(spec, number)| {
            let mut ids = SequentialIds::default();
            for _ in 0..number {
                let _ = AssignmentId::generate(&mut ids);
            }
            Row {
                id: AssignmentId::generate(&mut ids).unwrap(),
                who: Who::Person,
                role: spec.role.map(|at| world.roles[at]),
                project: spec.project.map(|at| world.projects[at]),
                applies_to: match (spec.project, spec.here_only) {
                    (None, _) => AppliesTo::Organization,
                    (Some(_), true) => AppliesTo::Project,
                    (Some(_), false) => AppliesTo::ProjectAndSubProjects,
                },
            }
        })
        .collect();
    let roles: Vec<RoleData> = world
        .roles
        .iter()
        .zip(masks)
        .enumerate()
        .map(|(at, (id, mask))| RoleData {
            id: *id,
            name: format!("Role {at}"),
            position: i64::try_from(at).unwrap(),
            permissions: permissions_of(mask),
            two_step: false,
        })
        .collect();
    // Every role may change every field and see every kind of people information, so a key that
    // only sees has something to be capped from.
    let levels = LevelRules {
        fields: world
            .roles
            .iter()
            .flat_map(|role| {
                Field::ALL.iter().map(|field| FieldRule {
                    project: None,
                    role: *role,
                    subject: *field,
                    level: FieldLevel::CanChange,
                })
            })
            .collect(),
        people: world
            .roles
            .iter()
            .flat_map(|role| {
                PeopleKind::ALL.iter().map(|kind| PeopleRule {
                    project: None,
                    role: *role,
                    subject: *kind,
                    level: PeopleLevel::Names,
                })
            })
            .collect(),
    };
    Inputs {
        person: world.person,
        place: match place {
            0 => Place::Organization,
            at => Place::Project(world.projects[at - 1]),
        },
        chain: match place {
            0 => Vec::new(),
            at => world.projects[..at].to_vec(),
        },
        rows,
        roles,
        rules: levels,
    }
}

/// The key as the store keeps it.
fn key_record(
    world: &World,
    choice: KeyChoice,
    permissions: &BTreeSet<Permission>,
    projects: Option<u8>,
) -> AccessKeyRecord {
    let mut ids = SequentialIds::default();
    AccessKeyRecord {
        key: AccessKey {
            id: AccessKeyId::generate(&mut ids),
            owner: world.person,
            name: "A key".to_owned(),
            prefix: "00000000".to_owned(),
            secret_hash: Digest::new([1; 32]),
            choice,
            created_at: Timestamp::UNIX_EPOCH,
            expires_at: Timestamp::UNIX_EPOCH,
            last_used_at: None,
        },
        permissions: if choice == KeyChoice::Permissions {
            permissions.iter().copied().collect()
        } else {
            Vec::new()
        },
        projects: projects.map_or_else(Vec::new, |mask| {
            world
                .projects
                .iter()
                .enumerate()
                .filter(|(at, _)| mask & (1 << at) != 0)
                .map(|(_, project)| *project)
                .collect()
        }),
    }
}

/// The kinds a choice keeps (for the three plain choices).
fn kinds_of(choice: KeyChoice) -> Option<BTreeSet<Kind>> {
    match choice {
        KeyChoice::SeeOnly => Some(BTreeSet::from([Kind::See])),
        KeyChoice::SeeAndChange => Some(BTreeSet::from([Kind::See, Kind::Change])),
        KeyChoice::Everything => Some(Kind::ALL.iter().copied().collect()),
        KeyChoice::Permissions => None,
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(600))]

    #[test]
    fn auth_07_a_key_can_do_only_what_its_owner_can_and_its_scopes_allow(
        masks in prop::array::uniform2(0_u16..(1 << 14)),
        rows in prop::collection::vec(row_strategy(), 0..8),
        place in 0..4_usize,
        choice in 0..4_usize,
        picked in 0_u16..(1 << 14),
        // A key with projects has at least one; a key without a list counts in every project.
        projects in prop::option::of(1_u8..8),
    ) {
        let world = World::new();
        let owner = evaluate(&owner_inputs(&world, masks, &rows, place));
        let choice = KeyChoice::ALL[choice];
        let picked = permissions_of(picked);
        let record = key_record(&world, choice, &picked, projects);
        let key = owner.narrowed(&narrowing_of(&record));

        // Never more than the owner.
        prop_assert!(key.permissions().is_subset(owner.permissions()));
        for field in Field::ALL {
            prop_assert!(key.field_level(*field) <= owner.field_level(*field));
        }

        // Only what the choice keeps.
        let kinds = kinds_of(choice);
        let keeps = |permission: &Permission| {
            permission.keys_may_hold()
                && match &kinds {
                    Some(kinds) => kinds.contains(&permission.kind()),
                    None => picked.contains(permission),
                }
        };
        prop_assert!(key.permissions().iter().all(keeps));
        // Never what keys do not hold (inviting people, changing roles), whatever the choice.
        prop_assert!(key.permissions().iter().all(|permission| !NOT_FOR_KEYS.contains(permission)));

        // Only inside its projects (and their sub-projects), and nothing for the whole
        // organization when it has projects.
        let inside = match (projects, place) {
            (None, _) => true,
            (Some(_), 0) => false,
            (Some(mask), at) => (0..at).any(|index| mask & (1 << index) != 0),
        };
        if !inside {
            prop_assert!(key.permissions().is_empty());
        }

        // And everything the owner has that the key may keep, where it counts, is kept: a key
        // is the owner narrowed, nothing is lost for no reason.
        let expected: BTreeSet<Permission> = owner
            .permissions()
            .iter()
            .copied()
            .filter(|permission| inside && keeps(permission))
            .collect();
        prop_assert_eq!(key.permissions(), &expected);

        // A key that cannot change anything shows no field above *Can see*.
        let may_change = key
            .permissions()
            .iter()
            .any(|permission| permission.kind() != Kind::See);
        let can_change = match &kinds {
            Some(kinds) => kinds.contains(&Kind::Change) || kinds.contains(&Kind::Manage),
            None => picked
                .iter()
                .any(|permission| permission.keys_may_hold() && permission.kind() != Kind::See),
        };
        if !can_change {
            prop_assert!(!may_change);
            for field in Field::ALL {
                prop_assert!(key.field_level(*field) <= FieldLevel::CanSee);
            }
        }
    }
}

/// `held_in` and `ensure_may_give_in` judge a person without their access key, so the use cases
/// that call them must be ones a key can never do: each needs a permission keys never hold.
#[test]
fn what_the_change_rules_judge_without_the_key_needs_a_permission_keys_never_hold() {
    // The permissions the callers require: roles (create, copy, change, delete, levels), who has
    // which role, and groups.
    for permission in [
        Permission::ManageRoles,
        Permission::DecideRoles,
        Permission::DecideWhatRolesSee,
        Permission::ManageGroups,
    ] {
        assert!(
            NOT_FOR_KEYS.contains(&permission),
            "{permission:?} is something an access key could do"
        );
    }
}
