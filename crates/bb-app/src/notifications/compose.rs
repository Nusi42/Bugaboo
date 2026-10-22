//! The text of an e-mail (NTF-02, ADR-0033 §3): what the notifications of one person about one
//! issue say, worded from the message catalog, and nothing else.
//!
//! The notifications were shaped for the recipient a moment before (`read.rs`), so the names,
//! fields and comments in them are what the person may see now. An e-mail is plain text: nothing is
//! escaped because nothing is markup, and nothing loads from outside. *Short e-mails* say only that
//! there is news and where: no title, no comment, no value of a field.

use bb_core::app_paths;
use bb_core::edit::NoticeKind;
use bb_core::fields::Field;
use bb_core::message::{self, Message};
use bb_core::notification::{MAIL_ENTRIES_MAX, MAIL_TEXT_MAX, NotificationKind};
use bb_core::text::{plain_line, plain_text};
use bb_policy::PersonRef;

use super::types::{NotificationItem, NotificationWhat};
use super::value_words::{Region, list_text, typed_text, word_of};
use crate::access::ProjectLabel;
use crate::history::{ChangeValue, FieldChangeView};

/// One e-mail: the subject on one line, and the text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Composed {
    pub(crate) subject: String,
    pub(crate) text: String,
}

/// What starts each line of a person's text in an e-mail, so a line they wrote cannot pass for the
/// text of Bugaboo: a link or a footer of their making is a quoted line.
const QUOTE: &str = "> ";

/// What ends a sentence of Bugaboo in an e-mail: the screen says it without, a line of an e-mail
/// has it.
const SENTENCE_END: &str = ".";

/// What ends the line before a comment that is quoted below it.
const LEAD_IN: &str = ":";

/// What already ends a sentence: a value of a person may end in one of these, then the sentence
/// gets no second mark.
const ENDS: [char; 3] = ['.', '?', '!'];

/// A sentence of the screen as a line of an e-mail.
fn sentence_of(message: &Message) -> String {
    let text = message.render();
    if text.ends_with(ENDS) {
        text
    } else {
        format!("{text}{SENTENCE_END}")
    }
}

/// Text of a person for the middle of a line.
fn clean(text: &str) -> String {
    plain_line(text, MAIL_TEXT_MAX)
}

/// A text of a person that stands on lines of its own, every line quoted.
fn quoted(text: &str) -> String {
    plain_text(text, MAIL_TEXT_MAX)
        .lines()
        .map(|line| format!("{QUOTE}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn who(person: Option<&PersonRef>) -> String {
    match person {
        None => Message::from(message::PERSON_BUGABOO).render(),
        Some(PersonRef::Name { name, .. }) => clean(name),
        Some(PersonRef::Role { role }) => message::person_role(clean(role)).render(),
        Some(PersonRef::Someone) => Message::from(message::PERSON_SOMEONE).render(),
        Some(PersonRef::Former { label }) => message::person_former(clean(label)).render(),
    }
}

/// A person in the middle of a sentence, where a role reads *a person with the role Lead*.
fn who_inside(person: &PersonRef) -> String {
    match person {
        PersonRef::Name { name, .. } => clean(name),
        PersonRef::Role { role } => message::person_role_inside(clean(role)).render(),
        PersonRef::Someone => Message::from(message::PERSON_SOMEONE_INSIDE).render(),
        PersonRef::Former { label } => message::person_former_inside(clean(label)).render(),
    }
}

/// One side of a change in words, as the history of the issue words it; `None` when the side holds
/// no value.
fn side_text(field: Option<Field>, value: Option<&ChangeValue>, region: &Region) -> Option<String> {
    let text = match value? {
        ChangeValue::Text { text } => word_of(field, text),
        ChangeValue::Names { names } => {
            list_text(&names.iter().map(|name| clean(name)).collect::<Vec<_>>())
        }
        ChangeValue::People { people } if people.is_empty() => {
            Message::from(message::HISTORY_PEOPLE_NONE).render()
        }
        ChangeValue::People { people } => {
            list_text(&people.iter().map(who_inside).collect::<Vec<_>>())
        }
        ChangeValue::Typed { kind, value } => typed_text(*kind, value, region)?,
        ChangeValue::Issue { issue } => issue.key.clone(),
        ChangeValue::Project { project } => project.name.clone(),
    };
    let text = clean(&text);
    (!text.is_empty()).then_some(text)
}

/// What a field is called: the name of an own field, or the name from the catalogue.
fn field_name(change: &FieldChangeView) -> String {
    match (&change.name, Field::from_code(&change.field)) {
        (Some(name), _) => clean(name),
        (None, Some(field)) => Message::from(field.name()).render(),
        (None, None) => clean(&change.field),
    }
}

/// The line about one change of a field: the sentence of the history of the issue, which the
/// screen says too, with a full stop.
fn change_line(by: Option<&PersonRef>, change: &FieldChangeView, region: &Region) -> String {
    let (by, field_name) = (who(by), field_name(change));
    let field = Field::from_code(&change.field);
    let before = side_text(field, change.before.as_ref(), region);
    let after = side_text(field, change.after.as_ref(), region);
    let sentence = match (before, after) {
        (Some(before), Some(after)) => message::history_changed(after, before, field_name, by),
        (None, Some(after)) => message::history_set(after, field_name, by),
        (Some(_), None) => message::history_cleared(field_name, by),
        (None, None) => message::history_changed_long_text(field_name, by),
    };
    sentence_of(&sentence)
}

/// The lines about one notification: the sentences the screen says (`notifications.*`, the
/// history), so the e-mail and the screen cannot say different things.
fn lines_of(item: &NotificationItem, short: bool, region: &Region) -> Vec<String> {
    // A short e-mail leaves out all issue text: what is about an issue says only that there is
    // news. The end of a run and a failed renewal are about no issue and say what they say.
    if short && item.issue.is_some() {
        return vec![news()];
    }
    // A short e-mail names no project either.
    if short && matches!(item.what, NotificationWhat::IssueIdsFailed { .. }) {
        return vec![sentence_of(&Message::from(
            message::NOTIFICATIONS_ISSUE_IDS_FAILED_SHORT,
        ))];
    }
    lines_about(item.by.as_ref(), &item.what, item.kind, region)
}

/// *There is news on this issue*, the sentence for what has no words of its own.
fn news() -> String {
    sentence_of(&Message::from(message::NOTIFICATIONS_NEWS))
}

fn lines_about(
    by: Option<&PersonRef>,
    what: &NotificationWhat,
    kind: NotificationKind,
    region: &Region,
) -> Vec<String> {
    match (what, kind) {
        (NotificationWhat::Comment { comment }, kind) => vec![
            format!(
                "{}{LEAD_IN}",
                if kind == NotificationKind::Mentioned {
                    message::notifications_mentioned(who(by))
                } else {
                    message::notifications_commented(who(by))
                }
                .render()
            ),
            quoted(&comment.text),
        ],
        (NotificationWhat::Changes { fields }, _) => fields
            .iter()
            .map(|change| change_line(by, change, region))
            .collect(),
        (NotificationWhat::EditNotice { notice, .. }, _) => vec![sentence_of(&match notice {
            NoticeKind::FinishAsked => message::notifications_finish_asked(who(by)),
            NoticeKind::TakenOver => message::notifications_taken_over(who(by)),
            NoticeKind::Released => message::notifications_released(who(by)),
        })],
        (NotificationWhat::RunEnded { done, .. }, _) => {
            vec![sentence_of(&Message::from(if *done {
                message::NOTIFICATIONS_RUN_DONE
            } else {
                message::NOTIFICATIONS_RUN_FAILED
            }))]
        }
        (NotificationWhat::IssueIdsFailed { project }, _) => {
            vec![sentence_of(&match project {
                ProjectLabel::Visible { name, .. } => {
                    message::notifications_issue_ids_failed(clean(name))
                }
                ProjectLabel::Hidden => {
                    Message::from(message::NOTIFICATIONS_ISSUE_IDS_FAILED_HIDDEN)
                }
            })]
        }
        (NotificationWhat::Plain, NotificationKind::Assigned) => {
            vec![sentence_of(&message::notifications_assigned(who(by)))]
        }
        (NotificationWhat::Plain, _) => vec![news()],
    }
}

/// The e-mail for the notifications of one person about one issue (or about no issue), oldest
/// first. `more` says that more waited than the e-mail holds. `region` is how the reader writes
/// days and numbers. `public_url` is the public address of the server, for the link.
/// `None` when there is nothing to say.
#[must_use]
pub(crate) fn compose(
    items: &[NotificationItem],
    more: bool,
    short: bool,
    region: &Region,
    public_url: &str,
) -> Option<Composed> {
    let first = items.first()?;
    let (subject, open) = match &first.issue {
        Some(issue) => (
            match (&issue.title, short) {
                (Some(title), false) => {
                    message::mail_subject(issue.key.clone(), clean(title)).render()
                }
                _ => message::mail_subject_short(issue.key.clone()).render(),
            },
            message::mail_open(format!("{public_url}{}", app_paths::issue(&issue.key))),
        ),
        // Work that is not about one issue ends in the notifications, so that is where the link goes.
        None => (
            Message::from(
                if matches!(first.what, NotificationWhat::IssueIdsFailed { .. }) {
                    message::MAIL_SUBJECT_IDS_FAILED
                } else {
                    message::MAIL_SUBJECT_RUN
                },
            )
            .render(),
            message::mail_open_notifications(format!("{public_url}{}", app_paths::NOTIFICATIONS)),
        ),
    };
    // The link of Bugaboo comes first, before any text of a person.
    let mut text = String::new();
    text.push_str(&open.render());
    text.push_str("\n\n");
    for item in items.iter().take(MAIL_ENTRIES_MAX) {
        for line in lines_of(item, short, region) {
            text.push_str(&line);
            text.push('\n');
        }
        // A renewal that gave up says to try again in the project settings: the link is there.
        if let (false, NotificationWhat::IssueIdsFailed { project }) = (short, &item.what)
            && let ProjectLabel::Visible { short_name, .. } = project
        {
            text.push_str(
                &message::mail_open_project_settings(format!(
                    "{public_url}{}",
                    app_paths::project_settings(short_name)
                ))
                .render(),
            );
            text.push('\n');
        }
        text.push('\n');
    }
    if more {
        text.push_str(&Message::from(message::MAIL_LINE_MORE).render());
        text.push_str("\n\n");
    }
    text.push_str(
        &message::mail_footer(
            Message::from(message::NAV_NOTIFICATION_SETTINGS).render(),
            Message::from(message::NAV_MY_SETTINGS).render(),
        )
        .render(),
    );
    text.push('\n');
    Some(Composed { subject, text })
}

#[cfg(test)]
#[path = "compose_tests.rs"]
mod tests;
