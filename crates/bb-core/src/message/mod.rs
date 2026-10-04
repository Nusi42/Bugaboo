//! Messages: every text a person reads comes from `messages/en.json` by key (UX-07).
//!
//! `cargo xtask gen messages` writes `generated.rs` from the catalog: one constant per message
//! without placeholders and one function per message with placeholders, with the placeholders as
//! typed arguments. Code never writes a message key as text, so a key that does not exist does
//! not compile (TXT-CATALOG finds keys that nothing uses).

use jiff::Timestamp;

mod generated;
mod render;

pub use generated::*;

/// The key of one message in the catalog.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MessageKey(&'static str);

impl MessageKey {
    /// Only the generated code makes keys, from the catalog.
    pub(crate) const fn new(key: &'static str) -> Self {
        Self(key)
    }

    /// The key as written in the catalog, for example `error.resource.not-found`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }

    /// The English template of this message, with its placeholders still in it.
    #[must_use]
    pub fn template(self) -> &'static str {
        CATALOG
            .binary_search_by(|(key, _)| (*key).cmp(self.0))
            .map_or(self.0, |found| CATALOG[found].1)
    }
}

impl std::fmt::Display for MessageKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.0)
    }
}

/// A value for a placeholder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    /// Text, or anything shown as text.
    Text(String),
    /// A whole number, for `number` and `plural` placeholders.
    Number(i64),
    /// A moment, for `date` and `time` placeholders.
    Date(Timestamp),
}

impl Value {
    fn to_text(&self) -> String {
        match self {
            Self::Text(text) => text.clone(),
            Self::Number(number) => number.to_string(),
            Self::Date(moment) => moment.to_string(),
        }
    }

    fn as_number(&self) -> Option<i64> {
        match self {
            Self::Number(number) => Some(*number),
            Self::Text(_) | Self::Date(_) => None,
        }
    }
}

/// A message with the values for its placeholders, ready to be shown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    key: MessageKey,
    arguments: Vec<(&'static str, Value)>,
}

impl Message {
    /// Only the generated functions make messages with arguments, so the arguments always match.
    pub(crate) fn with_arguments(key: MessageKey, arguments: Vec<(&'static str, Value)>) -> Self {
        Self { key, arguments }
    }

    /// The key, for clients that format the message themselves.
    #[must_use]
    pub const fn key(&self) -> MessageKey {
        self.key
    }

    /// The values of the placeholders, by name.
    #[must_use]
    pub fn arguments(&self) -> &[(&'static str, Value)] {
        &self.arguments
    }

    /// The English text with the values filled in.
    #[must_use]
    pub fn render(&self) -> String {
        render::render(self.key.template(), &self.arguments)
    }
}

impl From<MessageKey> for Message {
    fn from(key: MessageKey) -> Self {
        Self {
            key,
            arguments: Vec::new(),
        }
    }
}

impl std::fmt::Display for Message {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.render())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ux_07_a_message_without_placeholders_renders_its_catalog_text() {
        assert_eq!(Message::from(APP_NAME).render(), "Bugaboo");
        assert_eq!(APP_NAME.as_str(), "app.name");
        assert_eq!(APP_NAME.to_string(), "app.name");
    }

    #[test]
    fn ux_07_a_message_with_placeholders_takes_typed_arguments() {
        let message = hello_current("Dusk");
        assert_eq!(message.key().as_str(), "hello.current");
        assert_eq!(message.arguments().len(), 1);
        assert_eq!(message.to_string(), "Your theme: Dusk.");
    }

    #[test]
    fn the_catalog_is_sorted_so_lookups_find_every_key() {
        assert!(CATALOG.windows(2).all(|pair| pair[0].0 < pair[1].0));
        for (key, text) in CATALOG {
            assert!(!text.is_empty(), "{key}");
        }
    }
}
