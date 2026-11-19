//! [`Said`], the words of the wording as a tree of message keys and values, and `said!`, which
//! builds one from a message of the catalog with its words checked by name when the program is
//! built.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::message::{self, Message, MessageKey};

/// Some words: a text, a count, words that follow each other, or a message of the catalog with the
/// words for its placeholders. In JSON a text is a string, a count a number, words that follow each
/// other are a list, and a message is an object with its `key` and its `args` (`word_types.rs`
/// writes the browser's type from this declaration).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Said {
    /// Text that is not in the catalog: a name, a number, a text of a person.
    Text(String),
    /// A count that a plural of the catalog chooses its words by (`# hours`). Only a real count is
    /// one: a text of digits (`0815`) stays a text.
    Count(u32),
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

/// The value of a field of [`said!`], written out (`who: who()`) or by its name alone (`who`).
macro_rules! field_value {
    ($name:ident) => {
        $name
    };
    ($name:ident : $value:expr) => {
        $value
    };
}

/// A message of the catalog with the words for its placeholders. A message with one placeholder
/// takes its words as they are; a message with several takes them by the names of the struct of
/// its values (`said!(message::list_two, message::ListTwo { first: a, second: b })`). The typed
/// function of the catalog is called with empty texts in their place, which checks the names when
/// the program is built.
macro_rules! said {
    ($function:path, $($values:ident)::+ { $($name:ident $(: $value:expr)?),+ $(,)? }) => {
        Said::named(
            &$function($($values)::+ { $($name: &""),+ }),
            vec![$((stringify!($name), field_value!($name $(: $value)?))),+],
        )
    };
    ($function:path, $value:expr $(,)?) => {
        Said::with(&$function(""), $value)
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

    /// A message with one placeholder and the words for it (the typed function of the catalog gave
    /// `message` its one name). A message with several takes its words by name ([`Self::named`]).
    pub(crate) fn with(message: &Message, value: Self) -> Self {
        let name = message.arguments().first().map(|(name, _)| *name);
        Self::Message {
            key: message.key(),
            args: name.map(|name| (name, value)).into_iter().collect(),
        }
    }

    /// A message with the words for its placeholders, each given by the name of its field in the
    /// struct of the message's values ([`message::rust_name`]).
    pub(crate) fn named(message: &Message, mut values: Vec<(&'static str, Self)>) -> Self {
        let args = message
            .arguments()
            .iter()
            .filter_map(|(name, _)| {
                let field = message::rust_name(name);
                let at = values.iter().position(|(given, _)| *given == field)?;
                Some((*name, values.swap_remove(at).1))
            })
            .collect();
        Self::Message {
            key: message.key(),
            args,
        }
    }

    /// The English words, as the server writes them.
    #[must_use]
    pub fn render(&self) -> String {
        match self {
            Self::Text(text) => text.clone(),
            Self::Count(count) => count.to_string(),
            Self::Parts(parts) => parts.iter().map(Self::render).collect(),
            Self::Message { key, args } => Message::with_arguments(
                *key,
                args.iter()
                    .map(|(name, said)| {
                        let value = match said {
                            Self::Count(count) => message::Value::Number(i64::from(*count)),
                            _ => message::Value::Text(said.render()),
                        };
                        (*name, value)
                    })
                    .collect(),
            )
            .render(),
        }
    }
}
