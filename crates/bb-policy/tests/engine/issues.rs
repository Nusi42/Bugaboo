//! Which issues a person sees and may change (ISS-01, SEC-04, M3-Q6).

use bb_core::id::UserId;
use bb_policy::StartingRole;

use crate::common::World;

fn somebody_else() -> UserId {
    UserId::from_random([9; 16])
}

/// A world where the person holds a starting role on the project WEB.
fn with(role: StartingRole) -> World {
    let mut world = World::new();
    let id = world.role(role.code(), &role.permissions());
    world.give(id, world.web);
    world
}

#[test]
fn iss_01_a_guest_sees_only_the_issues_they_reported_and_everybody_else_sees_them_all() {
    let other = somebody_else();
    for role in StartingRole::ALL {
        let world = with(*role);
        let held = world.at(world.web);
        let own = world.person;
        assert!(held.sees_issue(own), "{role:?} sees what they reported");
        assert_eq!(
            held.sees_issue(other),
            *role != StartingRole::Guest,
            "{role:?} sees what somebody else reported"
        );
    }
}

#[test]
fn iss_01_nobody_sees_an_issue_in_a_project_they_do_not_see() {
    let world = with(StartingRole::Administrator);
    let outside = world.at(world.other);
    assert!(!outside.sees_issue(world.person));
    assert!(!outside.may_change_issue(world.person));
    assert!(!outside.may_delete_issue(world.person));
}

#[test]
fn iss_01_changing_takes_change_issues_or_change_own_issues_on_what_one_reported() {
    let other = somebody_else();
    let may = |role: StartingRole| {
        let world = with(role);
        let held = world.at(world.web);
        (
            held.may_change_issue(world.person),
            held.may_change_issue(other),
        )
    };
    assert_eq!(may(StartingRole::TeamMember), (true, true));
    assert_eq!(may(StartingRole::ProjectLead), (true, true));
    assert_eq!(may(StartingRole::Contributor), (true, false));
    assert_eq!(may(StartingRole::Guest), (true, false));
    assert_eq!(may(StartingRole::Viewer), (false, false));
}

#[test]
fn iss_01_deleting_takes_delete_issues_on_an_issue_one_sees() {
    let other = somebody_else();
    let may = |role: StartingRole| {
        let world = with(role);
        let held = world.at(world.web);
        (
            held.may_delete_issue(world.person),
            held.may_delete_issue(other),
        )
    };
    assert_eq!(may(StartingRole::TeamMember), (true, true));
    assert_eq!(may(StartingRole::Contributor), (false, false));
    assert_eq!(may(StartingRole::Guest), (false, false));
}
