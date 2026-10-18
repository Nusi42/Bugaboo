//! The e-mail says what the screen says (WP-13): every line is a sentence of the catalog that the
//! history or the notifications of the app use too, with a full stop. These tests build the
//! sentence of the screen from its key and compare.

use bb_core::edit::NoticeKind;
use bb_core::id::SequentialIds;
use bb_core::message::{self, Message};
use bb_core::notification::NotificationKind;
use bb_core::own_fields::FieldType;
use bb_core::settings::Layers;
use bb_policy::PersonRef;
use serde_json::json;

use super::*;
use crate::history::{ChangeValue, FieldChangeView};

fn ana() -> PersonRef {
    PersonRef::Name {
        id: bb_core::id::UserId::generate(&mut SequentialIds::default()),
        name: "Ana".to_owned(),
    }
}

fn region() -> Region {
    Region::of(&Layers::default())
}

fn word(text: &str) -> ChangeValue {
    ChangeValue::Text {
        text: text.to_owned(),
    }
}

fn change(field: &str, before: Option<ChangeValue>, after: Option<ChangeValue>) -> FieldChangeView {
    FieldChangeView {
        field: field.to_owned(),
        name: None,
        before,
        after,
    }
}

fn line(change: &FieldChangeView) -> String {
    change_line(Some(&ana()), change, &region())
}

#[test]
fn wp13_a_change_reads_as_the_history_reads_with_a_full_stop() {
    assert_eq!(
        line(&change(
            "priority",
            Some(word("normal")),
            Some(word("high"))
        )),
        "Ana changed Priority from Normal to High."
    );
    assert_eq!(
        line(&change("title", None, Some(word("A new title")))),
        format!(
            "{}.",
            message::history_set("A new title", "Title", "Ana").render()
        )
    );
}

#[test]
fn wp13_people_are_set_after_a_colon_and_nobody_is_a_word() {
    let people = |people| Some(ChangeValue::People { people });
    assert_eq!(
        line(&change("assigned-to", None, people(vec![ana()]))),
        format!(
            "{}.",
            message::history_set("Ana", "Assigned to", "Ana").render()
        )
    );
    assert_eq!(
        line(&change("assigned-to", people(vec![ana()]), people(vec![]))),
        "Ana changed Assigned to from Ana to nobody."
    );
}

#[test]
fn wp13_a_change_of_a_description_is_not_called_cleared() {
    assert_eq!(
        line(&change("description", None, None)),
        format!(
            "{}.",
            message::history_changed_long_text("Description", "Ana").render()
        )
    );
    assert_eq!(
        line(&change("title", Some(word("Old")), None)),
        format!("{}.", message::history_cleared("Title", "Ana").render())
    );
}

#[test]
fn wp13_yes_no_and_lengths_of_time_are_words_not_code_values() {
    let typed = |kind, value| Some(ChangeValue::Typed { kind, value });
    assert_eq!(
        line(&change(
            "status",
            None,
            typed(FieldType::YesNo, json!(true))
        )),
        format!("{}.", message::history_set("Yes", "Status", "Ana").render())
    );
    assert_eq!(
        line(&change(
            "status",
            typed(FieldType::Duration, json!(5400)),
            typed(FieldType::Date, json!("2026-12-31")),
        )),
        "Ana changed Status from 1h 30m to 2026-12-31."
    );
}

#[test]
fn wp13_the_notice_of_a_released_edit_is_not_the_one_of_a_takeover() {
    let notice = |notice| {
        lines_about(
            Some(&ana()),
            &NotificationWhat::EditNotice {
                notice,
                part: "description".to_owned(),
            },
            NotificationKind::EditNotice,
            &region(),
        )
    };
    assert_eq!(
        notice(NoticeKind::Released),
        [format!(
            "{}.",
            message::notifications_released("Ana").render()
        )]
    );
    assert_ne!(notice(NoticeKind::Released), notice(NoticeKind::TakenOver));
    assert_eq!(
        notice(NoticeKind::FinishAsked),
        [format!(
            "{}.",
            message::notifications_finish_asked("Ana").render()
        )]
    );
}

#[test]
fn wp13_a_failed_renewal_of_issue_ids_has_its_own_sentence() {
    assert_eq!(
        lines_about(
            None,
            &NotificationWhat::IssueIdsFailed {
                project: ProjectLabel::Hidden
            },
            NotificationKind::RunEnded,
            &region()
        ),
        ["Bugaboo could not give the issues of a project their new IDs. You cannot see that project any more.".to_owned()]
    );
}

#[test]
fn wp13_no_line_of_an_e_mail_ends_in_two_full_stops() {
    let sentences = [
        NotificationWhat::IssueIdsFailed {
            project: ProjectLabel::Hidden,
        },
        NotificationWhat::Plain,
        NotificationWhat::EditNotice {
            notice: NoticeKind::TakenOver,
            part: "description".to_owned(),
        },
    ];
    for what in sentences {
        for line in lines_about(Some(&ana()), &what, NotificationKind::Assigned, &region()) {
            assert!(!line.contains(".."), "{line}");
        }
    }
}

#[test]
fn wp13_the_end_of_a_run_and_the_news_use_the_sentences_of_the_screen() {
    let ended = |done| {
        lines_about(
            None,
            &NotificationWhat::RunEnded {
                run: bb_core::id::EntityId::from(bb_core::id::UserId::generate(
                    &mut SequentialIds::default(),
                )),
                done,
            },
            NotificationKind::RunEnded,
            &region(),
        )
    };
    assert_eq!(
        ended(true),
        [format!(
            "{}.",
            Message::from(message::NOTIFICATIONS_RUN_DONE).render()
        )]
    );
    assert_eq!(
        ended(false),
        [format!(
            "{}.",
            Message::from(message::NOTIFICATIONS_RUN_FAILED).render()
        )]
    );
    assert_eq!(
        lines_about(
            None,
            &NotificationWhat::Plain,
            NotificationKind::FollowedFields,
            &region()
        ),
        [format!(
            "{}.",
            Message::from(message::NOTIFICATIONS_NEWS).render()
        )]
    );
}

#[test]
fn wp13_a_short_e_mail_says_what_a_run_did_because_it_is_about_no_issue()
-> Result<(), bb_core::id::IdError> {
    let mut ids = SequentialIds::default();
    let item = NotificationItem {
        id: bb_core::id::NotificationId::generate(&mut ids)?,
        kind: NotificationKind::RunEnded,
        at: jiff::Timestamp::UNIX_EPOCH,
        read: false,
        issue: None,
        comment: None,
        by: None,
        what: NotificationWhat::RunEnded {
            run: bb_core::id::EntityId::from(bb_core::id::UserId::generate(&mut ids)),
            done: true,
        },
    };
    assert_eq!(
        lines_of(&item, true, &region()),
        [format!(
            "{}.",
            Message::from(message::NOTIFICATIONS_RUN_DONE).render()
        )]
    );
    Ok(())
}
