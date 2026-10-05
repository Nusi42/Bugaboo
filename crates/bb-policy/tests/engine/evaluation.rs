//! The rules of ADR-0025 decision 3: default deny, inheritance, *No access* and rank.

use bb_core::id::UserId;
use bb_policy::{Permission, Reason};

use crate::common::{World, keys};

const VIEW: [Permission; 1] = [Permission::SeeProjects];

#[test]
fn sec_01_a_person_without_a_role_can_do_nothing_and_sees_no_project() {
    let mut world = World::new();
    // Roles exist, but nobody gave one to this person.
    world.role("Administrator", Permission::ALL);
    for project in [world.acme, world.web, world.api, world.other] {
        let evaluation = world.at(project);
        assert!(evaluation.permissions().is_empty());
        assert!(!evaluation.sees_project());
        assert!(!evaluation.knows_project());
    }
    let organization = world.at_organization();
    assert!(organization.permissions().is_empty());
    assert!(organization.roles().is_empty());
    // A role that nobody can read does not exist: the explanation says so.
    let decision = organization.explain(Permission::SeeProjects);
    assert!(!decision.allowed);
    assert_eq!(decision.reasons, [Reason::NoRoleGivesIt]);
}

#[test]
fn sec_03_a_role_on_a_project_applies_to_its_sub_projects_unless_given_for_this_project_only() {
    let mut world = World::new();
    let viewer = world.role("Viewer", &VIEW);
    let row = world.give(viewer, world.acme);
    for project in [world.acme, world.web, world.api] {
        assert!(world.at(project).sees_project(), "inherited down the tree");
    }
    assert!(
        !world.at(world.other).sees_project(),
        "not in a sibling tree"
    );
    assert!(world.at(world.web).trace()[0].inherited);
    assert!(!world.at(world.acme).trace()[0].inherited);

    // Given for this project only: the sub-projects do not get it.
    world.rows.retain(|existing| existing.id != row);
    world.give_here_only(viewer, world.acme);
    assert!(world.at(world.acme).sees_project());
    assert!(!world.at(world.web).sees_project());
    assert!(!world.at(world.api).sees_project());
}

#[test]
fn sec_03_a_role_for_the_whole_organization_counts_in_every_project() {
    let mut world = World::new();
    let lead = world.role(
        "Lead",
        &[Permission::SeeProjects, Permission::MakeSubProjects],
    );
    world.give_everywhere(lead);
    for project in [world.acme, world.web, world.api, world.other] {
        assert_eq!(
            keys(&world.at(project)),
            ["see-projects", "make-sub-projects"]
        );
    }
}

#[test]
fn sec_03_no_access_on_a_sub_project_wins_over_a_role_from_above() {
    let mut world = World::new();
    let member = world.role("Team member", &VIEW);
    world.give(member, world.acme);
    world.no_access(world.web);
    assert!(world.at(world.acme).sees_project());
    assert!(
        !world.at(world.web).sees_project(),
        "the sub-project it is set on"
    );
    assert!(
        !world.at(world.api).sees_project(),
        "and the sub-projects below it"
    );
    assert!(world.at(world.api).permissions().is_empty());
}

#[test]
fn sec_03_the_nearer_row_decides_a_role_given_lower_down_counts_again() {
    let mut world = World::new();
    let member = world.role("Team member", &VIEW);
    let more = world.role("Contributor", &[Permission::MakeSubProjects]);
    world.give(member, world.acme);
    world.no_access(world.web);
    world.give(more, world.api);
    assert!(!world.at(world.web).sees_project());
    // On API a row of its own is nearer than the No access on WEB: it still counts. The role
    // from ACME is removed, so only the role given on API itself remains.
    let api = world.at(world.api);
    assert_eq!(keys(&api), ["make-sub-projects"]);
    assert!(!api.sees_project());
    // A No access given below a role removes it only there.
    let mut world = World::new();
    let member = world.role("Team member", &VIEW);
    world.give(member, world.web);
    world.no_access(world.api);
    assert!(world.at(world.web).sees_project());
    assert!(!world.at(world.api).sees_project());
}

#[test]
fn sec_03_the_same_project_rules_of_m2_q1_person_before_group() {
    // No access for the person wins over a role of one of their groups on the same project, and
    // over another group's role there.
    let mut world = World::new();
    let team = world.group();
    let other_group = world.group();
    let member = world.role("Team member", &VIEW);
    world.give_group(team, member, world.web);
    world.give_group(other_group, member, world.web);
    world.no_access(world.web);
    assert!(!world.at(world.web).sees_project());

    // No access for a group wins over a role of that group and of another group there, but a
    // role given to the person directly on the same project still counts.
    let mut world = World::new();
    let blocked = world.group();
    let other_group = world.group();
    let member = world.role("Team member", &VIEW);
    let direct = world.role("Contributor", &[Permission::MakeSubProjects]);
    world.give_group(blocked, member, world.web);
    world.give_group(other_group, member, world.web);
    world.no_access_group(blocked, world.web);
    assert!(world.at(world.web).permissions().is_empty());
    world.give(direct, world.web);
    assert_eq!(keys(&world.at(world.web)), ["make-sub-projects"]);

    // A role from higher up is removed by a group's No access as well.
    let mut world = World::new();
    let blocked = world.group();
    let member = world.role("Team member", &VIEW);
    world.give(member, world.acme);
    world.no_access_group(blocked, world.web);
    assert!(!world.at(world.web).sees_project());
}

#[test]
fn sec_03_no_access_cannot_lock_out_the_people_who_decide_roles_above() {
    let mut world = World::new();
    let lead = world.role(
        "Project lead",
        &[
            Permission::SeeProjects,
            Permission::ChangeProjectSettings,
            Permission::DecideRoles,
        ],
    );
    let lead_row = world.give(lead, world.acme);
    let no_access = world.no_access(world.web);
    let web = world.at(world.web);
    // They cannot see inside, but can still decide who has which role, and so take it away.
    assert!(!web.sees_project());
    assert!(web.allows(Permission::DecideRoles));
    assert!(web.decide_roles_was_kept());
    assert!(
        web.knows_project(),
        "they see the name and short name in Who has which role"
    );
    assert!(!web.allows(Permission::ChangeProjectSettings));
    assert_eq!(keys(&web), ["decide-roles"]);
    let reasons = web.explain(Permission::DecideRoles).reasons;
    assert!(matches!(
        reasons.as_slice(),
        [Reason::KeptFromRemovedRow { row, no_access: by, .. }]
            if row.assignment == lead_row && by.assignment == no_access
    ));
    // Somebody whose removed role cannot decide keeps nothing.
    let mut world = World::new();
    let member = world.role("Team member", &VIEW);
    world.give(member, world.acme);
    world.no_access(world.web);
    let web = world.at(world.web);
    assert!(web.permissions().is_empty());
    assert!(!web.knows_project());
    assert!(!web.decide_roles_was_kept());
}

#[test]
fn sec_03_organization_permissions_count_only_from_a_role_for_the_whole_organization() {
    let mut world = World::new();
    let admin = world.role("Administrator", Permission::ALL);
    world.give(admin, world.acme);
    // On a project the role holds organization permissions that count nowhere there.
    let web = world.at(world.web);
    assert!(web.allows(Permission::DecideRoles));
    assert!(!web.allows(Permission::ManageRoles));
    assert!(!web.allows(Permission::SeeStatus));
    assert!(world.at_organization().permissions().is_empty());
    let decision = web.explain(Permission::ManageRoles);
    assert!(!decision.allowed);
    assert!(matches!(
        decision.reasons.as_slice(),
        [Reason::CountsOnlyForTheOrganization { .. }]
    ));
    // Given for the whole organization, it counts at the organization.
    world.give_everywhere(admin);
    let organization = world.at_organization();
    assert!(organization.allows(Permission::ManageRoles));
    assert_eq!(organization.permissions().len(), Permission::ALL.len());
    assert!(!world.at(world.web).allows(Permission::ManageRoles));
}

#[test]
fn roles_are_listed_in_the_order_of_roles_and_the_first_one_is_the_label() {
    let mut world = World::new();
    let first = world.role("Viewer", &VIEW);
    let second = world.role("Team member", &VIEW);
    world.give(second, world.acme);
    world.give(first, world.web);
    let web = world.at(world.web);
    let names: Vec<&str> = web.roles().iter().map(|role| role.name.as_str()).collect();
    assert_eq!(
        names,
        ["Viewer", "Team member"],
        "the order of roles, not the rank"
    );
    assert_eq!(web.first_role_name(), Some("Viewer"));
    // The same role through two rows is one role.
    world.give_everywhere(first);
    assert_eq!(world.at(world.web).roles().len(), 2);
}

#[test]
fn rows_of_other_people_and_other_projects_are_not_part_of_the_evaluation() {
    let mut world = World::new();
    let member = world.role("Team member", &VIEW);
    world.give(member, world.other);
    assert!(!world.at(world.acme).sees_project());
    assert!(world.at(world.other).sees_project());
    assert_eq!(world.at(world.acme).person(), Some(world.person));
    assert_ne!(
        world.at(world.acme).person(),
        Some(UserId::from_random([9; 16]))
    );
}

#[test]
fn sec_03_decide_roles_is_kept_only_from_rows_strictly_above_the_no_access() {
    let lead = |world: &mut World| {
        world.role(
            "Project lead",
            &[Permission::SeeProjects, Permission::DecideRoles],
        )
    };

    // A role of one of the person's groups at the same depth as the person's own *No access* is
    // removed as a whole: they cannot lift their own *No access* with it.
    let mut world = World::new();
    let leads = world.group();
    let role = lead(&mut world);
    world.give_group(leads, role, world.acme);
    assert!(world.at(world.acme).allows(Permission::DecideRoles));
    world.no_access(world.acme);
    let acme = world.at(world.acme);
    assert!(acme.permissions().is_empty());
    assert!(!acme.decide_roles_was_kept());
    assert!(!acme.knows_project());
    assert!(matches!(
        acme.explain(Permission::DecideRoles).reasons.as_slice(),
        [Reason::RemovedByNoAccess { .. }]
    ));

    // A role for the whole organization is above every project: it is kept.
    let mut world = World::new();
    let role = lead(&mut world);
    let row = world.give_everywhere(role);
    let no_access = world.no_access(world.acme);
    let acme = world.at(world.acme);
    assert!(acme.allows(Permission::DecideRoles));
    assert!(acme.decide_roles_was_kept());
    assert!(matches!(
        acme.explain(Permission::DecideRoles).reasons.as_slice(),
        [Reason::KeptFromRemovedRow { row: kept, no_access: by, .. }]
            if kept.assignment == row && by.assignment == no_access
    ));

    // A *No access* lower down does not bring back what a *No access* at the depth of the
    // role took away: another *No access* never adds a permission.
    let mut world = World::new();
    let leads = world.group();
    let role = lead(&mut world);
    world.give_group(leads, role, world.acme);
    world.no_access(world.acme);
    world.no_access(world.web);
    assert!(!world.at(world.web).allows(Permission::DecideRoles));
    assert!(!world.at(world.acme).allows(Permission::DecideRoles));
}

#[test]
fn sec_03_the_depth_of_a_no_access_counts_whoever_it_is_for() {
    let lead = |world: &mut World| {
        world.role(
            "Project lead",
            &[Permission::SeeProjects, Permission::DecideRoles],
        )
    };

    // A role of one group and the *No access* of another group, both at ACME: they are of the
    // same rank, the role is removed as a whole, and a *No access* in WEB brings nothing back.
    let mut world = World::new();
    let (leads, blocked) = (world.group(), world.group());
    let role = lead(&mut world);
    world.give_group(leads, role, world.acme);
    world.no_access_group(blocked, world.acme);
    for place in [world.acme, world.web] {
        let held = world.at(place);
        assert!(!held.allows(Permission::DecideRoles), "{place:?}");
        assert!(!held.decide_roles_was_kept());
    }
    world.no_access(world.web);
    assert!(!world.at(world.web).allows(Permission::DecideRoles));

    // The person's own role in ACME stands above a group's *No access* there: in ACME it counts.
    // With their own *No access* in WEB it is removed there, and Decide who has which role is
    // kept from it, because the only *No access* at its depth ranks below it.
    let mut world = World::new();
    let blocked = world.group();
    let role = lead(&mut world);
    let row = world.give(role, world.acme);
    world.no_access_group(blocked, world.acme);
    let acme = world.at(world.acme);
    assert!(acme.allows(Permission::DecideRoles));
    assert!(!acme.decide_roles_was_kept(), "it counts, it is not kept");
    let no_access = world.no_access(world.web);
    let web = world.at(world.web);
    assert!(web.allows(Permission::DecideRoles));
    assert!(web.decide_roles_was_kept());
    assert!(matches!(
        web.explain(Permission::DecideRoles).reasons.as_slice(),
        [Reason::KeptFromRemovedRow { row: kept, no_access: by, .. }]
            if kept.assignment == row && by.assignment == no_access
    ));
}
