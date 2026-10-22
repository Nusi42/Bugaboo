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

/// The line of an e-mail about one change of a field.
fn line(change: &FieldChangeView) -> String {
    let what = NotificationWhat::Changes {
        fields: vec![change.clone()],
    };
    lines_about(
        Some(&ana()),
        &what,
        NotificationKind::FollowedFields,
        &region(),
    )
    .remove(0)
}

#[test]
fn ntf_02_a_change_reads_as_the_history_reads_with_a_full_stop() {
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
fn ntf_02_people_are_set_after_a_colon_and_nobody_is_a_word() {
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
fn ntf_02_a_change_of_a_description_is_not_called_cleared() {
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
fn ntf_02_yes_no_and_lengths_of_time_are_words_not_code_values() {
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
fn ntf_02_the_notice_of_a_released_edit_is_not_the_one_of_a_takeover() {
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
fn ntf_02_a_failed_renewal_of_issue_ids_has_its_own_sentence() {
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
fn ntf_02_no_line_of_an_e_mail_ends_in_two_full_stops() {
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
fn ntf_02_the_end_of_a_run_and_the_news_use_the_sentences_of_the_screen() {
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
fn ntf_02_a_short_e_mail_says_what_a_run_did_because_it_is_about_no_issue()
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

#[test]
fn ntf_02_a_value_that_ends_a_sentence_gets_no_second_mark() {
    for end in [".", "?", "!"] {
        let value = format!("Login fails{end}");
        let said = line(&change("title", None, Some(word(&value))));
        assert!(said.ends_with(&value), "{said}");
        assert!(!said.ends_with(&format!("{value}.")), "{said}");
    }
    assert!(line(&change("title", None, Some(word("Login fails")))).ends_with("Login fails."));
}

fn run_item(what: NotificationWhat) -> Result<NotificationItem, bb_core::id::IdError> {
    let mut ids = SequentialIds::default();
    Ok(NotificationItem {
        id: bb_core::id::NotificationId::generate(&mut ids)?,
        kind: NotificationKind::RunEnded,
        at: jiff::Timestamp::UNIX_EPOCH,
        read: false,
        issue: None,
        comment: None,
        by: None,
        what,
    })
}

/// The e-mail of the items, for the address of the test server.
fn mail_of(items: &[NotificationItem], short: bool) -> Result<Composed, String> {
    compose(items, false, short, &region(), "https://bb.test").ok_or_else(|| "no e-mail".to_owned())
}

#[test]
fn ntf_02_the_e_mail_of_the_end_of_a_run_links_to_the_notifications() -> Result<(), String> {
    let mut ids = SequentialIds::default();
    let item = run_item(NotificationWhat::RunEnded {
        run: bb_core::id::EntityId::from(bb_core::id::UserId::generate(&mut ids)),
        done: true,
    })
    .map_err(|error| error.to_string())?;
    let mail = mail_of(&[item], false)?;
    assert_eq!(
        mail.subject,
        Message::from(message::MAIL_SUBJECT_RUN).render()
    );
    let link = message::mail_open_notifications("https://bb.test/notifications").render();
    assert!(mail.text.starts_with(&link), "{}", mail.text);
    Ok(())
}

fn renewal(project: ProjectLabel) -> Result<NotificationItem, bb_core::id::IdError> {
    run_item(NotificationWhat::IssueIdsFailed { project })
}

fn visible() -> Result<ProjectLabel, bb_core::id::IdError> {
    Ok(ProjectLabel::Visible {
        id: bb_core::id::ProjectId::generate(&mut SequentialIds::default())?,
        short_name: "WEB".to_owned(),
        name: "The web site".to_owned(),
    })
}

#[test]
fn ntf_02_the_e_mail_of_a_failed_renewal_names_a_project_the_person_sees_and_links_to_it()
-> Result<(), String> {
    let item = renewal(visible().map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())?;
    let mail = mail_of(&[item], false)?;
    assert_eq!(
        mail.subject,
        Message::from(message::MAIL_SUBJECT_IDS_FAILED).render()
    );
    assert!(
        mail.text.contains(
            "Bugaboo could not give every issue of The web site its new ID. Open the project settings to try again.\n"
        ),
        "{}",
        mail.text
    );
    let settings = message::mail_open_project_settings("https://bb.test/p/WEB/settings").render();
    assert!(mail.text.contains(&settings), "{}", mail.text);
    let notifications = message::mail_open_notifications("https://bb.test/notifications").render();
    assert!(mail.text.starts_with(&notifications), "{}", mail.text);
    Ok(())
}

#[test]
fn ntf_02_the_short_e_mail_of_a_failed_renewal_names_no_project_and_has_no_link_to_one()
-> Result<(), String> {
    let item = renewal(visible().map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())?;
    let mail = mail_of(&[item], true)?;
    assert!(!mail.text.contains("The web site"), "{}", mail.text);
    assert!(!mail.text.contains("WEB"), "{}", mail.text);
    assert!(
        mail.text
            .contains("Bugaboo could not give every issue of a project its new ID.\n"),
        "{}",
        mail.text
    );
    Ok(())
}

#[test]
fn ntf_02_the_footer_names_the_page_where_the_person_changes_what_they_get() -> Result<(), String> {
    let item = renewal(ProjectLabel::Hidden).map_err(|error| error.to_string())?;
    let mail = mail_of(&[item], false)?;
    assert!(
        mail.text
            .ends_with("Bugaboo under My settings › Notification settings.\n"),
        "{}",
        mail.text
    );
    Ok(())
}
