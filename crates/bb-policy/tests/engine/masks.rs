//! Field masks, `Authorized<T>` and the narrowing for access keys (SEC-04, SEC-06, AUTH-07).

use std::collections::BTreeSet;

use bb_core::access::FieldLevel;
use bb_core::fields::Field;
use bb_policy::{
    Choice, Kind, Masked, NOT_FOR_KEYS, Narrowing, Permission, Reason, drop_hidden, mask_member,
};
use serde::Serialize;
use serde_json::{Value, json};

use crate::common::{World, keys};

#[test]
fn sec_04_the_highest_field_level_of_the_roles_wins() {
    let mut world = World::new();
    let viewer = world.role("Viewer", &[Permission::SeeProjects]);
    let member = world.role("Team member", &[Permission::SeeProjects]);
    world.field(viewer, None, Field::Title, FieldLevel::CanSee);
    world.field(member, None, Field::Title, FieldLevel::CanChange);
    world.field(viewer, None, Field::CostCenter, FieldLevel::Hidden);
    world.field(member, None, Field::CostCenter, FieldLevel::CanSee);
    world.give(viewer, world.acme);
    let one_role = world.at(world.acme);
    assert_eq!(one_role.field_level(Field::Title), FieldLevel::CanSee);
    assert_eq!(one_role.field_level(Field::CostCenter), FieldLevel::Hidden);
    // A second role never lowers a level: the highest counts.
    world.give(member, world.acme);
    let two_roles = world.at(world.acme);
    assert_eq!(two_roles.field_level(Field::Title), FieldLevel::CanChange);
    assert_eq!(two_roles.field_level(Field::CostCenter), FieldLevel::CanSee);
    // A field nobody set is hidden: default deny.
    assert_eq!(two_roles.field_level(Field::Priority), FieldLevel::Hidden);
    let mask = two_roles.field_mask();
    assert!(mask.can_change(Field::Title));
    assert!(mask.can_see(Field::CostCenter) && !mask.can_change(Field::CostCenter));
    assert!(!mask.can_see(Field::Priority));
    let readable: Vec<Field> = mask.readable().collect();
    assert_eq!(readable, [Field::Title, Field::CostCenter]);
}

#[test]
fn sec_04_a_project_sets_its_own_level_and_sub_projects_inherit_until_one_sets_its_own() {
    let mut world = World::new();
    let member = world.role("Team member", &[Permission::SeeProjects]);
    world.give_everywhere(member);
    world.field(member, None, Field::CostCenter, FieldLevel::CanSee);
    world.field(
        member,
        Some(world.acme),
        Field::CostCenter,
        FieldLevel::Hidden,
    );
    world.field(
        member,
        Some(world.web),
        Field::CostCenter,
        FieldLevel::CanChange,
    );
    assert_eq!(
        world.at_organization().field_level(Field::CostCenter),
        FieldLevel::CanSee,
        "the role's own level at the organization"
    );
    assert_eq!(
        world.at(world.other).field_level(Field::CostCenter),
        FieldLevel::CanSee
    );
    assert_eq!(
        world.at(world.acme).field_level(Field::CostCenter),
        FieldLevel::Hidden
    );
    assert_eq!(
        world.at(world.web).field_level(Field::CostCenter),
        FieldLevel::CanChange,
        "the nearer project wins"
    );
    assert_eq!(
        world.at(world.api).field_level(Field::CostCenter),
        FieldLevel::CanChange,
        "inherited from WEB, the nearest parent that set one"
    );
    let decision = world.at(world.api).explain_level(Field::CostCenter);
    assert_eq!(decision.level, FieldLevel::CanChange);
    assert_eq!(decision.roles.len(), 1);
    assert_eq!(decision.roles[0].level, FieldLevel::CanChange);
}

/// An issue as an answer shows it: two members hold fields of the catalogue.
#[derive(Serialize)]
struct IssueView {
    id: u32,
    title: String,
    cost_center: String,
    children: Vec<IssueView>,
    parent: Option<Box<IssueView>>,
}

impl Masked for IssueView {
    const FIELDS: &'static [(&'static str, Field)] =
        &[("title", Field::Title), ("cost_center", Field::CostCenter)];

    fn mask(value: &mut Value, mask: &bb_policy::FieldMask) {
        drop_hidden(value, Self::FIELDS, mask);
        // A mask works inside views too (M2-F01): what a view holds is masked the same way.
        mask_member::<Vec<IssueView>>(value, "children", mask);
        mask_member::<Option<IssueView>>(value, "parent", mask);
    }
}

fn issue(id: u32, children: Vec<IssueView>, parent: Option<IssueView>) -> IssueView {
    IssueView {
        id,
        title: format!("Issue {id}"),
        cost_center: "K-4711".to_owned(),
        children,
        parent: parent.map(Box::new),
    }
}

fn masked_world(cost_center: FieldLevel) -> World {
    let mut world = World::new();
    let member = world.role("Team member", &[Permission::SeeProjects]);
    world.field(member, None, Field::Title, FieldLevel::CanChange);
    world.field(member, None, Field::CostCenter, cost_center);
    world.give(member, world.acme);
    world
}

#[test]
fn sec_06_a_hidden_field_is_not_in_the_answer_and_asking_for_it_fails_like_an_unknown_field() {
    let world = masked_world(FieldLevel::Hidden);
    let evaluation = world.at(world.web);
    let authorized = evaluation.authorize(issue(
        1,
        vec![issue(2, Vec::new(), None)],
        Some(issue(3, Vec::new(), None)),
    ));
    let shown = authorized.masked_value().unwrap();
    assert_eq!(shown["title"], "Issue 1");
    assert!(shown.get("cost_center").is_none(), "{shown}");
    assert!(
        shown["children"][0].get("cost_center").is_none(),
        "inside a list"
    );
    assert!(
        shown["parent"].get("cost_center").is_none(),
        "inside an object"
    );
    assert_eq!(shown["children"][0]["title"], "Issue 2");
    // The serialized form is the same, whatever way it leaves.
    assert_eq!(serde_json::to_value(&authorized).unwrap(), shown);
    // The names the `fields` helper must refuse: the same ones it refuses for fields that do not
    // exist, so the answer never tells that a field is there.
    assert_eq!(authorized.hidden_properties(), ["cost_center"]);

    let visible = masked_world(FieldLevel::CanSee);
    let shown = visible
        .at(visible.web)
        .authorize(issue(1, Vec::new(), None));
    assert_eq!(shown.masked_value().unwrap()["cost_center"], "K-4711");
    assert!(shown.hidden_properties().is_empty());
}

#[test]
fn sec_06_a_person_without_any_role_sees_none_of_the_fields() {
    let world = masked_world(FieldLevel::CanSee);
    let outside = world.at(world.other);
    let shown = outside
        .authorize(issue(1, Vec::new(), None))
        .masked_value()
        .unwrap();
    assert_eq!(shown, json!({"id": 1, "children": [], "parent": null}));
}

#[test]
fn authorized_maps_to_another_shape_and_keeps_the_mask() {
    #[derive(Serialize)]
    struct Page {
        items: Vec<IssueView>,
    }
    impl Masked for Page {
        fn mask(value: &mut Value, mask: &bb_policy::FieldMask) {
            mask_member::<Vec<IssueView>>(value, "items", mask);
        }
    }
    let world = masked_world(FieldLevel::Hidden);
    let evaluation = world.at(world.acme);
    let items = evaluation.authorize(vec![1_u32, 2]).map(|numbers| Page {
        items: numbers
            .into_iter()
            .map(|n| issue(n, Vec::new(), None))
            .collect(),
    });
    let shown = items.masked_value().unwrap();
    assert_eq!(shown["items"].as_array().unwrap().len(), 2);
    assert!(shown["items"][1].get("cost_center").is_none());
    assert_eq!(items.mask().level(Field::Title), FieldLevel::CanChange);
}

fn narrowed_world() -> World {
    let mut world = World::new();
    let lead = world.role(
        "Lead",
        &[
            Permission::SeeProjects,
            Permission::MakeSubProjects,
            Permission::DecideRoles,
            Permission::CheckAccess,
            Permission::ManageRoles,
        ],
    );
    world.field(lead, None, Field::Title, FieldLevel::CanChange);
    world.give_everywhere(lead);
    world
}

#[test]
fn auth_07_a_key_with_see_only_keeps_only_permissions_of_the_kind_see_and_no_field_above_can_see() {
    let world = narrowed_world();
    let owner = world.at(world.web);
    assert_eq!(owner.field_level(Field::Title), FieldLevel::CanChange);
    let key = owner.narrowed(&Narrowing::see_only());
    assert_eq!(keys(&key), ["see-projects", "check-access"]);
    assert_eq!(key.field_level(Field::Title), FieldLevel::CanSee);
    // See and change adds the permissions of the kind Change; Manage stays out.
    let key = owner.narrowed(&Narrowing::see_and_change());
    assert_eq!(
        keys(&key),
        ["see-projects", "make-sub-projects", "check-access"]
    );
    assert_eq!(key.field_level(Field::Title), FieldLevel::CanChange);
    // Everything I can do is the owner without what no key holds: inviting people and changing
    // who has which role stay in the app (decision on access keys).
    let key = owner.narrowed(&Narrowing::everything());
    let owners_without: BTreeSet<Permission> = owner
        .permissions()
        .iter()
        .copied()
        .filter(|permission| !NOT_FOR_KEYS.contains(permission))
        .collect();
    assert_eq!(key.permissions(), &owners_without);
    assert!(owner.permissions().contains(&Permission::DecideRoles));
    assert!(!key.permissions().contains(&Permission::DecideRoles));
    // The owner's own evaluation is untouched.
    assert_eq!(owner.permissions().len(), 4);
}

#[test]
fn auth_07_single_permissions_never_give_what_the_owner_does_not_hold() {
    let world = narrowed_world();
    let owner = world.at(world.web);
    let picked: BTreeSet<Permission> = [
        Permission::SeeProjects,
        Permission::MakeProjects,
        Permission::ManagePeople,
    ]
    .into_iter()
    .collect();
    let key = owner.narrowed(&Narrowing::new(Choice::Permissions(picked)));
    assert_eq!(keys(&key), ["see-projects"]);
    let decision = key.explain(Permission::CheckAccess);
    assert!(!decision.allowed);
    // The chain: the owner has it through a role, and the key does not keep it.
    assert!(matches!(decision.reasons[0], Reason::GivenByRole { .. }));
    assert_eq!(decision.reasons.last(), Some(&Reason::NarrowedByKey));
    // Picking only permissions of the kind See caps the fields too.
    let only_see = Narrowing::new(Choice::Permissions(BTreeSet::from([
        Permission::SeeProjects,
        Permission::CheckAccess,
    ])));
    assert_eq!(
        owner.narrowed(&only_see).field_level(Field::Title),
        FieldLevel::CanSee
    );
}

#[test]
fn auth_07_a_key_for_projects_counts_in_them_and_their_sub_projects_and_nowhere_else() {
    let mut world = narrowed_world();
    let member = world.role("Team member", &[Permission::SeeProjects]);
    world.give(member, world.other);
    let key = Narrowing::everything().in_projects(BTreeSet::from([world.web]));
    // WEB itself and API below it.
    assert!(world.at(world.web).narrowed(&key).sees_project());
    assert!(world.at(world.api).narrowed(&key).sees_project());
    // The parent of its project, a sibling tree, and the whole organization: nothing.
    let parent = world.at(world.acme).narrowed(&key);
    assert!(parent.permissions().is_empty());
    assert_eq!(
        parent.explain(Permission::SeeProjects).reasons.last(),
        Some(&Reason::OutsideKeyProjects)
    );
    assert!(
        world
            .at(world.other)
            .narrowed(&key)
            .permissions()
            .is_empty()
    );
    assert!(
        world
            .at_organization()
            .narrowed(&key)
            .permissions()
            .is_empty()
    );
    assert_eq!(
        world.at(world.api).narrowed(&key).field_level(Field::Title),
        FieldLevel::CanChange
    );
    // Nothing is shown outside the key's projects, not even a field the owner can read.
    assert_eq!(
        world
            .at(world.acme)
            .narrowed(&key)
            .field_level(Field::Title),
        FieldLevel::Hidden
    );
}

#[test]
fn auth_07_a_key_follows_its_owner_the_evaluation_is_made_again_for_every_request() {
    let mut world = narrowed_world();
    let key = Narrowing::see_only();
    assert!(world.at(world.web).narrowed(&key).sees_project());
    // The owner loses the role: the key has nothing left, whatever it was given.
    world.rows.clear();
    assert!(world.at(world.web).narrowed(&key).permissions().is_empty());
}

#[test]
fn the_kinds_of_a_choice_are_all_three() {
    assert_eq!(Kind::ALL.len(), 3);
}

/// A person as a view inside another view: no field of its own.
#[derive(Serialize, bb_policy::Masked)]
struct PersonInside {
    #[masked(open)]
    name: String,
}

/// An issue inside an issue (a parent, a sub-issue), made with the derive: every member says how
/// it is masked, and the issues inside it are masked by their own fields (M2-F21).
#[derive(Serialize, bb_policy::Masked)]
struct NestedIssue {
    #[masked(open)]
    id: u32,
    #[masked(field = "title")]
    title: String,
    #[masked(field = "cost-center")]
    cost_center: String,
    #[masked(field = "assigned-to", nested)]
    assigned_to: Vec<PersonInside>,
    #[masked(nested)]
    parent: Option<Box<NestedIssue>>,
    #[masked(nested)]
    sub_issues: Vec<NestedIssue>,
}

fn nested_issue(id: u32, parent: Option<NestedIssue>, sub_issues: Vec<NestedIssue>) -> NestedIssue {
    NestedIssue {
        id,
        title: format!("Issue {id}"),
        cost_center: "K-4711".to_owned(),
        assigned_to: vec![PersonInside {
            name: "Ada".to_owned(),
        }],
        parent: parent.map(Box::new),
        sub_issues,
    }
}

#[test]
fn sec_04_a_nested_view_never_carries_a_hidden_field() {
    let hidden = masked_world(FieldLevel::Hidden);
    let issue = nested_issue(
        1,
        Some(nested_issue(2, None, Vec::new())),
        vec![nested_issue(
            3,
            None,
            vec![nested_issue(4, None, Vec::new())],
        )],
    );
    let shown = hidden
        .at(hidden.web)
        .authorize(issue)
        .masked_value()
        .unwrap();
    let text = shown.to_string();
    assert!(
        !text.contains("cost_center") && !text.contains("K-4711"),
        "{text}"
    );
    // The members that may be seen stay, at every depth.
    assert_eq!(shown["title"], "Issue 1");
    assert_eq!(shown["parent"]["title"], "Issue 2");
    assert_eq!(shown["sub_issues"][0]["sub_issues"][0]["title"], "Issue 4");
    // A field the viewer may not see is dropped, and so is the list of people inside it.
    assert!(shown.get("assigned_to").is_none(), "{text}");
    assert!(shown["parent"].get("assigned_to").is_none(), "{text}");
}

#[test]
fn sec_04_the_derive_lists_the_fields_a_view_holds_and_nothing_else() {
    assert_eq!(
        <NestedIssue as Masked>::FIELDS,
        [
            ("title", Field::Title),
            ("cost_center", Field::CostCenter),
            ("assigned_to", Field::AssignedTo)
        ]
    );
    assert!(<PersonInside as Masked>::FIELDS.is_empty());
    let world = masked_world(FieldLevel::Hidden);
    let evaluation = world.at(world.web);
    let shown = evaluation.authorize(nested_issue(1, None, Vec::new()));
    assert_eq!(shown.hidden_properties(), ["cost_center", "assigned_to"]);
}
