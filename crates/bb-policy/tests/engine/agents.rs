//! An agent never does more than its person (AI-30, ADR-0029, decision 2): its evaluation is the
//! person's, intersected with its work, without a permission of `NOT_FOR_AGENTS`, and only where
//! the person holds *Create agents that work for you*.

use std::collections::BTreeSet;

use bb_core::access::{AppliesTo, Assignment};
use bb_core::id::{AssignmentId, ProjectId, RoleId, SequentialIds, UserId};
use bb_policy::{
    NOT_FOR_AGENTS, Narrowing, Permission, PermissionBits, Place, Policy, RoleData, Tree,
};
use jiff::Timestamp;
use proptest::prelude::*;

/// ```text
/// WEB ─ API        OTHER
/// ```
struct World {
    policy: Policy,
    tree: Tree,
    person: UserId,
    web: ProjectId,
    api: ProjectId,
    other: ProjectId,
}

/// The person holds `permissions` for the whole organization.
fn world(permissions: BTreeSet<Permission>) -> World {
    let mut ids = SequentialIds::default();
    let person = UserId::generate(&mut ids);
    let web = ProjectId::generate(&mut ids).unwrap();
    let api = ProjectId::generate(&mut ids).unwrap();
    let other = ProjectId::generate(&mut ids).unwrap();
    let role = RoleId::generate(&mut ids).unwrap();
    let mut policy = Policy::default();
    policy.roles.push(RoleData {
        id: role,
        name: "Role".to_owned(),
        position: 0,
        permissions,
        two_step: false,
    });
    policy.assignments.push(Assignment {
        id: AssignmentId::generate(&mut ids).unwrap(),
        person: Some(person),
        group: None,
        role: Some(role),
        project: None,
        applies_to: AppliesTo::Organization,
        given_at: Timestamp::UNIX_EPOCH,
    });
    let tree = Tree::new([(web, None), (api, Some(web)), (other, None)]);
    World {
        policy,
        tree,
        person,
        web,
        api,
        other,
    }
}

fn held_by_agent(world: &World, narrowing: &Narrowing, project: ProjectId) -> BTreeSet<Permission> {
    world
        .policy
        .evaluate_as(
            world.person,
            Some(narrowing),
            &world.tree,
            Place::Project(project),
        )
        .map(|found| found.permissions().clone())
        .unwrap_or_default()
}

fn team_member() -> BTreeSet<Permission> {
    BTreeSet::from([
        Permission::SeeProjects,
        Permission::Comment,
        Permission::CreateAgents,
        Permission::ChangeProjectSettings,
        Permission::EditAnyway,
        Permission::DecideRoles,
    ])
}

#[test]
fn ai_30_an_agent_never_holds_a_manage_permission() {
    let world = world(team_member());
    let agent = Narrowing::agent(BTreeSet::from([world.web]));
    let held = held_by_agent(&world, &agent, world.web);
    assert_eq!(
        held,
        BTreeSet::from([Permission::SeeProjects, Permission::Comment])
    );
    assert!(held.iter().all(|permission| permission.agents_may_hold()));
}

#[test]
fn ai_30_without_work_an_agent_counts_nowhere() {
    let world = world(team_member());
    let agent = Narrowing::agent(BTreeSet::new());
    for project in [world.web, world.api, world.other] {
        assert!(held_by_agent(&world, &agent, project).is_empty());
    }
}

#[test]
fn ai_30_the_work_of_an_agent_counts_in_its_projects_and_below_only() {
    let world = world(team_member());
    let agent = Narrowing::agent(BTreeSet::from([world.web]));
    assert!(!held_by_agent(&world, &agent, world.api).is_empty());
    assert!(held_by_agent(&world, &agent, world.other).is_empty());
}

#[test]
fn ai_30_an_agent_counts_only_where_its_person_may_create_agents() {
    let mut without = team_member();
    without.remove(&Permission::CreateAgents);
    let world = world(without);
    let agent = Narrowing::agent(BTreeSet::from([world.web]));
    assert!(held_by_agent(&world, &agent, world.web).is_empty());
}

proptest! {
    #[test]
    fn ai_30_an_agent_never_does_more_than_its_person(
        bits in 0..Permission::every_set().end,
        work in proptest::collection::btree_set(0_usize..3, 0..3),
        at in 0_usize..3,
    ) {
        let mut permissions = Permission::set_of(bits as PermissionBits);
        permissions.insert(Permission::SeeProjects);
        let world = world(permissions);
        let projects = [world.web, world.api, world.other];
        let agent = Narrowing::agent(work.iter().map(|at| projects[*at]).collect());
        let project = projects[at];
        let theirs = world
            .policy
            .evaluate_at(world.person, &world.tree, Place::Project(project))
            .map(|found| found.permissions().clone())
            .unwrap_or_default();
        let held = held_by_agent(&world, &agent, project);
        prop_assert!(held.is_subset(&theirs));
        prop_assert!(held.iter().all(|permission| !NOT_FOR_AGENTS.contains(permission)));
    }
}
