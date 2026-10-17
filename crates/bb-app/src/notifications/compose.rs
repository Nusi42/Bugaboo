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

/// A value of a change in words.
fn value_text(value: &ChangeValue) -> String {
    let text = match value {
        ChangeValue::Text { text } => text.clone(),
        ChangeValue::Names { names } => names.join(", "),
        ChangeValue::People { people } => people
            .iter()
            .map(|person| who(Some(person)))
            .collect::<Vec<_>>()
            .join(", "),
        ChangeValue::Typed { value, .. } => match value.as_str() {
            Some(text) => text.to_owned(),
            None => value.to_string(),
        },
        ChangeValue::Issue { issue } => issue.key.clone(),
        ChangeValue::Project { project } => project.name.clone(),
    };
    clean(&text)
}

/// What a field is called: the name of an own field, or the name from the catalogue.
fn field_name(change: &FieldChangeView) -> String {
    match (&change.name, Field::from_code(&change.field)) {
        (Some(name), _) => clean(name),
        (None, Some(field)) => Message::from(field.name()).render(),
        (None, None) => clean(&change.field),
    }
}

/// The line about one change of a field.
fn change_line(by: Option<&PersonRef>, change: &FieldChangeView) -> String {
    let (by, field) = (who(by), field_name(change));
    match change.after.as_ref().map(value_text) {
        Some(value) if !value.is_empty() => message::mail_line_changed(field, value, by).render(),
        _ => message::mail_line_cleared(field, by).render(),
    }
}

/// The lines about one notification.
fn lines_of(item: &NotificationItem, short: bool) -> Vec<String> {
    let by = item.by.as_ref();
    if short {
        return vec![Message::from(message::MAIL_LINE_NEWS).render()];
    }
    match (&item.what, item.kind) {
        (NotificationWhat::Comment { comment }, NotificationKind::Mentioned) => vec![
            message::mail_line_mentioned(who(by)).render(),
            quoted(&comment.text),
        ],
        (NotificationWhat::Comment { comment }, _) => vec![
            message::mail_line_commented(who(by)).render(),
            quoted(&comment.text),
        ],
        (NotificationWhat::Changes { fields }, _) => fields
            .iter()
            .map(|change| change_line(by, change))
            .collect(),
        (NotificationWhat::EditNotice { notice, .. }, _) => vec![match notice {
            NoticeKind::FinishAsked => message::mail_line_finish_asked(who(by)).render(),
            NoticeKind::TakenOver | NoticeKind::Released => {
                message::mail_line_taken_over(who(by)).render()
            }
        }],
        (NotificationWhat::RunEnded { done, .. }, _) => vec![
            Message::from(if *done {
                message::MAIL_LINE_RUN_DONE
            } else {
                message::MAIL_LINE_RUN_FAILED
            })
            .render(),
        ],
        (NotificationWhat::Plain, NotificationKind::Assigned) => {
            vec![message::mail_line_assigned(who(by)).render()]
        }
        (NotificationWhat::Plain, _) => vec![Message::from(message::MAIL_LINE_NEWS).render()],
    }
}

/// The e-mail for the notifications of one person about one issue (or about no issue), oldest
/// first. `more` says that more waited than the e-mail holds. `address` is where the issue is in
/// Bugaboo, from the public address of the server.
/// `None` when there is nothing to say.
#[must_use]
pub(crate) fn compose(
    items: &[NotificationItem],
    more: bool,
    short: bool,
    public_url: &str,
) -> Option<Composed> {
    let first = items.first()?;
    let (subject, address) = match &first.issue {
        Some(issue) => (
            match (&issue.title, short) {
                (Some(title), false) => {
                    message::mail_subject(issue.key.clone(), clean(title)).render()
                }
                _ => message::mail_subject_short(issue.key.clone()).render(),
            },
            Some(format!("{public_url}{}", app_paths::issue(&issue.key))),
        ),
        None => (Message::from(message::MAIL_SUBJECT_RUN).render(), None),
    };
    // The link of Bugaboo comes first, before any text of a person.
    let mut text = String::new();
    if let Some(address) = address {
        text.push_str(&message::mail_open(address).render());
        text.push_str("\n\n");
    }
    for item in items.iter().take(MAIL_ENTRIES_MAX) {
        for line in lines_of(item, short) {
            text.push_str(&line);
            text.push('\n');
        }
        text.push('\n');
    }
    if more {
        text.push_str(&Message::from(message::MAIL_LINE_MORE).render());
        text.push_str("\n\n");
    }
    text.push_str(&Message::from(message::MAIL_FOOTER).render());
    text.push('\n');
    Some(Composed { subject, text })
}
