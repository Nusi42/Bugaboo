//! The words of a change and of a notification, chosen once for the server and the browser
//! (ADR-0033, the amendment about value wording).
//!
//! This code picks the message keys and builds the words of the values: the choice of sentence
//! (*changed*, *set*, *cleared*, the long text), the words of a status and a priority, yes and no,
//! a length of time and a list of names, and the sentences of a notification. It writes no text of
//! its own: the result is a [`Said`], a tree of message keys and values, and each side looks the
//! keys up in its own catalog. The server renders it with the English catalog it holds
//! ([`Said::render`]); the browser gets the same tree from `bb-wasm` and renders it with its
//! catalog.
//!
//! What depends on the region (the order of a day, the marks of a number, the zone of a moment) is
//! not decided here: the screen uses `Intl` for the choice *device*, which a server cannot run. The
//! caller words those values (the server with [`Region`]) and hands the words in. The two ways of
//! writing are held together by test vectors that this crate makes ([`vectors`]).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::edit::NoticeKind;
use crate::fields::Field;
use crate::issue::Priority;
use crate::message::{self, Message, MessageKey};
use crate::notification::NotificationKind;
use crate::own_fields::FieldType;
use crate::time::{MINUTES_PER_HOUR, SECONDS_PER_MINUTE};
use crate::workflow::starter::STARTERS;

pub mod region;
pub mod vectors;

pub use region::Region;

/// Some words: a text, words that follow each other, or a message of the catalog with the words
/// for its placeholders. In JSON a text is a string, words that follow each other are a list, and
/// a message is an object with its `key` and its `args` (`word_types.rs` writes the browser's type
/// from this declaration).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Said {
    /// Text that is not in the catalog: a name, a number, a text of a person.
    Text(String),
    /// Words that follow each other without anything between them.
    Parts(Vec<Said>),
    /// A message of the catalog; the browser looks `key` up in its catalog.
    Message {
        /// The key of the message.
        key: MessageKey,
        /// The words for its placeholders, by the name of the placeholder.
        args: BTreeMap<&'static str, Said>,
    },
}

/// The empty text for each value of a call of [`said!`]: the typed function of the catalog is
/// called with it, which checks the number of placeholders when the program is built.
macro_rules! blank {
    ($_value:expr) => {
        ""
    };
}

/// A message of the catalog with the words for its placeholders, given in the order of the
/// parameters of the message function.
macro_rules! said {
    ($function:path, $($value:expr),+ $(,)?) => {
        Said::with(&$function($(blank!($value)),+), vec![$($value),+])
    };
}

impl Said {
    /// A text that is not in the catalog.
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    /// A message without placeholders.
    #[must_use]
    pub const fn key(key: MessageKey) -> Self {
        Self::Message {
            key,
            args: BTreeMap::new(),
        }
    }

    /// A message with the words for its placeholders, in the order of the message function's
    /// parameters (the typed function of the catalog gave `message` its names).
    fn with(message: &Message, values: Vec<Self>) -> Self {
        let names = message.arguments().iter().map(|(name, _)| *name);
        Self::Message {
            key: message.key(),
            args: names.zip(values).collect(),
        }
    }

    /// The English words, as the server writes them.
    #[must_use]
    pub fn render(&self) -> String {
        match self {
            Self::Text(text) => text.clone(),
            Self::Parts(parts) => parts.iter().map(Self::render).collect(),
            Self::Message { key, args } => Message::with_arguments(
                *key,
                args.iter()
                    .map(|(name, said)| (*name, message::Value::Text(said.render())))
                    .collect(),
            )
            .render(),
        }
    }
}

/// What ends a sentence of Bugaboo in a line of an e-mail: the screen says it without.
const SENTENCE_END: &str = ".";

/// What already ends a sentence: a value of a person may end in one of these, then the sentence
/// gets no second mark.
const ENDS: [char; 3] = ['.', '?', '!'];

/// A sentence with a full stop, unless it ends in a full stop, a question mark or an exclamation
/// mark already (a value of a person may end in one: *Login fails.*).
#[must_use]
pub fn end_sentence(text: &str) -> String {
    if text.ends_with(ENDS) {
        text.to_owned()
    } else {
        format!("{text}{SENTENCE_END}")
    }
}

/// Names in a sentence as the catalog lists them: `Ana, Ben, and Cleo`. `None` for no name.
#[must_use]
pub fn list_said(items: &[String]) -> Option<Said> {
    let said = |text: &String| Said::text(text.as_str());
    match items {
        [] => None,
        [one] => Some(said(one)),
        [first, second] => Some(said!(message::list_two, said(first), said(second))),
        [start @ .., last] => Some(said!(
            message::list_many,
            said(last),
            Said::text(start.join(", "))
        )),
    }
}

/// The word of a status or a priority; any other text as it is.
#[must_use]
pub fn word_of(field: Option<Field>, text: &str) -> Said {
    let known = match field {
        Some(Field::Status) => STARTERS
            .iter()
            .flat_map(|starter| starter.statuses)
            .find(|status| status.key == text)
            .map(|status| status.name),
        Some(Field::Priority) => Priority::from_code(text).map(Priority::name),
        _ => None,
    };
    known.map_or_else(|| Said::text(text), Said::key)
}

/// Seconds as "2h 5m", "45m" or "0m": whole minutes; a part of a second is dropped like the rest of
/// a minute.
fn duration_said(seconds: f64) -> Said {
    let (per_minute, per_hour) = (f64::from(SECONDS_PER_MINUTE), f64::from(MINUTES_PER_HOUR));
    let minutes_all = (seconds / per_minute).floor();
    let hours = (minutes_all / per_hour).floor();
    let minutes = minutes_all - hours * per_hour;
    let hour = Said::key(message::FIELDS_UNIT_HOUR);
    let minute = Said::key(message::FIELDS_UNIT_MINUTE);
    if hours == 0.0 {
        Said::Parts(vec![Said::text(minutes.to_string()), minute])
    } else if minutes == 0.0 {
        Said::Parts(vec![Said::text(hours.to_string()), hour])
    } else {
        Said::Parts(vec![
            Said::text(hours.to_string()),
            hour,
            Said::text(" "),
            Said::text(minutes.to_string()),
            minute,
        ])
    }
}

/// The words of a value of an own field. `worded` is the words of a number, a day or a moment in
/// the region of the reader, which the caller wrote (see [`Region::worded`]); it is `None` when the
/// value does not fit its kind. The result is `None` for a value that does not fit the kind (the
/// screen shows nothing for it either).
#[must_use]
pub fn typed_said(kind: FieldType, value: &Value, worded: Option<&str>) -> Option<Said> {
    match (kind, value) {
        (FieldType::YesNo, Value::Bool(yes)) => Some(Said::key(if *yes {
            message::FIELDS_YES
        } else {
            message::FIELDS_NO
        })),
        (FieldType::Duration, Value::Number(seconds)) => seconds
            .as_f64()
            .filter(|seconds| *seconds >= 0.0)
            .map(duration_said),
        (FieldType::WholeNumber | FieldType::Decimal, Value::Number(_))
        | (FieldType::Date | FieldType::DateTime, Value::String(_)) => worded.map(Said::text),
        (FieldType::WholeNumber | FieldType::Decimal, Value::String(text)) => {
            Some(Said::text(text.as_str()))
        }
        (
            FieldType::YesNo
            | FieldType::Duration
            | FieldType::WholeNumber
            | FieldType::Decimal
            | FieldType::Date
            | FieldType::DateTime,
            _,
        ) => None,
        (_, Value::String(text)) => Some(Said::text(text.as_str())),
        _ => None,
    }
}

/// One side of a change: what the field was or what it became.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "shown", rename_all = "kebab-case")]
pub enum Side {
    /// A word: a status, a priority or any other text.
    Text {
        /// The text.
        text: String,
    },
    /// Names, already written for the reader. No name reads as no value.
    Names {
        /// The names.
        names: Vec<String>,
    },
    /// People, already named for the reader. No person reads as *nobody*.
    People {
        /// The names, as they stand in the middle of a sentence.
        people: Vec<String>,
    },
    /// A value of an own field.
    Typed {
        /// The kind of the field.
        kind: FieldType,
        /// The value, as the API gives it.
        value: Value,
        /// The words of a number, a day or a moment in the region of the reader.
        #[serde(default)]
        worded: Option<String>,
    },
    /// An issue ID or the name of a project, as it stands.
    Plain {
        /// The text.
        text: String,
    },
}

/// The words of one side of a change; `None` when the side holds no value.
#[must_use]
pub fn side_said(field: Option<Field>, side: &Side) -> Option<Said> {
    let said = match side {
        Side::Text { text } => word_of(field, text),
        Side::Names { names } => list_said(names)?,
        Side::People { people } if people.is_empty() => Said::key(message::HISTORY_PEOPLE_NONE),
        Side::People { people } => list_said(people)?,
        Side::Typed {
            kind,
            value,
            worded,
        } => typed_said(*kind, value, worded.as_deref())?,
        Side::Plain { text } => Said::text(text.as_str()),
    };
    (said != Said::text("")).then_some(said)
}

/// A change of one field, with the words the reader sees for the people and the field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Change {
    /// Who did it, at the start of a sentence.
    pub who: String,
    /// The code of the field: a built-in field, or the ID of an own field.
    pub field: String,
    /// What the field is called.
    pub field_name: String,
    /// What it was, if it held a value.
    #[serde(default)]
    pub before: Option<Side>,
    /// What it became, if it holds a value.
    #[serde(default)]
    pub after: Option<Side>,
}

/// The sentence of a change of one field: *Ben changed Priority from Normal to High*.
#[must_use]
pub fn change_sentence(change: &Change) -> Said {
    let field = Field::from_code(&change.field);
    let side = |side: &Option<Side>| side.as_ref().and_then(|side| side_said(field, side));
    let (who, name) = (
        Said::text(change.who.as_str()),
        Said::text(change.field_name.as_str()),
    );
    match (side(&change.before), side(&change.after)) {
        (Some(before), Some(after)) => {
            said!(message::history_changed, after, before, name, who)
        }
        // After a colon for every field: a field such as *Assigned to* ends in the word that
        // *set … to* would need.
        (None, Some(after)) => said!(message::history_set, after, name, who),
        (Some(_), None) => said!(message::history_cleared, name, who),
        (None, None) => said!(message::history_changed_long_text, name, who),
    }
}

/// A renewal of issue IDs that gave up.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Renewal {
    /// The name of the project, if the reader may know it.
    #[serde(default)]
    pub project: Option<String>,
}

/// What a notification says, with what the sentences need.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notice {
    /// What the notification is about.
    pub kind: NotificationKind,
    /// Who did it, at the start of a sentence.
    pub who: String,
    /// Whether it is about a comment.
    #[serde(default)]
    pub comment: bool,
    /// The changes of fields it is about.
    #[serde(default)]
    pub changes: Vec<Change>,
    /// What a note about the reader's editing says.
    #[serde(default)]
    pub notice: Option<NoticeKind>,
    /// Whether a run of many changes ended well.
    #[serde(default)]
    pub run_done: Option<bool>,
    /// A renewal of issue IDs that gave up.
    #[serde(default)]
    pub renewal: Option<Renewal>,
}

/// What happened, in words: one sentence, or one for each field a change touched.
#[must_use]
pub fn notification_sentences(notice: &Notice) -> Vec<Said> {
    let who = || Said::text(notice.who.as_str());
    let one = |said| vec![said];
    match notice.kind {
        NotificationKind::Mentioned => one(said!(message::notifications_mentioned, who())),
        NotificationKind::Assigned => one(said!(message::notifications_assigned, who())),
        NotificationKind::EditNotice => one(match notice.notice {
            Some(NoticeKind::TakenOver) => said!(message::notifications_taken_over, who()),
            Some(NoticeKind::Released) => said!(message::notifications_released, who()),
            Some(NoticeKind::FinishAsked) | None => {
                said!(message::notifications_finish_asked, who())
            }
        }),
        NotificationKind::RunEnded => one(match (&notice.renewal, notice.run_done) {
            (
                Some(Renewal {
                    project: Some(name),
                }),
                _,
            ) => said!(
                message::notifications_issue_ids_failed,
                Said::text(name.as_str())
            ),
            (Some(Renewal { project: None }), _) => {
                Said::key(message::NOTIFICATIONS_ISSUE_IDS_FAILED_HIDDEN)
            }
            (None, Some(false)) => Said::key(message::NOTIFICATIONS_RUN_FAILED),
            (None, _) => Said::key(message::NOTIFICATIONS_RUN_DONE),
        }),
        NotificationKind::FollowedActivity | NotificationKind::FollowedFields => {
            if !notice.changes.is_empty() {
                notice.changes.iter().map(change_sentence).collect()
            } else if notice.comment {
                one(said!(message::notifications_commented, who()))
            } else {
                one(Said::key(message::NOTIFICATIONS_NEWS))
            }
        }
    }
}

#[cfg(test)]
mod tests;
