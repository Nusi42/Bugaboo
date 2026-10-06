//! An access key narrows what its owner sees in the tree, everywhere the engine decides it
//! (ADR-0025 decision 8, M2-F27): a key for a project lists that project and what is inside it,
//! and nothing above or beside it.

use std::collections::BTreeSet;

use bb_core::access::{AppliesTo, Assignment};
use bb_core::id::{AssignmentId, ProjectId, RoleId, SequentialIds, UserId};
use bb_policy::{Narrowing, Permission, Place, Policy, RoleData, Tree};
use jiff::Timestamp;

/// ```text
/// ACME ─ WEB ─ API        OTHER
/// ```
struct World {
    policy: Policy,
    tree: Tree,
    owner: UserId,
    acme: ProjectId,
    web: ProjectId,
    api: ProjectId,
    other: ProjectId,
}

/// The owner is a project lead for the whole organization, so they see and may do everything.
fn world() -> World {
    let mut ids = SequentialIds::default();
    let owner = UserId::generate(&mut ids);
    let acme = ProjectId::generate(&mut ids).unwrap();
    let web = ProjectId::generate(&mut ids).unwrap();
    let api = ProjectId::generate(&mut ids).unwrap();
    let other = ProjectId::generate(&mut ids).unwrap();
    let role = RoleId::generate(&mut ids).unwrap();
    let mut policy = Policy::default();
    policy.roles.push(RoleData {
        id: role,
        name: "Project lead".to_owned(),
        position: 0,
        permissions: BTreeSet::from([
            Permission::SeeProjects,
            Permission::MakeSubProjects,
            Permission::DecideRoles,
        ]),
        two_step: false,
    });
    policy.assignments.push(Assignment {
        id: AssignmentId::generate(&mut ids).unwrap(),
        person: Some(owner),
        group: None,
        role: Some(role),
        project: None,
        applies_to: AppliesTo::Organization,
        given_at: Timestamp::UNIX_EPOCH,
    });
    let tree = Tree::new([
        (acme, None),
        (web, Some(acme)),
        (api, Some(web)),
        (other, None),
    ]);
    World {
        policy,
        tree,
        owner,
        acme,
        web,
        api,
        other,
    }
}

#[test]
fn auth_07_a_key_for_a_project_sees_that_project_and_below_and_nothing_else() {
    let world = world();
    let everything = world.policy.visible(world.owner, &world.tree);
    assert_eq!(
        everything,
        BTreeSet::from([world.acme, world.web, world.api, world.other])
    );
    // A key for WEB: WEB and API, not ACME above it and not OTHER beside it.
    let key = Narrowing::see_only().in_projects(BTreeSet::from([world.web]));
    let seen = world
        .policy
        .visible_as(world.owner, Some(&key), &world.tree);
    assert_eq!(seen, BTreeSet::from([world.web, world.api]));
    // No key: the same as before.
    assert_eq!(
        world.policy.visible_as(world.owner, None, &world.tree),
        everything
    );
    // A key without projects sees what its owner sees.
    let all = Narrowing::see_only();
    assert_eq!(
        world
            .policy
            .visible_as(world.owner, Some(&all), &world.tree),
        everything
    );
}

#[test]
fn auth_07_a_key_for_another_project_sees_nothing_of_this_one() {
    let world = world();
    let key = Narrowing::everything().in_projects(BTreeSet::from([world.other]));
    let seen = world
        .policy
        .visible_as(world.owner, Some(&key), &world.tree);
    assert_eq!(seen, BTreeSet::from([world.other]));
}

#[test]
fn auth_07_the_evaluation_of_the_actor_is_narrowed_and_the_owners_own_is_not() {
    let world = world();
    let key = Narrowing::see_only().in_projects(BTreeSet::from([world.web]));
    let place = |project| Place::Project(project);
    let narrowed = |project| {
        world
            .policy
            .evaluate_as(world.owner, Some(&key), &world.tree, place(project))
            .unwrap()
    };
    // Inside the key's project: what the key's choice keeps (See only: no deciding).
    assert!(narrowed(world.api).sees_project());
    assert!(!narrowed(world.api).allows(Permission::DecideRoles));
    // Above and beside it: not even the knowledge that the project exists.
    for project in [world.acme, world.other] {
        assert!(!narrowed(project).sees_project(), "{project}");
        assert!(!narrowed(project).knows_project(), "{project}");
    }
    // The owner's own evaluation is the engine's, whole: other people are judged with it.
    let owner = world
        .policy
        .evaluate_at(world.owner, &world.tree, place(world.acme))
        .unwrap();
    assert!(owner.sees_project() && owner.allows(Permission::DecideRoles));
    // A place that is not in the tree is no place, for a key as for a person.
    let nowhere = ProjectId::generate(&mut SequentialIds::default()).unwrap();
    assert!(
        world
            .policy
            .evaluate_as(world.owner, Some(&key), &world.tree, place(nowhere))
            .is_none()
    );
}
