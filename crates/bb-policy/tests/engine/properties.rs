//! Properties of the engine over many random organizations (SEC-03, AUTH-07, SEC-04):
//! a new role never takes a permission away, *No access* never adds one, and any scope list gives
//! at most what the owner has. Check access explains every result.

use std::collections::BTreeSet;

use bb_core::access::{AppliesTo, FieldLevel, PeopleLevel};
use bb_core::fields::Field;
use bb_core::id::{AssignmentId, GroupId, ProjectId, RoleId, SequentialIds, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_policy::{
    Choice, Evaluation, FieldRule, Inputs, Kind, Narrowing, PeopleRule, Permission, Place, Reason,
    RoleData, Row, Who, evaluate,
};
use proptest::prelude::*;

/// The tree of the properties: ACME, WEB inside it, API inside that.
struct Tree {
    person: UserId,
    projects: [ProjectId; 3],
    roles: [RoleId; 3],
    groups: [GroupId; 2],
}

impl Tree {
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
                RoleId::generate(&mut ids).unwrap(),
            ],
            groups: [
                GroupId::generate(&mut ids).unwrap(),
                GroupId::generate(&mut ids).unwrap(),
            ],
        }
    }
}

#[derive(Clone, Debug)]
struct RowSpec {
    group: Option<usize>,
    role: Option<usize>,
    project: Option<usize>,
    here_only: bool,
}

#[derive(Clone, Debug)]
struct RuleSpec {
    project: Option<usize>,
    role: usize,
    field: usize,
    level: usize,
}

#[derive(Clone, Debug)]
struct Spec {
    role_permissions: [u16; 3],
    rows: Vec<RowSpec>,
    rules: Vec<RuleSpec>,
    place: usize,
}

fn row_strategy() -> impl Strategy<Value = RowSpec> {
    (
        prop::option::of(0..2_usize),
        prop::option::of(0..3_usize),
        prop::option::of(0..3_usize),
        any::<bool>(),
    )
        .prop_map(|(group, role, project, here_only)| RowSpec {
            group,
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

fn spec_strategy() -> impl Strategy<Value = Spec> {
    (
        prop::array::uniform3(0_u16..(1 << 14)),
        prop::collection::vec(row_strategy(), 0..8),
        prop::collection::vec(
            (
                prop::option::of(0..3_usize),
                0..3_usize,
                0..16_usize,
                0..3_usize,
            )
                .prop_map(|(project, role, field, level)| RuleSpec {
                    project,
                    role,
                    field,
                    level,
                }),
            0..10,
        ),
        0..4_usize,
    )
        .prop_map(|(role_permissions, rows, rules, place)| Spec {
            role_permissions,
            rows,
            rules,
            place,
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

fn row_of(tree: &Tree, number: u64, spec: &RowSpec) -> Row {
    let mut ids = SequentialIds::default();
    for _ in 0..number {
        let _ = AssignmentId::generate(&mut ids);
    }
    Row {
        id: AssignmentId::generate(&mut ids).unwrap(),
        who: spec
            .group
            .map_or(Who::Person, |at| Who::Group(tree.groups[at])),
        role: spec.role.map(|at| tree.roles[at]),
        project: spec.project.map(|at| tree.projects[at]),
        applies_to: match (spec.project, spec.role, spec.here_only) {
            (None, _, _) => AppliesTo::Organization,
            (Some(_), None, _) | (Some(_), Some(_), false) => AppliesTo::ProjectAndSubProjects,
            (Some(_), Some(_), true) => AppliesTo::Project,
        },
    }
}

fn inputs(tree: &Tree, spec: &Spec, extra: Option<&RowSpec>) -> Inputs {
    let place = match spec.place {
        0 => Place::Organization,
        at => Place::Project(tree.projects[at - 1]),
    };
    let chain = match spec.place {
        0 => Vec::new(),
        at => tree.projects[..at].to_vec(),
    };
    let mut rows: Vec<Row> = spec
        .rows
        .iter()
        .zip(1_u64..)
        .map(|(row, number)| row_of(tree, number, row))
        .collect();
    if let Some(extra) = extra {
        rows.push(row_of(tree, 1000, extra));
    }
    let roles = tree
        .roles
        .iter()
        .zip(spec.role_permissions)
        .enumerate()
        .map(|(at, (id, mask))| RoleData {
            id: *id,
            name: format!("Role {at}"),
            position: i64::try_from(at).unwrap(),
            permissions: permissions_of(mask),
            two_step: false,
        })
        .collect();
    let field_rules = spec
        .rules
        .iter()
        .map(|rule| FieldRule {
            project: rule.project.map(|at| tree.projects[at]),
            role: tree.roles[rule.role],
            field: Field::ALL[rule.field],
            level: FieldLevel::ALL[rule.level],
        })
        .collect();
    let people_rules = spec
        .rules
        .iter()
        .map(|rule| PeopleRule {
            project: rule.project.map(|at| tree.projects[at]),
            role: tree.roles[rule.role],
            kind: PeopleKind::ALL[rule.field % PeopleKind::ALL.len()],
            level: PeopleLevel::ALL[rule.level],
        })
        .collect();
    Inputs {
        person: tree.person,
        place,
        chain,
        rows,
        roles,
        field_rules,
        people_rules,
    }
}

fn narrowing_strategy() -> impl Strategy<Value = (u8, u16, Option<u8>)> {
    (0_u8..8, 0_u16..(1 << 14), prop::option::of(0_u8..8))
}

fn narrowing_of(
    tree: &Tree,
    kinds: u8,
    permissions: u16,
    projects: Option<u8>,
    by_kind: bool,
) -> Narrowing {
    let choice = if by_kind {
        Choice::Kinds(
            Kind::ALL
                .iter()
                .enumerate()
                .filter(|(at, _)| kinds & (1 << at) != 0)
                .map(|(_, kind)| *kind)
                .collect(),
        )
    } else {
        Choice::Permissions(permissions_of(permissions))
    };
    let narrowing = Narrowing::new(choice);
    match projects {
        None => narrowing,
        Some(mask) => narrowing.in_projects(
            tree.projects
                .iter()
                .enumerate()
                .filter(|(at, _)| mask & (1 << at) != 0)
                .map(|(_, project)| *project)
                .collect(),
        ),
    }
}

fn levels(evaluation: &Evaluation) -> (Vec<FieldLevel>, Vec<PeopleLevel>) {
    (
        Field::ALL
            .iter()
            .map(|field| evaluation.field_level(*field))
            .collect(),
        PeopleKind::ALL
            .iter()
            .map(|kind| evaluation.kind_level(*kind))
            .collect(),
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]

    #[test]
    fn sec_03_a_new_role_never_takes_a_permission_or_a_level_away(
        spec in spec_strategy(),
        extra in row_strategy(),
    ) {
        prop_assume!(extra.role.is_some());
        let tree = Tree::new();
        let before = evaluate(&inputs(&tree, &spec, None));
        let after = evaluate(&inputs(&tree, &spec, Some(&extra)));
        prop_assert!(after.permissions().is_superset(before.permissions()));
        let (fields_before, people_before) = levels(&before);
        let (fields_after, people_after) = levels(&after);
        prop_assert!(fields_after.iter().zip(&fields_before).all(|(a, b)| a >= b));
        prop_assert!(people_after.iter().zip(&people_before).all(|(a, b)| a >= b));
    }

    #[test]
    fn sec_03_no_access_never_adds_a_permission_or_a_level(
        spec in spec_strategy(),
        extra in row_strategy(),
    ) {
        prop_assume!(extra.role.is_none());
        let tree = Tree::new();
        let before = evaluate(&inputs(&tree, &spec, None));
        let after = evaluate(&inputs(&tree, &spec, Some(&extra)));
        prop_assert!(after.permissions().is_subset(before.permissions()));
        let (fields_before, people_before) = levels(&before);
        let (fields_after, people_after) = levels(&after);
        prop_assert!(fields_after.iter().zip(&fields_before).all(|(a, b)| a <= b));
        prop_assert!(people_after.iter().zip(&people_before).all(|(a, b)| a <= b));
    }

    #[test]
    fn auth_07_any_scope_list_gives_at_most_the_owners_permissions(
        spec in spec_strategy(),
        (kinds, permissions, projects) in narrowing_strategy(),
        by_kind in any::<bool>(),
    ) {
        let tree = Tree::new();
        let owner = evaluate(&inputs(&tree, &spec, None));
        let key = narrowing_of(&tree, kinds, permissions, projects, by_kind);
        let narrowed = owner.narrowed(&key);
        prop_assert!(narrowed.permissions().is_subset(owner.permissions()));
        let (owner_fields, owner_people) = levels(&owner);
        let (key_fields, key_people) = levels(&narrowed);
        prop_assert!(key_fields.iter().zip(&owner_fields).all(|(k, o)| k <= o));
        prop_assert!(key_people.iter().zip(&owner_people).all(|(k, o)| k <= o));
        if by_kind {
            let chosen: BTreeSet<Kind> = Kind::ALL
                .iter()
                .enumerate()
                .filter(|(at, _)| kinds & (1 << at) != 0)
                .map(|(_, kind)| *kind)
                .collect();
            prop_assert!(narrowed.permissions().iter().all(|p| chosen.contains(&p.kind())));
        }
        // Narrowing twice is narrowing once: a key cannot be widened by another key.
        let twice = narrowed.narrowed(&key);
        prop_assert_eq!(twice.permissions(), narrowed.permissions());
    }

    #[test]
    fn sec_03_check_access_explains_every_result_with_the_chain_of_reasons(
        spec in spec_strategy(),
        (kinds, permissions, projects) in narrowing_strategy(),
        narrowed in any::<bool>(),
    ) {
        let tree = Tree::new();
        let mut evaluation = evaluate(&inputs(&tree, &spec, None));
        if narrowed {
            evaluation = evaluation.narrowed(&narrowing_of(&tree, kinds, permissions, projects, true));
        }
        let report = evaluation.report();
        prop_assert_eq!(report.permissions.len(), Permission::ALL.len());
        for decision in &report.permissions {
            // The decision is the engine's.
            prop_assert_eq!(decision.allowed, evaluation.allows(decision.permission));
            prop_assert!(!decision.reasons.is_empty());
            let gives = decision.reasons.iter().any(|reason| matches!(
                reason,
                Reason::GivenByRole { .. } | Reason::KeptFromRemovedRow { .. }
            ));
            let key_says_no = decision.reasons.iter().any(|reason| matches!(
                reason,
                Reason::NarrowedByKey | Reason::OutsideKeyProjects
            ));
            // And the reasons lead to it: a yes has a row that gives it and no key that says no, a
            // no has no row that gives it or a key that says no.
            prop_assert_eq!(decision.allowed, gives && !key_says_no, "{:?}", decision);
            prop_assert!(narrowed || !key_says_no);
        }
        for decision in &report.fields {
            prop_assert_eq!(decision.level, evaluation.field_level(decision.subject));
        }
        for decision in &report.kinds {
            prop_assert_eq!(decision.level, evaluation.kind_level(decision.subject));
        }
    }
}
