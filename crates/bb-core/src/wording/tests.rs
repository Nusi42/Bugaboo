//! The wording of a change and of a notification: the choice of sentence, the words of values, and
//! the shape that the browser reads.

use serde_json::json;

use super::*;
use crate::notification::NotificationKind;

fn text(text: &str) -> Option<Side> {
    Some(Side::Text {
        text: text.to_owned(),
    })
}

fn change(field: &str, before: Option<Side>, after: Option<Side>) -> Change {
    Change {
        who: "Ben".to_owned(),
        field: field.to_owned(),
        field_name: "Priority".to_owned(),
        before,
        after,
    }
}

#[test]
fn a_change_is_changed_set_cleared_or_a_long_text() {
    let said = |before, after| change_sentence(&change("priority", before, after)).render();
    assert_eq!(
        said(text("normal"), text("high")),
        "Ben changed Priority from Normal to High"
    );
    assert_eq!(said(None, text("high")), "Ben set Priority: High");
    assert_eq!(said(text("normal"), None), "Ben cleared Priority");
    assert_eq!(said(None, None), "Ben changed Priority");
}

#[test]
fn a_status_and_a_priority_have_their_words_and_other_text_stays() {
    let word = |field, text| word_of(Field::from_code(field), text).render();
    assert_eq!(word("status", "in-progress"), "In progress");
    assert_eq!(word("status", "my-own-status"), "my-own-status");
    assert_eq!(word("priority", "critical"), "Critical");
    assert_eq!(word("priority", "urgent!"), "urgent!");
    assert_eq!(word("title", "open"), "open");
}

#[test]
fn names_are_listed_the_way_the_catalog_lists_them() {
    let list = |names: &[&str]| {
        let names: Vec<String> = names.iter().map(|name| (*name).to_owned()).collect();
        list_said(&names).map(|said| said.render())
    };
    assert_eq!(list(&[]), None);
    assert_eq!(list(&["Ana"]).as_deref(), Some("Ana"));
    assert_eq!(list(&["Ana", "Ben"]).as_deref(), Some("Ana and Ben"));
    assert_eq!(
        list(&["Ana", "Ben", "Cem"]).as_deref(),
        Some("Ana, Ben, and Cem")
    );
}

#[test]
fn no_person_reads_as_nobody_and_no_name_as_no_value() {
    let field = Field::from_code("assigned-to");
    let nobody = Side::People { people: Vec::new() };
    assert_eq!(
        side_said(field, &nobody)
            .map(|said| said.render())
            .as_deref(),
        Some("nobody")
    );
    let none = Side::Names { names: Vec::new() };
    assert_eq!(side_said(field, &none), None);
}

#[test]
fn yes_no_and_lengths_of_time_are_words_not_code_values() {
    let typed = |kind, value| typed_said(kind, &value, None).map(|said| said.render());
    assert_eq!(typed(FieldType::YesNo, json!(true)).as_deref(), Some("Yes"));
    assert_eq!(typed(FieldType::YesNo, json!("maybe")), None);
    assert_eq!(
        typed(FieldType::Duration, json!(7500)).as_deref(),
        Some("2h 5m")
    );
    assert_eq!(
        typed(FieldType::Duration, json!(7200)).as_deref(),
        Some("2h")
    );
    assert_eq!(
        typed(FieldType::Duration, json!(2700)).as_deref(),
        Some("45m")
    );
    assert_eq!(
        typed(FieldType::Duration, json!(59.9)).as_deref(),
        Some("0m")
    );
    assert_eq!(typed(FieldType::Duration, json!(-90)), None);
    assert_eq!(
        typed(FieldType::Text, json!("hello")).as_deref(),
        Some("hello")
    );
}

#[test]
fn a_value_of_a_region_is_the_words_the_caller_wrote() {
    let value = json!(1_234_567);
    assert_eq!(
        typed_said(FieldType::WholeNumber, &value, Some("1.234.567")).map(|said| said.render()),
        Some("1.234.567".to_owned())
    );
    assert_eq!(typed_said(FieldType::WholeNumber, &value, None), None);
}

#[test]
fn a_sentence_that_ends_in_a_mark_gets_no_second_one() {
    assert_eq!(end_sentence("Login fails"), "Login fails.");
    for end in [".", "?", "!"] {
        let text = format!("Login fails{end}");
        assert_eq!(end_sentence(&text), text);
    }
}

fn notice(kind: NotificationKind) -> Notice {
    Notice {
        kind,
        who: "Ana".to_owned(),
        comment: false,
        changes: Vec::new(),
        notice: None,
        run_done: None,
        renewal: None,
    }
}

fn sentences(notice: &Notice) -> Vec<String> {
    notification_sentences(notice)
        .iter()
        .map(Said::render)
        .collect()
}

#[test]
fn the_sentences_of_a_notification_follow_its_kind() {
    assert_eq!(
        sentences(&notice(NotificationKind::Mentioned)),
        ["Ana mentioned you"]
    );
    assert_eq!(
        sentences(&notice(NotificationKind::Assigned)),
        ["Ana assigned this issue to you"]
    );
    let mut commented = notice(NotificationKind::FollowedActivity);
    commented.comment = true;
    assert_eq!(sentences(&commented), ["Ana wrote a comment"]);
    assert_eq!(
        sentences(&notice(NotificationKind::FollowedFields)),
        ["There is news on this issue"]
    );
    let mut changes = notice(NotificationKind::FollowedFields);
    changes.changes = vec![
        change("priority", text("normal"), text("high")),
        change("priority", None, text("low")),
    ];
    assert_eq!(sentences(&changes).len(), 2);
}

#[test]
fn the_end_of_a_run_and_a_failed_renewal_have_their_own_sentences() {
    let mut run = notice(NotificationKind::RunEnded);
    run.run_done = Some(true);
    assert_eq!(sentences(&run), ["Your change to many issues is done"]);
    run.run_done = Some(false);
    assert_eq!(
        sentences(&run),
        ["Your change to many issues stopped before it was done"]
    );
    run.renewal = Some(Renewal {
        project: Some("Website".to_owned()),
    });
    assert_eq!(
        sentences(&run),
        [
            "Bugaboo could not give every issue of Website its new ID. Open the project settings to try again."
        ]
    );
    run.renewal = Some(Renewal { project: None });
    assert_eq!(
        sentences(&run),
        [
            "Bugaboo could not give the issues of a project their new IDs. You cannot see that project any more."
        ]
    );
}

#[test]
fn a_note_about_editing_says_which_note() {
    let mut edit = notice(NotificationKind::EditNotice);
    edit.notice = Some(NoticeKind::Released);
    assert!(sentences(&edit)[0].contains("ended your editing"));
    edit.notice = Some(NoticeKind::TakenOver);
    assert!(sentences(&edit)[0].contains("took over"));
}

#[test]
fn the_browser_gets_keys_and_words_not_english() {
    let said = change_sentence(&change("priority", text("normal"), text("high")));
    let wire = serde_json::to_value(&said).unwrap_or_default();
    assert_eq!(
        wire,
        json!({
            "key": "history.changed",
            "args": {
                "after": {"key": "priority.high", "args": {}},
                "before": {"key": "priority.normal", "args": {}},
                "field": "Priority",
                "who": "Ben",
            }
        })
    );
    let duration = typed_said(FieldType::Duration, &json!(5400), None);
    assert_eq!(
        serde_json::to_value(duration).unwrap_or_default(),
        json!([
            "1",
            {"key": "fields.unit-hour", "args": {}},
            " ",
            "30",
            {"key": "fields.unit-minute", "args": {}}
        ])
    );
}

#[test]
fn the_input_of_the_browser_reads_as_a_change() {
    let input = json!({
        "who": "Ben",
        "field": "status",
        "field_name": "Status",
        "after": {"shown": "text", "text": "done"},
    });
    let parsed: Result<Change, _> = serde_json::from_value(input);
    assert_eq!(
        parsed
            .map(|change| change_sentence(&change).render())
            .ok()
            .as_deref(),
        Some("Ben set Status: Done")
    );
}
