//! The settings registry (ADR-0018): every setting is declared once, here in `bb-core`.
//!
//! A setting has a key, a type, a default, the levels it can be set on (organization, project,
//! person), who may change it, the message that explains it, and whether administrators can lock
//! it. The registry ([`ALL`]) lists them all; storage (WP-05), the API and the settings dialogs
//! are built from that list, never from a second one. Code reads a setting only through its typed
//! handle (`settings::APPEARANCE_THEME`), never by writing its key as text (ARCH-SETTINGS).

use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use crate::message::MessageKey;

mod catalog;

pub use catalog::*;

/// Where a setting can be set. A lower level overrides a higher one unless it is locked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    /// For the whole Bugaboo server.
    Organization,
    /// For one project and its sub-projects.
    Project,
    /// For one person (*My settings*).
    Person,
}

/// The least privileged group that may change a setting. Administrators can always change it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChangedBy {
    /// Only administrators.
    Administrators,
    /// Administrators and project leads.
    ProjectLeads,
    /// Each person, for their own value.
    EachPerson,
}

/// What a setting holds, as it is stored and sent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SettingValue {
    /// Yes or no.
    Bool(bool),
    /// A whole number.
    Integer(i64),
    /// Text, or one choice from a list written as text.
    Text(String),
}

/// The default of a setting, in a form a `const` can hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefaultValue {
    /// Yes or no.
    Bool(bool),
    /// A whole number.
    Integer(i64),
    /// Text.
    Text(&'static str),
}

impl DefaultValue {
    /// The default as a storable value.
    #[must_use]
    pub fn to_value(self) -> SettingValue {
        match self {
            Self::Bool(value) => SettingValue::Bool(value),
            Self::Integer(value) => SettingValue::Integer(value),
            Self::Text(value) => SettingValue::Text(value.to_owned()),
        }
    }
}

/// Everything the registry knows about one setting, whatever its type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingDefinition {
    /// The key, `topic.name`: lower-case words joined by hyphens, like `security.two-step-required`.
    pub key: &'static str,
    /// The value when nobody set anything.
    pub default: DefaultValue,
    /// The levels it can be set on.
    pub levels: &'static [Level],
    /// Who may change it.
    pub changed_by: ChangedBy,
    /// The message that explains it in the settings dialog.
    pub help: MessageKey,
    /// Whether administrators can lock the value so lower levels cannot override it.
    pub lockable: bool,
}

impl SettingDefinition {
    /// The topic the settings dialog groups it under: the first part of the key.
    #[must_use]
    pub fn topic(&self) -> &'static str {
        self.key
            .split_once('.')
            .map_or(self.key, |(topic, _)| topic)
    }
}

/// A type a setting can have. Only [`Bool`], [`Integer`] and [`Text`] exist.
pub trait SettingKind {
    /// The Rust type of the value.
    type Value;

    /// The value stored in `value`, if it is of this kind.
    fn read(value: &SettingValue) -> Option<Self::Value>;

    /// The default as a value of this kind.
    fn default_of(default: DefaultValue) -> Option<Self::Value>;
}

/// Settings that are yes or no.
pub struct Bool;
/// Settings that are whole numbers.
pub struct Integer;
/// Settings that are text.
pub struct Text;

impl Bool {
    const fn default_value(value: bool) -> DefaultValue {
        DefaultValue::Bool(value)
    }
}

impl Integer {
    const fn default_value(value: i64) -> DefaultValue {
        DefaultValue::Integer(value)
    }
}

impl Text {
    const fn default_value(value: &'static str) -> DefaultValue {
        DefaultValue::Text(value)
    }
}

impl SettingKind for Bool {
    type Value = bool;

    fn read(value: &SettingValue) -> Option<bool> {
        match value {
            SettingValue::Bool(value) => Some(*value),
            SettingValue::Integer(_) | SettingValue::Text(_) => None,
        }
    }

    fn default_of(default: DefaultValue) -> Option<bool> {
        Self::read(&default.to_value())
    }
}

impl SettingKind for Integer {
    type Value = i64;

    fn read(value: &SettingValue) -> Option<i64> {
        match value {
            SettingValue::Integer(value) => Some(*value),
            SettingValue::Bool(_) | SettingValue::Text(_) => None,
        }
    }

    fn default_of(default: DefaultValue) -> Option<i64> {
        Self::read(&default.to_value())
    }
}

impl SettingKind for Text {
    type Value = String;

    fn read(value: &SettingValue) -> Option<String> {
        match value {
            SettingValue::Text(value) => Some(value.clone()),
            SettingValue::Bool(_) | SettingValue::Integer(_) => None,
        }
    }

    fn default_of(default: DefaultValue) -> Option<String> {
        Self::read(&default.to_value())
    }
}

/// A setting with its type. This is what code uses to read a setting.
pub struct Setting<T: SettingKind> {
    definition: SettingDefinition,
    kind: PhantomData<fn() -> T>,
}

impl<T: SettingKind> Setting<T> {
    const fn new(definition: SettingDefinition) -> Self {
        Self {
            definition,
            kind: PhantomData,
        }
    }

    /// The registry entry.
    #[must_use]
    pub const fn definition(&self) -> &SettingDefinition {
        &self.definition
    }

    /// The default value.
    #[must_use]
    pub fn default_value(&self) -> Option<T::Value> {
        T::default_of(self.definition.default)
    }

    /// The value to use: the stored one if it has the right type, otherwise the default.
    #[must_use]
    pub fn resolve(&self, stored: Option<&SettingValue>) -> Option<T::Value> {
        stored.and_then(T::read).or_else(|| self.default_value())
    }
}

/// Declares settings. Used only in `catalog.rs`, so the registry is the one list (ARCH-SETTINGS).
macro_rules! settings {
    ($(
        $(#[$doc:meta])*
        $name:ident: $kind:ident = {
            key: $key:literal,
            default: $default:expr,
            levels: [$($level:ident),+ $(,)?],
            changed_by: $changed_by:ident,
            help: $help:expr,
            lockable: $lockable:literal $(,)?
        }
    )+) => {
        $(
            $(#[$doc])*
            pub const $name: Setting<$kind> = Setting::new(SettingDefinition {
                key: $key,
                default: $kind::default_value($default),
                levels: &[$(Level::$level),+],
                changed_by: ChangedBy::$changed_by,
                help: $help,
                lockable: $lockable,
            });
        )+

        /// Every setting, in the order of the declarations.
        pub const ALL: &[&SettingDefinition] = &[$(&$name.definition),+];
    };
}
pub(crate) use settings;

/// The registry entry for `key`, if there is one.
#[must_use]
pub fn find(key: &str) -> Option<&'static SettingDefinition> {
    ALL.iter().copied().find(|definition| definition.key == key)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::message::Message;

    #[test]
    fn keys_are_unique_topic_dot_name_words() {
        let mut seen = HashSet::new();
        for definition in ALL {
            assert!(
                seen.insert(definition.key),
                "{} is declared twice",
                definition.key
            );
            let (topic, name) = definition.key.split_once('.').unwrap();
            for part in [topic, name] {
                assert!(
                    part.starts_with(|c: char| c.is_ascii_lowercase())
                        && part
                            .chars()
                            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                    "{}",
                    definition.key
                );
            }
            assert_eq!(definition.topic(), topic);
        }
    }

    #[test]
    fn every_setting_explains_itself_and_has_a_level() {
        for definition in ALL {
            assert!(
                !Message::from(definition.help).render().is_empty(),
                "{}",
                definition.key
            );
            assert!(!definition.levels.is_empty(), "{}", definition.key);
            assert!(
                definition.levels.windows(2).all(|pair| pair[0] < pair[1]),
                "{}: list the levels from organization down to person",
                definition.key
            );
        }
    }

    #[test]
    fn a_lockable_setting_has_a_lower_level_to_lock() {
        for definition in ALL.iter().filter(|definition| definition.lockable) {
            assert!(
                definition.levels.len() > 1 && definition.levels[0] == Level::Organization,
                "{}",
                definition.key
            );
        }
    }

    #[test]
    fn the_default_has_the_declared_type() {
        assert_eq!(APPEARANCE_THEME.default_value().as_deref(), Some("device"));
        assert_eq!(SECURITY_SESSION_IDLE_MINUTES.default_value(), Some(30));
        assert_eq!(SECURITY_TWO_STEP_REQUIRED.default_value(), Some(false));
    }

    #[test]
    fn resolve_uses_the_stored_value_only_when_it_has_the_right_type() {
        let stored = SettingValue::Integer(10);
        assert_eq!(
            SECURITY_SESSION_IDLE_MINUTES.resolve(Some(&stored)),
            Some(10)
        );
        assert_eq!(SECURITY_SESSION_IDLE_MINUTES.resolve(None), Some(30));
        let wrong = SettingValue::Text("soon".to_owned());
        assert_eq!(
            SECURITY_SESSION_IDLE_MINUTES.resolve(Some(&wrong)),
            Some(30)
        );
    }

    #[test]
    fn find_looks_a_key_up_in_the_registry() {
        assert_eq!(
            find("appearance.theme"),
            Some(APPEARANCE_THEME.definition())
        );
        assert_eq!(find("appearance.nothing"), None);
    }

    #[test]
    fn values_are_stored_as_plain_json() {
        assert_eq!(
            serde_json::to_string(&SettingValue::Bool(true)).unwrap(),
            "true"
        );
        assert_eq!(
            serde_json::from_str::<SettingValue>("30").unwrap(),
            SettingValue::Integer(30)
        );
        assert_eq!(
            serde_json::from_str::<SettingValue>("\"dusk\"").unwrap(),
            SettingValue::Text("dusk".to_owned())
        );
    }
}
