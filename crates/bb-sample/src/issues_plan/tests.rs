use bb_dev_words::sample::Layout;

use super::*;

fn plan_of(wanted: u32, seed: u64) -> Plan {
    let names = Layout::bundled().unwrap().issues;
    let words = IssueWords::bundled().unwrap();
    // About eighteen months of working time.
    let calendar = Calendar::between(
        1_799_020_800,
        1_799_020_800 + 548 * bb_core::time::SECONDS_PER_DAY,
    );
    Plan::new(&names, &words, seed, wanted, &calendar).unwrap()
}

#[test]
fn a_share_is_whole_and_the_shares_make_the_total() {
    assert_eq!(allocate(10, &[1, 1, 1]), [4, 3, 3]);
    assert_eq!(allocate(100, &[45, 30, 20, 5]), [45, 30, 20, 5]);
    assert_eq!(allocate(7, &[0, 0]), [0, 0]);
    assert_eq!(
        allocate(1000, &[4800, 3000, 1800, 400]),
        [480, 300, 180, 40]
    );
    assert_eq!(allocate(0, &[1, 2]), [0, 0]);
}

#[test]
fn a_count_is_scaled_to_the_size_of_the_sample() {
    assert_eq!(scaled(30_000, 10_000, 1000), 3000);
    assert_eq!(scaled(3000, 10_000, 1000), 300);
    assert_eq!(scaled(3000, 10_000, 0), 0);
    assert_eq!(scaled(5, 0, 10), 0);
}

#[test]
fn the_plan_has_the_issues_per_project_and_per_type_of_the_layout() {
    let plan = plan_of(1000, 42);
    assert_eq!(plan.issues.len(), 1000);
    let names = Layout::bundled().unwrap().issues;
    let count = |family: usize| plan.issues.iter().filter(|i| i.family == family).count();
    assert_eq!(
        [count(0), count(1), count(2), count(3)],
        [480, 300, 180, 40]
    );
    let of_type = |name: &str| {
        let kind = kind_of(&names.types, name).unwrap();
        plan.issues
            .iter()
            .filter(|issue| issue.kind == kind)
            .count()
    };
    // 45, 30, 20 and 5 of a hundred, except that the Handbook has fewer bugs and more tasks.
    assert!((430..=440).contains(&of_type("Bug")), "{}", of_type("Bug"));
    assert!(
        (310..=318).contains(&of_type("Task")),
        "{}",
        of_type("Task")
    );
    assert!((198..=202).contains(&of_type("Feature")));
    assert!((49..=51).contains(&of_type("Goal")));
    // The sub-projects hold their shares.
    let in_place = |name: &str| plan.issues.iter().filter(|i| i.place == name).count();
    assert_eq!(
        [in_place("APP"), in_place("IOS"), in_place("AND")],
        [48, 240, 192]
    );
}

#[test]
fn the_same_seed_gives_the_same_plan_and_another_seed_another() {
    let titles = |seed| {
        plan_of(300, seed)
            .issues
            .into_iter()
            .map(|issue| issue.title)
            .collect::<Vec<_>>()
    };
    assert_eq!(titles(42), titles(42));
    assert_ne!(titles(42), titles(43));
}

#[test]
fn everything_comes_after_the_issue_it_is_about_and_the_steps_are_in_order_of_time() {
    let plan = plan_of(500, 42);
    assert!(plan.steps.windows(2).all(|pair| pair[0].at <= pair[1].at));
    let mut made = vec![None; plan.issues.len()];
    for step in &plan.steps {
        match step.action {
            Action::Make(issue) => made[issue] = Some(step.at),
            Action::Parent(issue)
            | Action::Tag(issue, _)
            | Action::Label(issue, _)
            | Action::Status(issue, _) => assert!(made[issue].is_some_and(|at| at <= step.at)),
            Action::Comment(comment) => {
                let issue = plan.comments[comment].issue;
                assert!(made[issue].is_some_and(|at| at <= step.at));
            }
            Action::Edit(comment) => {
                assert!(
                    plan.comments[comment]
                        .edit
                        .as_ref()
                        .is_some_and(|e| e.at >= plan.comments[comment].at)
                );
            }
            Action::Relate(relation) => {
                let (from, to) = (plan.relations[relation].from, plan.relations[relation].to);
                assert!(made[from].is_some() && made[to].is_some());
            }
        }
    }
}

#[test]
fn goals_have_sub_issues_in_their_own_project_and_nobody_else_does() {
    let plan = plan_of(1000, 42);
    let names = Layout::bundled().unwrap().issues;
    let goal = kind_of(&names.types, &starting_name(IssueIcon::Flag)).unwrap();
    let mut children = 0;
    for issue in &plan.issues {
        if let Some(parent) = issue.parent {
            children += 1;
            assert_eq!(plan.issues[parent].kind, goal);
            assert_eq!(plan.issues[parent].place, issue.place);
            assert_ne!(issue.kind, goal);
        }
    }
    assert!(children >= 50, "{children}");
}

#[test]
fn own_fields_are_only_given_where_the_field_is_used() {
    let plan = plan_of(1000, 42);
    let names = Layout::bundled().unwrap().issues;
    let severity = names
        .fields
        .iter()
        .position(|f| f.name == "Severity")
        .unwrap();
    let bug = kind_of(&names.types, &starting_name(IssueIcon::Bug)).unwrap();
    for issue in &plan.issues {
        assert_eq!(issue.fields[severity].is_some(), issue.kind == bug);
    }
    assert!(
        plan.issues
            .iter()
            .any(|issue| matches!(&issue.fields[2], Some(FieldPick::Text(_))))
    );
}

#[test]
fn a_tiny_sample_still_has_everything_in_it() {
    let plan = plan_of(30, 7);
    assert_eq!(plan.issues.len(), 30);
    assert!(!plan.comments.is_empty() && !plan.relations.is_empty());
}
