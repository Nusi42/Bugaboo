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

impl Level {
    /// The word used when a level is stored.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Organization => "organization",
            Self::Project => "project",
            Self::Person => "person",
        }
    }

    /// The level stored under `code`.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        [Self::Organization, Self::Project, Self::Person]
            .into_iter()
            .find(|level| level.code() == code)
    }
}

// The three fixed levels.
impl crate::personal::NotPersonal for Level {}

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
    key: &'static str,
    default: DefaultValue,
    levels: &'static [Level],
    changed_by: ChangedBy,
    help: MessageKey,
    lockable: bool,
}

impl SettingDefinition {
    /// Only the `settings!` macro builds definitions, so the registry is the one list.
    const fn new(
        key: &'static str,
        default: DefaultValue,
        levels: &'static [Level],
        changed_by: ChangedBy,
        help: MessageKey,
        lockable: bool,
    ) -> Self {
        Self {
            key,
            default,
            levels,
            changed_by,
            help,
            lockable,
        }
    }

    /// The key, `topic.name`: lower-case words joined by hyphens, like `security.two-step-required`.
    #[must_use]
    pub const fn key(&self) -> &'static str {
        self.key
    }

    /// The value when nobody set anything.
    #[must_use]
    pub const fn default(&self) -> DefaultValue {
        self.default
    }

    /// The levels it can be set on, from organization down to person.
    #[must_use]
    pub const fn levels(&self) -> &'static [Level] {
        self.levels
    }

    /// Who may change it.
    #[must_use]
    pub const fn changed_by(&self) -> ChangedBy {
        self.changed_by
    }

    /// The message that explains it in the settings dialog.
    #[must_use]
    pub const fn help(&self) -> MessageKey {
        self.help
    }

    /// Whether administrators can lock the value so lower levels cannot override it.
    #[must_use]
    pub const fn lockable(&self) -> bool {
        self.lockable
    }

    /// The topic the settings dialog groups it under: the first part of the key.
    #[must_use]
    pub fn topic(&self) -> &'static str {
        self.key
            .split_once('.')
            .map_or(self.key, |(topic, _)| topic)
    }
}

/// A stored value has another type than the setting declares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WrongType {
    /// The key of the setting.
    pub key: &'static str,
}

impl std::fmt::Display for WrongType {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "the stored value of `{}` has the wrong type",
            self.key
        )
    }
}

impl std::error::Error for WrongType {}

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
    ///
    /// # Errors
    /// Fails if the declared default has another type than the setting. The macro makes that
    /// impossible, so it would be a defect in the registry.
    pub fn default_value(&self) -> Result<T::Value, WrongType> {
        T::default_of(self.definition.default).ok_or(WrongType {
            key: self.definition.key,
        })
    }

    /// The value to use: the stored one, or the default when nothing is stored.
    ///
    /// # Errors
    /// Fails when a value is stored but has another type than the setting declares. The caller
    /// decides what to do; the wrong value is not silently replaced by the default.
    pub fn resolve(&self, stored: Option<&SettingValue>) -> Result<T::Value, WrongType> {
        match stored {
            Some(value) => T::read(value).ok_or(WrongType {
                key: self.definition.key,
            }),
            None => self.default_value(),
        }
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
            pub const $name: Setting<$kind> = Setting::new(SettingDefinition::new(
                $key,
                $kind::default_value($default),
                &[$(Level::$level),+],
                ChangedBy::$changed_by,
                $help,
                $lockable,
            ));
        )+

        /// Every setting, in the order of the declarations.
        pub const ALL: &[&SettingDefinition] = &[$(&$name.definition),+];
    };
}
pub(crate) use settings;

/// One stored value of a setting, on one level (ADR-0018). The key must be one of the registry
/// ([`find`]); `bb-store` is the only place that reads or writes these (ARCH-SETTINGS).
#[derive(Clone, Debug, PartialEq, Eq, crate::personal::Entity)]
pub struct StoredSetting {
    /// The key of the setting, as declared in the registry.
    #[not_personal]
    pub key: crate::technical::TechnicalName,
    /// The level the value is set on.
    #[not_personal]
    pub level: Level,
    /// The project or person the value is for; `None` on the organization level.
    #[not_personal]
    pub scope: Option<crate::id::EntityId>,
    /// The value. On the person level it is that person's own choice, so it counts as their data.
    #[personal(category = "content", basis = "contract")]
    pub value: SettingValue,
    /// Whether the value is locked: lower levels cannot override it.
    #[not_personal]
    pub locked: bool,
    /// When it was last changed.
    #[not_personal]
    pub updated_at: jiff::Timestamp,
}

/// The registry entry for `key`, if there is one.
#[must_use]
pub fn find(key: &str) -> Option<&'static SettingDefinition> {
    ALL.iter()
        .copied()
        .find(|definition| definition.key() == key)
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
                seen.insert(definition.key()),
                "{} is declared twice",
                definition.key()
            );
            let (topic, name) = definition.key().split_once('.').unwrap();
            for part in [topic, name] {
                assert!(
                    part.starts_with(|c: char| c.is_ascii_lowercase())
                        && part
                            .chars()
                            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                    "{}",
                    definition.key()
                );
            }
            assert_eq!(definition.topic(), topic);
        }
    }

    #[test]
    fn every_setting_explains_itself_and_has_a_level() {
        for definition in ALL {
            assert!(
                !Message::from(definition.help()).render().is_empty(),
                "{}",
                definition.key()
            );
            assert!(!definition.levels().is_empty(), "{}", definition.key());
            assert!(
                definition.levels().windows(2).all(|pair| pair[0] < pair[1]),
                "{}: list the levels from organization down to person",
                definition.key()
            );
        }
    }

    #[test]
    fn a_lockable_setting_has_a_lower_level_to_lock() {
        for definition in ALL.iter().filter(|definition| definition.lockable()) {
            assert!(
                definition.levels().len() > 1 && definition.levels()[0] == Level::Organization,
                "{}",
                definition.key()
            );
        }
    }

    #[test]
    fn the_default_has_the_declared_type() {
        assert_eq!(APPEARANCE_THEME.default_value().as_deref(), Ok("device"));
        assert_eq!(SECURITY_SESSION_IDLE_MINUTES.default_value(), Ok(30));
        assert_eq!(SECURITY_TWO_STEP_REQUIRED.default_value(), Ok(false));
    }

    #[test]
    fn resolve_uses_the_stored_value_and_reports_one_of_the_wrong_type() {
        let stored = SettingValue::Integer(10);
        assert_eq!(SECURITY_SESSION_IDLE_MINUTES.resolve(Some(&stored)), Ok(10));
        assert_eq!(SECURITY_SESSION_IDLE_MINUTES.resolve(None), Ok(30));
        let wrong = SettingValue::Text("soon".to_owned());
        assert_eq!(
            SECURITY_SESSION_IDLE_MINUTES.resolve(Some(&wrong)),
            Err(WrongType {
                key: "security.session-idle-minutes"
            })
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
