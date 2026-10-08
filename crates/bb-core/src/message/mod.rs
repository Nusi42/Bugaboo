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

    /// The key that [`derived_key!`](crate::derived_key) made from a word. TXT-CATALOG and the
    /// checks of the tables hold every such key against the catalog, so no key is made up.
    #[doc(hidden)]
    #[must_use]
    pub const fn __derived(key: &'static str) -> Self {
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

impl serde::Serialize for MessageKey {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0)
    }
}

impl std::fmt::Display for MessageKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.0)
    }
}

/// The bytes of a key made from a prefix and a word: the prefix, then the word with each `_`
/// written as `-` (the way the catalog spells a word that has underscores).
#[doc(hidden)]
#[must_use]
pub const fn derive_key<const LENGTH: usize>(prefix: &str, word: &str) -> [u8; LENGTH] {
    let (prefix, word) = (prefix.as_bytes(), word.as_bytes());
    let mut key = [0_u8; LENGTH];
    let mut at = 0;
    while at < prefix.len() {
        key[at] = prefix[at];
        at += 1;
    }
    let mut from = 0;
    while from < word.len() {
        key[at + from] = if word[from] == b'_' { b'-' } else { word[from] };
        from += 1;
    }
    key
}

/// The text of the bytes that [`derive_key`] made. They are letters, digits and punctuation, so
/// they are always text.
#[doc(hidden)]
#[must_use]
pub const fn key_text(bytes: &'static [u8]) -> &'static str {
    match core::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(_) => "",
    }
}

/// The key of the message that belongs to a word: `derived_key!("error.", "auth.wrong_credentials")`
/// is `error.auth.wrong-credentials`. It is made when the program is built, from the word that is
/// written once next to the variant, so no table pairs a word with a key by hand. The key has to be
/// in the catalog: ARCH-ERRORS, ARCH-PERMS and TXT-CATALOG find one that is not.
#[macro_export]
macro_rules! derived_key {
    ($prefix:literal, $word:literal) => {{
        const LENGTH: usize = $prefix.len() + $word.len();
        const BYTES: [u8; LENGTH] = $crate::message::derive_key::<LENGTH>($prefix, $word);
        const TEXT: &str = $crate::message::key_text(&BYTES);
        $crate::message::MessageKey::__derived(TEXT)
    }};
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
        let message = shell_signed_in_as("Ana");
        assert_eq!(message.key().as_str(), "shell.signed-in-as");
        assert_eq!(message.arguments().len(), 1);
        assert_eq!(message.to_string(), "Signed in as Ana");
    }

    #[test]
    fn a_derived_key_is_the_prefix_and_the_word_with_hyphens() {
        let key = crate::derived_key!("error.", "auth.wrong_credentials");
        assert_eq!(key, ERROR_AUTH_WRONG_CREDENTIALS);
        assert_eq!(key.as_str(), "error.auth.wrong-credentials");
        assert_eq!(
            crate::derived_key!("permission.", "see-projects"),
            PERMISSION_SEE_PROJECTS
        );
    }

    #[test]
    fn the_catalog_is_sorted_so_lookups_find_every_key() {
        assert!(CATALOG.windows(2).all(|pair| pair[0].0 < pair[1].0));
        for (key, text) in CATALOG {
            assert!(!text.is_empty(), "{key}");
        }
    }
}
