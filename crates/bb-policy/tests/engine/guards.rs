//! The rules that keep the organization from locking itself out (ADR-0025 decision 6): the last
//! administrator, and no escalation.

use std::collections::BTreeSet;

use bb_core::access::{AppliesTo, Assignment};
use bb_core::id::{AssignmentId, GroupId, RoleId, SequentialIds, UserId};
use bb_policy::guards::{
    Holder, OrganizationState, Refusal, RowForNobody, anybody_manages_roles,
    ensure_a_manager_remains, ensure_no_escalation, role_managers,
};
use bb_policy::{Permission, Policy, RoleData};
use jiff::Timestamp;

use crate::common::World;

struct Org {
    admin: RoleId,
    deciders: RoleId,
    managers: RoleId,
    ana: UserId,
    bob: UserId,
    state: OrganizationState,
}

fn role(ids: &mut SequentialIds, name: &str, permissions: &[Permission]) -> RoleData {
    RoleData {
        id: RoleId::generate(ids).unwrap(),
        name: name.to_owned(),
        position: 0,
        permissions: permissions.iter().copied().collect(),
        two_step: false,
    }
}

/// Ana is an administrator herself; Bob is one through the group `Operators`.
fn org() -> Org {
    let mut ids = SequentialIds::default();
    let admin = role(&mut ids, "Administrator", Permission::ALL);
    let deciders = role(&mut ids, "Deciders", &[Permission::DecideRoles]);
    let managers = role(&mut ids, "Managers", &[Permission::ManageRoles]);
    let ana = UserId::generate(&mut ids);
    let bob = UserId::generate(&mut ids);
    let group = GroupId::generate(&mut ids).unwrap();
    let state = OrganizationState {
        rows: vec![
            (Holder::Person(ana), admin.id),
            (Holder::Group(group), admin.id),
        ],
        members: vec![(group, bob)],
        roles: vec![admin.clone(), deciders.clone(), managers.clone()],
    };
    Org {
        admin: admin.id,
        deciders: deciders.id,
        managers: managers.id,
        ana,
        bob,
        state,
    }
}

#[test]
fn sec_08_the_people_who_manage_roles_hold_both_permissions_through_their_row_or_a_group() {
    let mut org = org();
    assert_eq!(
        role_managers(&org.state),
        BTreeSet::from([org.ana, org.bob])
    );
    // Both permissions may come from two different roles of the same person.
    org.state.rows = vec![
        (Holder::Person(org.ana), org.deciders),
        (Holder::Person(org.ana), org.managers),
        (Holder::Person(org.bob), org.deciders),
    ];
    assert_eq!(role_managers(&org.state), BTreeSet::from([org.ana]));
    org.state.rows.clear();
    assert!(!anybody_manages_roles(&org.state));
}

#[test]
fn sec_08_nobody_can_remove_the_last_way_to_manage_roles() {
    let org = org();
    // Taking away one of two ways is fine.
    let mut after = org.state.clone();
    after
        .rows
        .retain(|(holder, _)| *holder != Holder::Person(org.ana));
    assert_eq!(ensure_a_manager_remains(&org.state, &after), Ok(()));
    // Removing the row of the group as well leaves nobody.
    let mut nobody = after.clone();
    nobody.rows.clear();
    assert_eq!(
        ensure_a_manager_remains(&after, &nobody),
        Err(Refusal::WouldLockOut)
    );
    // Taking the last member out of the group that holds the role.
    let mut emptied = after.clone();
    emptied.members.clear();
    assert_eq!(
        ensure_a_manager_remains(&after, &emptied),
        Err(Refusal::WouldLockOut)
    );
    // Deleting the group's role, or taking a permission out of it.
    let mut deleted = after.clone();
    deleted.roles.retain(|role| role.id != org.admin);
    assert_eq!(
        ensure_a_manager_remains(&after, &deleted),
        Err(Refusal::WouldLockOut)
    );
    let mut weaker = after.clone();
    for role in &mut weaker.roles {
        role.permissions.remove(&Permission::ManageRoles);
    }
    assert_eq!(
        ensure_a_manager_remains(&after, &weaker),
        Err(Refusal::WouldLockOut)
    );
}

#[test]
fn sec_08_a_fresh_installation_with_nobody_is_no_reason_to_refuse_a_change() {
    let mut fresh = org().state;
    fresh.rows.clear();
    assert!(!anybody_manages_roles(&fresh));
    assert_eq!(ensure_a_manager_remains(&fresh, &fresh), Ok(()));
}

fn held_at_organization(permissions: &[Permission]) -> bb_policy::Evaluation {
    let mut world = World::new();
    let role = world.role("Some role", permissions);
    world.give_everywhere(role);
    world.at_organization()
}

#[test]
fn sec_08_nobody_adds_a_permission_they_do_not_hold_to_a_role() {
    let me = held_at_organization(&[Permission::ManageRoles, Permission::SeeProjects]);
    let wanted = |permissions: &[Permission]| permissions.iter().copied().collect();
    assert_eq!(
        ensure_no_escalation(&me, &wanted(&[Permission::SeeProjects])),
        Ok(())
    );
    assert_eq!(
        ensure_no_escalation(
            &me,
            &wanted(&[Permission::SeeProjects, Permission::ManagePeople])
        ),
        Err(Refusal::TooMuch)
    );
    assert_eq!(
        ensure_no_escalation(&me, &wanted(&[Permission::ManageRoles])),
        Ok(()),
        "they may add what they hold themselves"
    );
    assert_eq!(ensure_no_escalation(&me, &BTreeSet::new()), Ok(()));
}

#[test]
fn sec_02_nobody_gives_a_role_with_more_than_they_hold_there() {
    let mut world = World::new();
    let lead = world.role(
        "Project lead",
        &[
            Permission::SeeProjects,
            Permission::DecideRoles,
            Permission::MakeSubProjects,
        ],
    );
    world.give(lead, world.acme);
    let admin_permissions: BTreeSet<Permission> = Permission::ALL.iter().copied().collect();
    let view: BTreeSet<Permission> = BTreeSet::from([Permission::SeeProjects]);
    let held = world.at(world.web);
    // A role they hold all of: fine. The Administrator role: too much.
    assert_eq!(ensure_no_escalation(&held, &view), Ok(()));
    assert_eq!(
        ensure_no_escalation(&held, &admin_permissions),
        Err(Refusal::TooMuch)
    );
    // Organization permissions count nowhere in a project, so a role that holds them is compared
    // without them for a row on a project: a lead may give a role with Manage people on a
    // project if they hold the project permissions of it, because it gives nothing there.
    let with_organization = BTreeSet::from([Permission::SeeProjects, Permission::ManagePeople]);
    assert_eq!(ensure_no_escalation(&held, &with_organization), Ok(()));
    // For the whole organization the comparison is complete.
    let organization = held_at_organization(&[Permission::SeeProjects]);
    assert_eq!(
        ensure_no_escalation(&organization, &with_organization),
        Err(Refusal::TooMuch)
    );
}

#[test]
fn sec_08_somebody_who_only_manages_roles_does_not_keep_the_organization_open() {
    let mut org = org();
    // Ana is the only one with both halves. Bob holds only *Manage roles*, through his own row,
    // and the people of a group hold only *Manage roles* too.
    // Numbers that the organization above did not use.
    let mut ids = SequentialIds::default();
    for _ in 0..20 {
        let _ = UserId::generate(&mut ids);
    }
    let group = GroupId::generate(&mut ids).unwrap();
    let cleo = UserId::generate(&mut ids);
    org.state.rows = vec![
        (Holder::Person(org.ana), org.admin),
        (Holder::Person(org.bob), org.managers),
        (Holder::Group(group), org.managers),
    ];
    org.state.members = vec![(group, cleo)];
    assert_eq!(role_managers(&org.state), BTreeSet::from([org.ana]));
    // So taking Ana's row away leaves nobody, whoever else can manage roles.
    let mut after = org.state.clone();
    after
        .rows
        .retain(|(holder, _)| *holder != Holder::Person(org.ana));
    assert!(!anybody_manages_roles(&after));
    assert_eq!(
        ensure_a_manager_remains(&org.state, &after),
        Err(Refusal::WouldLockOut)
    );
}

fn row(
    ids: &mut SequentialIds,
    person: Option<UserId>,
    group: Option<GroupId>,
    role: RoleId,
) -> Assignment {
    Assignment {
        id: AssignmentId::generate(ids).unwrap(),
        person,
        group,
        role: Some(role),
        project: None,
        applies_to: AppliesTo::Organization,
        given_at: Timestamp::UNIX_EPOCH,
    }
}

#[test]
fn sec_08_the_organization_state_is_made_from_rows_and_a_row_for_nobody_is_refused() {
    let mut ids = SequentialIds::default();
    let admin = role(&mut ids, "Administrator", Permission::ALL);
    let ana = UserId::generate(&mut ids);
    let group = GroupId::generate(&mut ids).unwrap();
    let mut policy = Policy::default();
    policy.roles.push(admin.clone());
    policy
        .assignments
        .push(row(&mut ids, Some(ana), None, admin.id));
    policy
        .assignments
        .push(row(&mut ids, None, Some(group), admin.id));
    let state = OrganizationState::of(&policy).unwrap();
    assert_eq!(
        state.rows,
        [
            (Holder::Person(ana), admin.id),
            (Holder::Group(group), admin.id)
        ]
    );
    assert_eq!(state.roles, std::slice::from_ref(&admin));
    assert_eq!(state.members, []);

    // A row for nobody is broken data: an error, never a row that is left out. Even when it is
    // for a project, or gives *No access*, so it would not count.
    for broken in [
        row(&mut ids, None, None, admin.id),
        Assignment {
            project: Some(bb_core::id::ProjectId::generate(&mut ids).unwrap()),
            role: None,
            ..row(&mut ids, None, None, admin.id)
        },
    ] {
        assert_eq!(Holder::try_from(&broken), Err(RowForNobody));
        let mut policy = policy.clone();
        policy.assignments.push(broken);
        assert_eq!(OrganizationState::of(&policy), Err(RowForNobody));
    }
}
