//! What a move of a project changes (PRJ-03, M2-F23) on plain data: the preview is the
//! difference of two evaluations, and a move lifts no *No access*.

use std::collections::BTreeSet;

use bb_core::access::{AppliesTo, Assignment, FieldLevel, GroupMember, PeopleLevel};
use bb_core::fields::Field;
use bb_core::id::{AssignmentId, GroupId, ProjectId, RoleId, SequentialIds, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_policy::{
    FieldRule, Lift, MoveRefusal, MoveRequest, PeopleRule, Permission, Place, Policy, RoleData,
    Tree, check_move, preview_move,
};
use jiff::Timestamp;
use proptest::prelude::*;

use bb_policy::PermissionBits;

/// A world of people, roles and projects, made with sequential IDs.
struct World {
    ids: SequentialIds,
    policy: Policy,
    links: Vec<(ProjectId, Option<ProjectId>)>,
}

impl World {
    fn new() -> Self {
        Self {
            ids: SequentialIds::default(),
            policy: Policy::default(),
            links: Vec::new(),
        }
    }

    fn person(&mut self) -> UserId {
        UserId::generate(&mut self.ids)
    }

    fn project(&mut self, parent: Option<ProjectId>) -> ProjectId {
        let id = ProjectId::generate(&mut self.ids).unwrap();
        self.links.push((id, parent));
        id
    }

    fn role(&mut self, name: &str, permissions: &[Permission]) -> RoleId {
        let id = RoleId::generate(&mut self.ids).unwrap();
        self.policy.roles.push(RoleData {
            id,
            name: name.to_owned(),
            position: i64::try_from(self.policy.roles.len()).unwrap(),
            permissions: permissions.iter().copied().collect(),
            two_step: false,
        });
        id
    }

    fn group(&mut self, people: &[UserId]) -> GroupId {
        let group = GroupId::generate(&mut self.ids).unwrap();
        for person in people {
            self.policy.members.push(GroupMember {
                group,
                person: *person,
                added_at: Timestamp::UNIX_EPOCH,
            });
        }
        group
    }

    fn row(
        &mut self,
        who: Result<UserId, GroupId>,
        role: Option<RoleId>,
        project: Option<ProjectId>,
        applies_to: AppliesTo,
    ) {
        self.policy.assignments.push(Assignment {
            id: AssignmentId::generate(&mut self.ids).unwrap(),
            person: who.ok(),
            group: who.err(),
            role,
            project,
            applies_to,
            given_at: Timestamp::UNIX_EPOCH,
        });
    }

    fn tree(&self) -> Tree {
        Tree::new(self.links.clone())
    }
}

const LEAD: [Permission; 4] = [
    Permission::SeeProjects,
    Permission::MoveProjects,
    Permission::DecideRoles,
    Permission::MakeSubProjects,
];
const VIEW: [Permission; 1] = [Permission::SeeProjects];

#[test]
fn prj_03_the_preview_names_a_person_who_gains_a_role_and_one_who_loses_access() {
    let mut world = World::new();
    let (bob, cleo) = (world.person(), world.person());
    let acme = world.project(None);
    let other = world.project(None);
    let web = world.project(Some(other));
    let api = world.project(Some(web));
    let lead = world.role("Project lead", &LEAD);
    let viewer = world.role("Viewer", &VIEW);
    // Bob leads ACME with its sub-projects; Cleo views OTHER with its sub-projects.
    world.row(
        Ok(bob),
        Some(lead),
        Some(acme),
        AppliesTo::ProjectAndSubProjects,
    );
    world.row(
        Ok(cleo),
        Some(viewer),
        Some(other),
        AppliesTo::ProjectAndSubProjects,
    );

    // WEB (with API inside) moves from OTHER into ACME.
    let preview = preview_move(&world.policy, &world.tree(), web, Some(acme));
    let projects: BTreeSet<ProjectId> = preview
        .changes
        .iter()
        .map(|change| change.project)
        .collect();
    assert_eq!(
        projects,
        BTreeSet::from([web, api]),
        "WEB and what is inside it"
    );
    let of = |person: UserId, project: ProjectId| {
        preview
            .changes
            .iter()
            .find(|change| change.person == person && change.project == project)
            .unwrap_or_else(|| panic!("no change for {person} at {project}"))
    };
    let gains = of(bob, web);
    assert!(!gains.before.sees && gains.after.sees);
    assert_eq!(gains.after.roles, ["Project lead"]);
    assert!(gains.after.permissions.contains(&Permission::MoveProjects));
    let loses = of(cleo, api);
    assert!(loses.before.sees && !loses.after.sees);
    assert_eq!(loses.before.roles, ["Viewer"]);
    assert!(loses.after.roles.is_empty());
    assert!(preview.lifts.is_empty());
}

#[test]
fn prj_03_a_person_whose_role_does_not_change_is_not_in_the_preview() {
    let mut world = World::new();
    let (bob, cleo) = (world.person(), world.person());
    let acme = world.project(None);
    let web = world.project(Some(acme));
    let other = world.project(None);
    let lead = world.role("Project lead", &LEAD);
    // Bob has the role everywhere; Cleo only for ACME alone (not reaching WEB).
    world.row(Ok(bob), Some(lead), None, AppliesTo::Organization);
    world.row(Ok(cleo), Some(lead), Some(acme), AppliesTo::Project);
    let preview = preview_move(&world.policy, &world.tree(), web, Some(other));
    assert!(preview.changes.is_empty(), "{:?}", preview.changes);
}

#[test]
fn prj_03_a_move_out_from_under_a_no_access_is_named() {
    let mut world = World::new();
    let bob = world.person();
    let acme = world.project(None);
    let web = world.project(Some(acme));
    let viewer = world.role("Viewer", &VIEW);
    world.row(Ok(bob), Some(viewer), None, AppliesTo::Organization);
    world.row(Ok(bob), None, Some(acme), AppliesTo::ProjectAndSubProjects);
    let tree = world.tree();
    let before = world
        .policy
        .evaluate_at(bob, &tree, Place::Project(web))
        .unwrap();
    assert!(!before.sees_project());

    // WEB goes to the top level: the No access of ACME no longer covers it.
    let preview = preview_move(&world.policy, &tree, web, None);
    let change = &preview.changes[0];
    assert_eq!(change.project, web);
    assert!(change.before.held_back && !change.after.held_back);
    assert!(!change.before.sees && change.after.sees);
}

#[test]
fn prj_03_a_move_cannot_let_somebody_lift_a_no_access_they_are_subject_to() {
    let mut world = World::new();
    let bob = world.person();
    let acme = world.project(None);
    let x = world.project(None);
    let lead = world.role("Project lead", &LEAD);
    // Bob leads ACME with its sub-projects, and has a No access on X.
    world.row(
        Ok(bob),
        Some(lead),
        Some(acme),
        AppliesTo::ProjectAndSubProjects,
    );
    world.row(Ok(bob), None, Some(x), AppliesTo::ProjectAndSubProjects);
    let tree = world.tree();
    let decides_at = |tree: &Tree| {
        world
            .policy
            .evaluate_at(bob, tree, Place::Project(x))
            .unwrap()
            .allows(Permission::DecideRoles)
    };
    assert!(!decides_at(&tree), "he cannot decide at X now");

    let preview = preview_move(&world.policy, &tree, x, Some(acme));
    assert_eq!(
        preview.lifts,
        [Lift {
            person: bob,
            project: x
        }]
    );
    // After the move he could take the No access away: the rule exists for this.
    assert!(decides_at(&tree.moved(x, Some(acme))));

    // Moving X somewhere that gives him nothing lifts nothing.
    let elsewhere = world.project(None);
    let tree = world.tree();
    assert!(
        preview_move(&world.policy, &tree, x, Some(elsewhere))
            .lifts
            .is_empty()
    );
}

#[test]
fn prj_03_somebody_who_could_lift_before_the_move_is_no_new_lift() {
    let mut world = World::new();
    let bob = world.person();
    let acme = world.project(None);
    let web = world.project(Some(acme));
    let api = world.project(Some(web));
    let lead = world.role("Project lead", &LEAD);
    world.row(
        Ok(bob),
        Some(lead),
        Some(acme),
        AppliesTo::ProjectAndSubProjects,
    );
    world.row(Ok(bob), None, Some(api), AppliesTo::ProjectAndSubProjects);
    // API is under ACME already, so Bob can take his No access away now; moving API from WEB
    // to ACME itself changes nothing about that.
    let tree = world.tree();
    let preview = preview_move(&world.policy, &tree, api, Some(acme));
    assert!(preview.lifts.is_empty(), "{:?}", preview.lifts);
}

#[test]
fn prj_03_a_groups_no_access_next_to_a_role_of_the_person_is_judged_like_their_own() {
    let mut world = World::new();
    let bob = world.person();
    let acme = world.project(None);
    let x = world.project(None);
    let lead = world.role("Project lead", &LEAD);
    let group = world.group(&[bob]);
    world.row(
        Ok(bob),
        Some(lead),
        Some(acme),
        AppliesTo::ProjectAndSubProjects,
    );
    world.row(Err(group), None, Some(x), AppliesTo::ProjectAndSubProjects);
    let preview = preview_move(&world.policy, &world.tree(), x, Some(acme));
    assert_eq!(
        preview.lifts,
        [Lift {
            person: bob,
            project: x
        }]
    );
}

// ---- The preview is the difference of the evaluations ----

#[derive(Clone, Debug)]
struct RowSpec {
    person: usize,
    group: Option<usize>,
    role: Option<usize>,
    project: Option<usize>,
    here_only: bool,
}

#[derive(Clone, Debug)]
struct Spec {
    /// The parent of project `n` is a project before it, or the top level.
    parents: Vec<Option<usize>>,
    permissions: [PermissionBits; 3],
    members: Vec<(usize, usize)>,
    rows: Vec<RowSpec>,
    rules: Vec<(Option<usize>, usize, usize, usize)>,
    /// The levels of *Who sees people*: project, role, kind, level.
    people_rules: Vec<(Option<usize>, usize, usize, usize)>,
    moved: usize,
    to: Option<usize>,
}

fn spec() -> impl Strategy<Value = Spec> {
    (
        prop::collection::vec(prop::option::of(0..6_usize), 6),
        prop::array::uniform3(Permission::every_set()),
        prop::collection::vec((0..2_usize, 0..4_usize), 0..6),
        prop::collection::vec(
            (
                0..4_usize,
                prop::option::of(0..2_usize),
                prop::option::of(0..3_usize),
                prop::option::of(0..6_usize),
                any::<bool>(),
            )
                .prop_map(|(person, group, role, project, here_only)| RowSpec {
                    person,
                    group,
                    role,
                    // *No access* is only ever set on a project.
                    project: if role.is_none() {
                        project.or(Some(0))
                    } else {
                        project
                    },
                    here_only,
                }),
            0..14,
        ),
        prop::collection::vec(
            (
                prop::option::of(0..6_usize),
                0..3_usize,
                0..16_usize,
                0..3_usize,
            ),
            0..8,
        ),
        prop::collection::vec(
            (
                prop::option::of(0..6_usize),
                0..3_usize,
                0..PeopleKind::ALL.len(),
                0..PeopleLevel::ALL.len(),
            ),
            0..8,
        ),
        0..6_usize,
        prop::option::of(0..6_usize),
    )
        .prop_map(
            |(parents, permissions, members, rows, rules, people_rules, moved, to)| Spec {
                parents,
                permissions,
                members,
                rows,
                rules,
                people_rules,
                moved,
                to,
            },
        )
}

/// The world of a spec, and the project that moves and where it goes. `None` when the spec
/// asks for a move that is no move (below itself, or where it is).
fn build(spec: &Spec) -> Option<(World, Vec<UserId>, ProjectId, Option<ProjectId>)> {
    let mut world = World::new();
    let people: Vec<UserId> = (0..4).map(|_| world.person()).collect();
    let mut projects: Vec<ProjectId> = Vec::new();
    for (at, parent) in spec.parents.iter().enumerate() {
        // Only a project made before may be the parent, so there is never a loop.
        let parent = parent
            .filter(|parent| *parent < at)
            .map(|parent| projects[parent]);
        projects.push(world.project(parent));
    }
    let roles: Vec<RoleId> = spec
        .permissions
        .iter()
        .enumerate()
        .map(|(at, bits)| {
            let granted: Vec<Permission> = Permission::set_of(*bits).into_iter().collect();
            world.role(&format!("Role {at}"), &granted)
        })
        .collect();
    let groups: Vec<GroupId> = (0..2).map(|_| world.group(&[])).collect();
    for (group, person) in &spec.members {
        world.policy.members.push(GroupMember {
            group: groups[*group],
            person: people[*person],
            added_at: Timestamp::UNIX_EPOCH,
        });
    }
    for row in &spec.rows {
        let who = row
            .group
            .map_or(Ok(people[row.person]), |group| Err(groups[group]));
        let project = row.project.map(|project| projects[project]);
        let applies_to = match project {
            None => AppliesTo::Organization,
            Some(_) if row.role.is_none() || !row.here_only => AppliesTo::ProjectAndSubProjects,
            Some(_) => AppliesTo::Project,
        };
        world.row(who, row.role.map(|role| roles[role]), project, applies_to);
    }
    for (project, role, field, level) in &spec.rules {
        world.policy.rules.add(FieldRule {
            project: project.map(|project| projects[project]),
            role: roles[*role],
            subject: Field::ALL[*field].into(),
            level: FieldLevel::ALL[*level],
        });
    }
    for (project, role, kind, level) in &spec.people_rules {
        world.policy.rules.add(PeopleRule {
            project: project.map(|project| projects[project]),
            role: roles[*role],
            subject: PeopleKind::ALL[*kind],
            level: PeopleLevel::ALL[*level],
        });
    }
    let moved = projects[spec.moved];
    let to = spec.to.map(|to| projects[to]);
    let tree = world.tree();
    if to.is_some_and(|to| tree.is_within(to, moved)) || tree.parent(moved) == Some(to) {
        return None;
    }
    Some((world, people, moved, to))
}

proptest! {
    #[test]
    fn prj_03_the_move_preview_names_who_gains_and_loses_access(spec in spec()) {
        let Some((world, people, moved, to)) = build(&spec) else {
            return Ok(());
        };
        let before = world.tree();
        let after = before.moved(moved, to);
        let preview = preview_move(&world.policy, &before, moved, to);

        // The difference of the evaluations of everybody at every project of the subtree, made
        // the long way round.
        let mut expected = Vec::new();
        let mut lifts = Vec::new();
        for person in &people {
            for project in before.subtree(moved) {
                let old = world.policy.evaluate_at(*person, &before, Place::Project(project));
                let new = world.policy.evaluate_at(*person, &after, Place::Project(project));
                let describe = |found: &Option<bb_policy::Evaluation>| {
                    found.as_ref().map(|found| {
                        (
                            found.sees_project(),
                            found.permissions().clone(),
                            found.roles().iter().map(|role| role.name.clone()).collect::<Vec<_>>(),
                            Field::ALL.iter().map(|field| found.field_level(*field)).collect::<Vec<_>>(),
                            PeopleKind::ALL.iter().map(|kind| found.kind_level(*kind)).collect::<Vec<_>>(),
                        )
                    })
                };
                if describe(&old) != describe(&new) {
                    expected.push((*person, project));
                }
                let has_no_access_here = world
                    .policy
                    .rows_of(*person)
                    .iter()
                    .any(|row| row.role.is_none() && row.project == Some(project));
                let decides = |found: &Option<bb_policy::Evaluation>| {
                    found.as_ref().is_some_and(|found| found.allows(Permission::DecideRoles))
                };
                if has_no_access_here && decides(&new) && !decides(&old) {
                    lifts.push(Lift { person: *person, project });
                }
            }
        }
        expected.sort();
        lifts.sort();
        let mut named: Vec<(UserId, ProjectId)> =
            preview.changes.iter().map(|change| (change.person, change.project)).collect();
        named.sort();
        prop_assert_eq!(named, expected);
        prop_assert_eq!(preview.lifts, lifts);
    }
}

/// The refusal (or `Ok`) for `person` moving `project` into `to`, in the world as it is.
fn may_move(
    world: &World,
    person: UserId,
    project: ProjectId,
    to: Option<ProjectId>,
) -> Result<(), MoveRefusal> {
    check_move(
        &world.policy,
        &world.tree(),
        &MoveRequest {
            person,
            key: None,
            project,
            to,
        },
    )
}

const MOVER: [Permission; 3] = [
    Permission::SeeProjects,
    Permission::MoveProjects,
    Permission::MakeProjects,
];

#[test]
fn prj_03_a_move_is_asked_for_by_one_set_of_rules_and_each_refusal_has_its_word() {
    let mut world = World::new();
    let (bob, cleo) = (world.person(), world.person());
    let (acme, other) = (world.project(None), world.project(None));
    let web = world.project(Some(acme));
    let api = world.project(Some(web));
    let mover = world.role("Mover", &MOVER);
    let viewer = world.role("Viewer", &VIEW);
    // Bob may see and move everywhere; Cleo only sees ACME and what is inside it.
    world.row(Ok(bob), Some(mover), None, AppliesTo::Organization);
    world.row(
        Ok(cleo),
        Some(viewer),
        Some(acme),
        AppliesTo::ProjectAndSubProjects,
    );

    assert_eq!(may_move(&world, bob, web, Some(other)), Ok(()));
    assert_eq!(may_move(&world, bob, web, None), Ok(()), "to the top level");
    assert_eq!(
        may_move(&world, bob, web, Some(api)),
        Err(MoveRefusal::IntoItself),
        "inside what is inside it"
    );
    assert_eq!(
        may_move(&world, bob, web, Some(web)),
        Err(MoveRefusal::IntoItself)
    );
    assert_eq!(
        may_move(&world, bob, web, Some(acme)),
        Err(MoveRefusal::SameParent)
    );
    assert_eq!(
        may_move(&world, bob, acme, None),
        Err(MoveRefusal::SameParent),
        "already at the top level"
    );
    // Cleo sees the project but may not move anything there; a project she does not see, and a
    // parent she does not see, are not there for her.
    assert_eq!(
        may_move(&world, cleo, web, Some(acme)),
        Err(MoveRefusal::Forbidden),
        "she sees both places but may move nothing there"
    );
    assert_eq!(
        may_move(&world, cleo, other, Some(acme)),
        Err(MoveRefusal::NotFound)
    );
    assert_eq!(
        may_move(&world, cleo, web, Some(other)),
        Err(MoveRefusal::NotFound),
        "a new parent she may not know about"
    );
    assert_eq!(
        may_move(
            &world,
            bob,
            web,
            Some(ProjectId::generate(&mut world.ids.clone()).unwrap())
        ),
        Err(MoveRefusal::NotFound),
        "a parent that does not exist"
    );
}

#[test]
fn prj_03_a_move_to_the_top_level_takes_make_projects_for_the_whole_organization() {
    let mut world = World::new();
    let (bob, cleo) = (world.person(), world.person());
    let (acme, other) = (world.project(None), world.project(None));
    let web = world.project(Some(acme));
    let mover = world.role("Mover", &MOVER);
    let moves_only = world.role(
        "Moves only",
        &[Permission::SeeProjects, Permission::MoveProjects],
    );
    world.row(Ok(bob), Some(mover), None, AppliesTo::Organization);
    // Cleo may move everywhere but is not allowed to make projects.
    world.row(Ok(cleo), Some(moves_only), None, AppliesTo::Organization);

    assert_eq!(may_move(&world, bob, web, None), Ok(()));
    assert_eq!(
        may_move(&world, cleo, web, None),
        Err(MoveRefusal::Forbidden),
        "a project that goes to the top level is made there"
    );
    assert_eq!(
        may_move(&world, cleo, web, Some(other)),
        Ok(()),
        "into another project it takes no Make projects"
    );
}

#[test]
fn prj_03_a_move_takes_move_projects_in_the_old_parent_as_well_as_the_new_one() {
    let mut world = World::new();
    let (bob, cleo) = (world.person(), world.person());
    let (acme, other) = (world.project(None), world.project(None));
    let web = world.project(Some(acme));
    let loose = world.project(None);
    let mover = world.role("Mover", &MOVER);
    let viewer = world.role("Viewer", &VIEW);
    // Bob sees everything and may move only in OTHER; Cleo may also move in ACME.
    for person in [bob, cleo] {
        world.row(Ok(person), Some(viewer), None, AppliesTo::Organization);
        world.row(
            Ok(person),
            Some(mover),
            Some(other),
            AppliesTo::ProjectAndSubProjects,
        );
    }
    world.row(
        Ok(cleo),
        Some(mover),
        Some(acme),
        AppliesTo::ProjectAndSubProjects,
    );

    assert_eq!(
        may_move(&world, bob, web, Some(other)),
        Err(MoveRefusal::Forbidden),
        "he may move into OTHER but not out of ACME"
    );
    assert_eq!(may_move(&world, cleo, web, Some(other)), Ok(()));
    assert_eq!(
        may_move(&world, bob, loose, Some(other)),
        Err(MoveRefusal::Forbidden),
        "a project at the top level is held by the whole organization"
    );
}

#[test]
fn prj_03_a_project_cannot_be_moved_deeper_than_a_tree_may_go() {
    let mut world = World::new();
    let bob = world.person();
    let mover = world.role("Mover", &MOVER);
    world.row(Ok(bob), Some(mover), None, AppliesTo::Organization);
    // A chain as deep as a tree may be, and a project that would make it one deeper.
    let mut top = world.project(None);
    for _ in 1..bb_core::project::DEPTH_MAX {
        top = world.project(Some(top));
    }
    let loose = world.project(None);
    assert_eq!(
        may_move(&world, bob, loose, Some(top)),
        Err(MoveRefusal::TooDeep)
    );
}
