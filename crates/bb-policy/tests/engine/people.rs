//! `PersonRef`, *Who sees people* and the labels of removed people (PRIV-12, PRIV-13).

use std::collections::BTreeSet;

use bb_core::access::PeopleLevel;
use bb_core::id::{IdSource, SequentialIds, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_policy::{
    FormerUser, PeopleContext, Permission, PersonRef, Place, Target, former_user_label, resolve,
    taken_in,
};

use crate::common::World;

fn target(id: UserId, role: Option<&'static str>) -> Target<'static> {
    Target {
        id,
        name: "Bea Builder",
        role,
        removed: None,
    }
}

/// The viewer holds the role "Guest" on ACME and the role sees `kind` as `level` there.
fn viewer_world(kind: PeopleKind, level: PeopleLevel) -> World {
    let mut world = World::new();
    let guest = world.role("Guest", &[Permission::SeeProjects]);
    world.give(guest, world.acme);
    world.people(guest, Some(world.acme), kind, level);
    world
}

#[test]
fn priv_12_people_are_shown_by_name_role_or_someone_as_the_rules_say() {
    let bea = UserId::from_random([2; 16]);
    for kind in PeopleKind::ALL.iter().copied() {
        for level in PeopleLevel::ALL.iter().copied() {
            let world = viewer_world(kind, level);
            // Set on ACME, inherited down the tree: WEB and API (two below) show the same.
            for project in [world.acme, world.web, world.api] {
                let viewer = world.at(project);
                let shown = resolve(
                    &viewer,
                    kind,
                    PeopleContext::Ordinary,
                    target(bea, Some("a Team member")),
                );
                let expected = match level {
                    PeopleLevel::Names => PersonRef::Name {
                        id: bea,
                        name: "Bea Builder".to_owned(),
                    },
                    PeopleLevel::RoleOnly => PersonRef::Role {
                        role: "a Team member".to_owned(),
                    },
                    PeopleLevel::Hidden => PersonRef::Someone,
                };
                assert_eq!(shown, expected, "{kind:?} as {level:?}");
                // A person who has no role there is Someone, never an empty label.
                let without_role =
                    resolve(&viewer, kind, PeopleContext::Ordinary, target(bea, None));
                if level == PeopleLevel::RoleOnly {
                    assert_eq!(without_role, PersonRef::Someone);
                }
            }
            // A place outside the tree has no role of the viewer: nothing is shown.
            let outside = world.at(world.other);
            assert_eq!(
                resolve(
                    &outside,
                    kind,
                    PeopleContext::Ordinary,
                    target(bea, Some("x"))
                ),
                PersonRef::Someone
            );
        }
    }
}

#[test]
fn priv_12_a_nearer_project_sets_its_own_level_for_a_kind() {
    let mut world = viewer_world(PeopleKind::Followers, PeopleLevel::Names);
    let guest = world.roles[0].id;
    world.people(
        guest,
        Some(world.web),
        PeopleKind::Followers,
        PeopleLevel::Hidden,
    );
    let bea = UserId::from_random([2; 16]);
    let at = |project| {
        resolve(
            &world.at(project),
            PeopleKind::Followers,
            PeopleContext::Ordinary,
            target(bea, None),
        )
    };
    assert!(matches!(at(world.acme), PersonRef::Name { .. }));
    assert_eq!(at(world.web), PersonRef::Someone);
    assert_eq!(at(world.api), PersonRef::Someone, "inherited from WEB");
}

#[test]
fn priv_12_the_most_open_level_of_the_roles_counts() {
    let mut world = viewer_world(PeopleKind::Mentions, PeopleLevel::Hidden);
    let better = world.role("Team member", &[Permission::SeeProjects]);
    world.people(better, None, PeopleKind::Mentions, PeopleLevel::RoleOnly);
    world.give(better, world.acme);
    let bea = UserId::from_random([2; 16]);
    let shown = resolve(
        &world.at(world.web),
        PeopleKind::Mentions,
        PeopleContext::Ordinary,
        target(bea, Some("a Viewer")),
    );
    assert_eq!(
        shown,
        PersonRef::Role {
            role: "a Viewer".to_owned()
        }
    );
}

#[test]
fn priv_12_the_viewer_always_sees_their_own_name() {
    let world = viewer_world(PeopleKind::AssignedTo, PeopleLevel::Hidden);
    let me = target(world.person, None);
    let viewer = world.at(world.web);
    for kind in PeopleKind::ALL.iter().copied() {
        assert!(matches!(
            resolve(&viewer, kind, PeopleContext::Ordinary, me),
            PersonRef::Name { id, .. } if id == world.person
        ));
    }
    // Even a person with no role anywhere.
    let nobody = bb_policy::evaluate(&World::new().inputs(Place::Organization));
    assert!(matches!(
        resolve(&nobody, PeopleKind::AssignedTo, PeopleContext::Ordinary, me),
        PersonRef::Name { .. }
    ));
}

#[test]
fn priv_12_a_role_label_and_someone_carry_no_id_of_the_person() {
    let bea = UserId::from_random([2; 16]);
    let text = bea.to_string();
    let world = viewer_world(PeopleKind::AssignedTo, PeopleLevel::RoleOnly);
    let viewer = world.at(world.web);
    for shown in [
        resolve(
            &viewer,
            PeopleKind::AssignedTo,
            PeopleContext::Ordinary,
            target(bea, Some("a Viewer")),
        ),
        resolve(
            &viewer,
            PeopleKind::AssignedTo,
            PeopleContext::Ordinary,
            target(bea, None),
        ),
        resolve(
            &viewer,
            PeopleKind::Followers,
            PeopleContext::Ordinary,
            target(bea, Some("a Viewer")),
        ),
    ] {
        let json = serde_json::to_string(&shown).unwrap();
        assert!(!json.contains(&text), "{json}");
        assert!(!json.contains("Bea"), "{json}");
        assert!(!json.contains('@'), "{json}");
    }
    let name = resolve(
        &world.at(world.web),
        PeopleKind::Pictures,
        PeopleContext::Ordinary,
        target(bea, None),
    );
    assert_eq!(name, PersonRef::Someone);
}

#[test]
fn priv_12_the_person_ref_is_the_tagged_union_of_architecture_7() {
    let id = UserId::from_random([2; 16]);
    let json = |shown: &PersonRef| serde_json::to_value(shown).unwrap();
    assert_eq!(
        json(&PersonRef::Name {
            id,
            name: "Bea".to_owned()
        }),
        serde_json::json!({"shown": "name", "id": id.to_string(), "name": "Bea"})
    );
    assert_eq!(
        json(&PersonRef::Role {
            role: "a Viewer".to_owned()
        }),
        serde_json::json!({"shown": "role", "role": "a Viewer"})
    );
    assert_eq!(
        json(&PersonRef::Someone),
        serde_json::json!({"shown": "someone"})
    );
    assert_eq!(
        json(&PersonRef::Former {
            code: "K7Q2".to_owned()
        }),
        serde_json::json!({"shown": "former", "code": "K7Q2"})
    );
}

#[test]
fn priv_13_a_removed_person_is_a_former_user_with_names_and_someone_otherwise() {
    let gone = FormerUser {
        label: "K7Q2".to_owned(),
    };
    let removed = |id| Target {
        id,
        name: "ignored",
        role: Some("a Viewer"),
        removed: Some(&gone),
    };
    let id = UserId::from_random([2; 16]);
    for (level, expected) in [
        (
            PeopleLevel::Names,
            PersonRef::Former {
                code: "K7Q2".to_owned(),
            },
        ),
        (PeopleLevel::RoleOnly, PersonRef::Someone),
        (PeopleLevel::Hidden, PersonRef::Someone),
    ] {
        let world = viewer_world(PeopleKind::Authors, level);
        let viewer = world.at(world.acme);
        assert_eq!(
            resolve(
                &viewer,
                PeopleKind::Authors,
                PeopleContext::Ordinary,
                removed(id)
            ),
            expected
        );
    }
}

#[test]
fn priv_12_people_who_manage_see_names_in_management_screens_and_others_see_project_members() {
    let bea = UserId::from_random([2; 16]);
    // A viewer who decides who has which role sees names, whatever their levels say.
    let mut world = viewer_world(PeopleKind::ProjectMembers, PeopleLevel::Hidden);
    let lead = world.role(
        "Project lead",
        &[Permission::SeeProjects, Permission::DecideRoles],
    );
    world.give(lead, world.acme);
    let manager = world.at(world.web);
    assert!(matches!(
        resolve(
            &manager,
            PeopleKind::ProjectMembers,
            PeopleContext::Management,
            target(bea, None)
        ),
        PersonRef::Name { .. }
    ));
    // Everybody else sees the members by the kind Project members, at the level of their role.
    for (level, expected) in [
        (PeopleLevel::Names, true),
        (PeopleLevel::RoleOnly, false),
        (PeopleLevel::Hidden, false),
    ] {
        let world = viewer_world(PeopleKind::ProjectMembers, level);
        let shown = resolve(
            &world.at(world.web),
            PeopleKind::ProjectMembers,
            PeopleContext::Management,
            target(bea, Some("a Viewer")),
        );
        assert_eq!(
            matches!(shown, PersonRef::Name { .. }),
            expected,
            "{level:?}"
        );
    }
    // The people-management permissions of the organization count at the organization.
    let mut world = World::new();
    let admin = world.role("Administrator", &[Permission::ManagePeople]);
    world.give_everywhere(admin);
    assert!(matches!(
        resolve(
            &world.at_organization(),
            PeopleKind::ProjectMembers,
            PeopleContext::Management,
            target(bea, None)
        ),
        PersonRef::Name { .. }
    ));
}

#[test]
fn priv_12_a_picture_is_shown_only_where_the_kind_also_gives_names() {
    let mut world = viewer_world(PeopleKind::AssignedTo, PeopleLevel::Names);
    let guest = world.roles[0].id;
    world.people(guest, None, PeopleKind::Pictures, PeopleLevel::Names);
    let viewer = world.at(world.web);
    assert!(viewer.shows_picture(PeopleKind::AssignedTo, PeopleContext::Ordinary));
    assert!(!viewer.shows_picture(PeopleKind::Followers, PeopleContext::Ordinary));
}

/// A deterministic source that is not a counter: xorshift.
struct Scrambled(u64);

impl IdSource for Scrambled {
    fn now(&mut self) -> jiff::Timestamp {
        jiff::Timestamp::UNIX_EPOCH
    }

    fn random(&mut self) -> [u8; 16] {
        let mut bytes = [0_u8; 16];
        for chunk in bytes.chunks_mut(8) {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            chunk.copy_from_slice(&self.0.to_le_bytes());
        }
        bytes
    }
}

#[test]
fn priv_13_former_user_labels_are_random_and_unique() {
    let mut source = Scrambled(0x9E37_79B9_7F4A_7C15);
    let mut taken = BTreeSet::new();
    let mut labels = Vec::new();
    for _ in 0..3000 {
        let label = former_user_label(&mut source, &taken_in(&taken)).unwrap();
        assert!(label.len() >= 4);
        assert!(
            label
                .chars()
                .all(|c| c.is_ascii() && bb_core::text::CROCKFORD.contains(&(c as u8))),
            "Crockford Base32: {label}"
        );
        assert!(taken.insert(label.clone()), "{label} was made twice");
        labels.push(label);
    }
    // They do not count: about half of the neighbours go up, and none is one more than the one
    // before it.
    let up = labels.windows(2).filter(|pair| pair[1] > pair[0]).count();
    assert!((1200..1800).contains(&up), "{up} of 2999 go up");
    // Four characters are used while there is room.
    assert!(labels.iter().filter(|label| label.len() == 4).count() > 2900);
}

#[test]
fn priv_13_a_label_is_drawn_again_on_a_clash_and_gets_longer_when_four_run_short() {
    let mut source = Scrambled(42);
    let first = former_user_label(&mut source, &|_| false).unwrap();
    // A clash with the first draw: the second draw is used.
    let mut again = Scrambled(42);
    let clash = first.clone();
    let second = former_user_label(&mut again, &move |label| label == clash).unwrap();
    assert_ne!(first, second);
    assert_eq!(second.len(), 4);
    // When no label of four characters is free, five are made, then six.
    let mut source = Scrambled(7);
    let five = former_user_label(&mut source, &|label| label.len() < 5).unwrap();
    assert_eq!(five.len(), 5);
    let six = former_user_label(&mut source, &|label| label.len() < 6).unwrap();
    assert_eq!(six.len(), 6);
    // Nothing is free: no label, and no endless loop.
    assert_eq!(former_user_label(&mut source, &|_| true), None);
}

#[test]
fn priv_13_a_label_is_never_made_from_the_id_of_the_person() {
    // The source is the only input of a label: the person's ID is not an argument at all, and the
    // same source state gives the same label for every person.
    let mut one = Scrambled(5);
    let mut two = Scrambled(5);
    assert_eq!(
        former_user_label(&mut one, &|_| false),
        former_user_label(&mut two, &|_| false)
    );
    let mut sequential = SequentialIds::default();
    let counted = former_user_label(&mut sequential, &|_| false).unwrap();
    assert_eq!(counted.len(), 4);
}

#[test]
fn priv_12_a_role_label_is_shown_exactly_when_the_viewer_sees_roles_and_it_is_not_themselves() {
    let bea = UserId::from_random([2; 16]);
    for kind in PeopleKind::ALL.iter().copied() {
        for level in PeopleLevel::ALL.iter().copied() {
            let world = viewer_world(kind, level);
            let viewer = world.at(world.acme);
            let labelled = viewer.shows_role_label(kind, PeopleContext::Ordinary, bea);
            assert_eq!(
                labelled,
                level == PeopleLevel::RoleOnly,
                "{kind:?} as {level:?}"
            );
            // `resolve` shows a role label exactly when the predicate says so.
            let shown = resolve(
                &viewer,
                kind,
                PeopleContext::Ordinary,
                target(bea, Some("a Team member")),
            );
            assert_eq!(matches!(shown, PersonRef::Role { .. }), labelled);
        }
    }
    // Nobody is shown their own role: they see their name.
    let world = viewer_world(PeopleKind::Authors, PeopleLevel::RoleOnly);
    let viewer = world.at(world.acme);
    if let Some(me) = viewer.person() {
        assert!(!viewer.shows_role_label(PeopleKind::Authors, PeopleContext::Ordinary, me));
    }
}
