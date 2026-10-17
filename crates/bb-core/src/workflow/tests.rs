use super::starter::{BUG, DEVELOPMENT, DONE, IN_PROGRESS, OPEN, SIMPLE, STARTERS, Starter};
use super::*;
use crate::fields::Field;
use crate::id::SequentialIds;

fn key(text: &str) -> StatusKey {
    StatusKey::new(text).unwrap()
}

struct Ids {
    parent: ProjectId,
    child: ProjectId,
    bug: IssueTypeId,
    task: IssueTypeId,
    simple: WorkflowId,
    own: WorkflowId,
}

fn ids() -> Ids {
    let mut source = SequentialIds::default();
    Ids {
        parent: ProjectId::generate(&mut source).unwrap(),
        child: ProjectId::generate(&mut source).unwrap(),
        bug: IssueTypeId::generate(&mut source).unwrap(),
        task: IssueTypeId::generate(&mut source).unwrap(),
        simple: WorkflowId::generate(&mut source).unwrap(),
        own: WorkflowId::generate(&mut source).unwrap(),
    }
}

#[test]
fn wf_01_the_starting_workflow_keeps_the_keys_the_issues_of_m3_have() {
    let simple = SIMPLE.content();
    let keys: Vec<&str> = simple
        .statuses
        .iter()
        .map(|each| each.key.as_str())
        .collect();
    assert_eq!(keys, ["open", "in-progress", "done"]);
    let groups: Vec<StatusGroup> = simple.statuses.iter().map(|each| each.group).collect();
    assert_eq!(groups, StatusGroup::ALL);
    assert_eq!(simple.initial(), Some(&key(OPEN)));
    // Any status can follow any other, as it did in M3.
    for from in &simple.statuses {
        for to in &simple.statuses {
            assert_eq!(
                simple.change(&from.key, &to.key).is_some(),
                from.key != to.key,
                "{} to {}",
                from.key,
                to.key
            );
        }
    }
}

#[test]
fn wf_06_the_three_starter_workflows_are_workflows_a_save_accepts() {
    let names: Vec<String> = STARTERS.iter().map(Starter::title).collect();
    assert_eq!(names, ["Simple", "Bug", "Development"]);
    for starter in STARTERS {
        assert_eq!(starter.content().check(), Ok(()), "{}", starter.title());
    }
    let statuses_of = |starter: &Starter| -> Vec<(String, StatusGroup)> {
        starter
            .content()
            .statuses
            .into_iter()
            .map(|status| (status.name, status.group))
            .collect()
    };
    let entry = |name: &str, group| (name.to_owned(), group);
    assert_eq!(
        statuses_of(&BUG),
        [
            entry("Open", StatusGroup::NotStarted),
            entry("In progress", StatusGroup::InProgress),
            entry("Testing", StatusGroup::InProgress),
            entry("Done", StatusGroup::Done),
            entry("Won't fix", StatusGroup::Done),
        ]
    );
    assert_eq!(
        statuses_of(&DEVELOPMENT),
        [
            entry("Open", StatusGroup::NotStarted),
            entry("In progress", StatusGroup::InProgress),
            entry("In review", StatusGroup::InProgress),
            entry("Testing", StatusGroup::InProgress),
            entry("Done", StatusGroup::Done),
        ]
    );
}

#[test]
fn wf_06_every_starter_status_is_reachable_and_can_be_left() {
    for starter in STARTERS {
        let content = starter.content();
        let first = content.initial().unwrap().clone();
        let mut reached = vec![first];
        let mut at = 0;
        while at < reached.len() {
            let from = reached[at].clone();
            for change in content.changes_from(&from) {
                if !reached.contains(&change.to) {
                    reached.push(change.to.clone());
                }
            }
            at += 1;
        }
        assert_eq!(reached.len(), content.statuses.len(), "{}", starter.title());
        // A status that cannot be left would keep its issues there for good.
        for status in &content.statuses {
            assert!(
                content.changes_from(&status.key).next().is_some(),
                "{} cannot be left in {}",
                status.name,
                starter.title()
            );
        }
    }
}

#[test]
fn wf_01_find_picks_the_nearest_place_and_an_issue_type_over_all_types() {
    let Ids {
        parent,
        child,
        bug,
        task,
        simple,
        own,
    } = ids();
    let chain = [parent, child];
    let uses = [
        WorkflowUse {
            project: None,
            issue_type: None,
            workflow: simple,
        },
        WorkflowUse {
            project: Some(parent),
            issue_type: Some(bug),
            workflow: own,
        },
    ];
    let found = |issue_type| find(&chain, issue_type, &uses).map(|each| each.workflow);
    // The parent's row for bugs is nearer than the organization's row for all types, and a
    // sub-project follows it.
    assert_eq!(found(Some(bug)), Some(own));
    assert_eq!(found(Some(task)), Some(simple));
    assert_eq!(found(None), Some(simple));
    // Nobody set a workflow: the starting workflow counts.
    assert_eq!(find(&chain, Some(bug), &[]), None);
    // A row of another project is no part of the answer.
    let elsewhere = [WorkflowUse {
        project: Some(child),
        issue_type: None,
        workflow: own,
    }];
    assert_eq!(find(&[parent], Some(bug), &elsewhere), None);
}

#[test]
fn wf_01_rows_and_content_convert_into_each_other() {
    let workflow = ids().own;
    let mut content = BUG.content();
    content.changes[0].rules.who.reporter = true;
    content.changes[0].rules.must_fill = vec![FieldKey::BuiltIn(Field::Description)];
    let (statuses, changes) = content.rows(workflow);
    assert_eq!(WorkflowContent::from_rows(&statuses, &changes), content);
    // The order of the rows does not matter, their positions do.
    let (mut statuses, mut changes) = (statuses, changes);
    statuses.reverse();
    changes.reverse();
    assert_eq!(WorkflowContent::from_rows(&statuses, &changes), content);
}

#[test]
fn wf_02_a_workflow_breaks_a_rule_when_it_cannot_work() {
    let good = SIMPLE.content();
    let broken = |change: fn(&mut WorkflowContent)| {
        let mut content = good.clone();
        change(&mut content);
        content.check()
    };
    assert_eq!(broken(|c| c.statuses.clear()), Err(Problem::NoStatus));
    assert_eq!(
        broken(|c| c.statuses[1].key = c.statuses[0].key.clone()),
        Err(Problem::KeyTwice)
    );
    assert_eq!(
        broken(|c| c.statuses[1].name = " OPEN ".into()),
        Err(Problem::NameTwice)
    );
    assert_eq!(
        broken(|c| c.statuses[0].name = "  ".into()),
        Err(Problem::NameEmpty)
    );
    assert_eq!(
        broken(|c| c.statuses[0].name = "x".repeat(NAME_MAX + 1)),
        Err(Problem::NameTooLong)
    );
    assert_eq!(
        broken(|c| c.changes[0].button.clear()),
        Err(Problem::NameEmpty)
    );
    assert_eq!(
        broken(|c| c.changes[0].to = StatusKey::new("gone").unwrap()),
        Err(Problem::UnknownStatus)
    );
    assert_eq!(
        broken(|c| c.changes[0].to = c.changes[0].from.clone()),
        Err(Problem::SameStatus)
    );
    assert_eq!(
        broken(|c| c.changes[1] = c.changes[0].clone()),
        Err(Problem::ChangeTwice)
    );
    assert_eq!(
        broken(|c| c.changes[0].rules.must_fill = vec![FieldKey::BuiltIn(Field::Priority)]),
        Err(Problem::FieldNotAllowed)
    );
    // The description can be required, but not asked for: a status change cannot carry it.
    assert_eq!(
        broken(|c| c.changes[0].rules.ask_for = vec![FieldKey::BuiltIn(Field::Description)]),
        Err(Problem::FieldNotAllowed)
    );
    assert_eq!(
        broken(|c| {
            c.statuses = (0..=STATUSES_MAX)
                .map(|at| WorkflowStatus {
                    key: StatusKey::new(&format!("s{at}")).unwrap(),
                    name: format!("S{at}"),
                    group: StatusGroup::NotStarted,
                })
                .collect();
            c.changes.clear();
        }),
        Err(Problem::TooBig)
    );
}

#[test]
fn wf_02_the_statuses_a_workflow_drops_are_the_ones_it_no_longer_has() {
    let before = DEVELOPMENT.content();
    let mut after = before.clone();
    after
        .statuses
        .retain(|status| status.key.as_str() != "testing");
    after
        .changes
        .retain(|change| change.from.as_str() != "testing" && change.to.as_str() != "testing");
    assert_eq!(before.dropped_by(&after), [key("testing")]);
    assert!(after.dropped_by(&before).is_empty());
}

fn draft(key: Option<&str>, name: &str, group: StatusGroup) -> StatusDraft {
    StatusDraft {
        key: key.map(|text| StatusKey::new(text).unwrap()),
        name: name.to_owned(),
        group,
    }
}

fn change_draft(from: &str, to: &str, button: &str) -> ChangeDraft {
    ChangeDraft {
        from: from.to_owned(),
        to: to.to_owned(),
        button: button.to_owned(),
        rules: StatusRules::default(),
    }
}

#[test]
fn wf_02_a_new_status_gets_a_key_and_a_status_change_names_it_by_its_name() {
    let statuses = [
        draft(Some("open"), "Open", StatusGroup::NotStarted),
        draft(None, "In QA", StatusGroup::InProgress),
        draft(Some("done"), "Done", StatusGroup::Done),
    ];
    let changes = [
        change_draft("open", "In QA", "Send to QA"),
        change_draft(" in qa ", "done", "Pass"),
        change_draft("done", "open", "Reopen"),
    ];
    let content = WorkflowContent::from_drafts(&statuses, &changes).unwrap();
    let keys: Vec<&str> = content
        .statuses
        .iter()
        .map(|each| each.key.as_str())
        .collect();
    assert_eq!(keys, ["open", "in-qa", "done"]);
    assert_eq!(content.changes[0].to.as_str(), "in-qa");
    assert_eq!(content.changes[1].from.as_str(), "in-qa");
    // A key that the workflow has counts before a name; a status that is not there is not one.
    let unknown = [change_draft("open", "nowhere", "Go")];
    assert_eq!(
        WorkflowContent::from_drafts(&statuses, &unknown),
        Err(Problem::UnknownStatus)
    );
    // Two new statuses with the same words get keys of their own, and then are one problem.
    let twice = [
        draft(None, "In QA", StatusGroup::InProgress),
        draft(None, "in qa", StatusGroup::InProgress),
    ];
    assert_eq!(
        WorkflowContent::from_drafts(&twice, &[]),
        Err(Problem::NameTwice)
    );
}

#[test]
fn wf_02_one_reading_names_a_new_status_for_the_editor_and_for_publishing() {
    let statuses = [
        WorkflowStatus {
            key: key("open"),
            name: "Open".to_owned(),
            group: StatusGroup::NotStarted,
        },
        WorkflowStatus {
            key: key("in-qa"),
            name: "In QA".to_owned(),
            group: StatusGroup::InProgress,
        },
    ];
    let is_new = |status: &WorkflowStatus| status.key.as_str() == "in-qa";
    let found = |text: &str| status_named(&statuses, text, is_new).map(|each| each.key.as_str());
    // By key; by name with spaces at the ends and other capitals.
    assert_eq!(found("in-qa"), Some("in-qa"));
    assert_eq!(found("  in qa "), Some("in-qa"));
    assert_eq!(found("IN QA"), Some("in-qa"));
    // A status that is not new is named by its key only.
    assert_eq!(found("open"), Some("open"));
    assert_eq!(found("Open"), None);
    assert_eq!(found("nowhere"), None);
}

#[test]
fn wf_02_the_key_of_a_new_status_is_made_from_its_name() {
    let none = |_: &str| false;
    assert_eq!(StatusKey::from_name("In QA", none).as_str(), "in-qa");
    assert_eq!(
        StatusKey::from_name("  Won't fix!  ", none).as_str(),
        "won-t-fix"
    );
    assert_eq!(
        StatusKey::from_name("2nd look", none).as_str(),
        "status-2nd-look"
    );
    assert_eq!(StatusKey::from_name("???", none).as_str(), "status");
    let taken = |key: &str| key == "in-qa" || key == "in-qa-2";
    assert_eq!(StatusKey::from_name("In QA", taken).as_str(), "in-qa-3");
    let long = StatusKey::from_name(&"word ".repeat(40), none);
    assert!(long.as_str().len() <= 40, "{long}");
    assert!(StatusKey::new(long.as_str()).is_ok());
}

#[test]
fn wf_03_who_can_do_it_with_nobody_named_is_for_everybody() {
    assert!(WhoCan::default().is_everybody());
    assert!(
        !WhoCan {
            reporter: true,
            ..WhoCan::default()
        }
        .is_everybody()
    );
}

#[test]
fn wf_01_a_status_key_in_json_is_its_text() {
    let json = serde_json::to_string(&key(IN_PROGRESS)).unwrap();
    assert_eq!(json, "\"in-progress\"");
    assert!(serde_json::from_str::<StatusKey>("\"In Progress\"").is_err());
    assert_eq!(key(DONE).to_string(), "done");
}
