//! The settings registry (ADR-0018): every setting is declared once, here in `bb-core`.
//!
//! A setting has a key, a type, a default, the levels it can be set on (organization, project,
//! person), who may change it, the message that names it and the one that explains it, and whether
//! administrators can lock it. The registry ([`ALL`]) lists them all; storage, the API and the
//! settings forms are built from that list, never from a second one. Code reads a setting only
//! through its typed handle (`settings::APPEARANCE_MODE`), never by writing its key as text
//! (ARCH-SETTINGS).

use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use crate::message::MessageKey;

mod catalog;
mod format;
mod layers;
mod themes;

pub use catalog::*;
pub use format::{Choice, DEVICE, Format, is_host};
pub use layers::{Layers, OrganizationValue, Resolved, Scope, Source};

words! {
    /// Where a setting can be set. A lower level overrides a higher one unless it is locked. The
    /// word is how a level is stored.
    pub enum Level {
        /// For the whole Bugaboo server.
        Organization => "organization",
        /// For one project and its sub-projects.
        Project => "project",
        /// For one person (*My settings*).
        Person => "person",
    }
}

/// The topics whose settings change how people sign in or what keeps the organization safe.
const SECURITY_TOPICS: [&str; 2] = ["security", "auth"];

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

/// The two messages of a setting: the one that names it and the one that explains it.
#[derive(Clone, Copy)]
pub(crate) struct Words {
    pub(crate) name: MessageKey,
    pub(crate) help: MessageKey,
}

/// The limits a setting can declare: the bounds of a whole number, the size of a list.
#[derive(Clone, Copy)]
pub(crate) struct Limits {
    pub(crate) bounds: Option<(i64, i64)>,
    pub(crate) list: Option<(usize, usize)>,
    pub(crate) format: Format,
}

/// Everything the registry knows about one setting, whatever its type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingDefinition {
    key: &'static str,
    name: MessageKey,
    default: DefaultValue,
    levels: &'static [Level],
    changed_by: ChangedBy,
    help: MessageKey,
    lockable: bool,
    bounds: Option<(i64, i64)>,
    list: Option<(usize, usize)>,
    format: Format,
}

impl SettingDefinition {
    /// Only the `settings!` macro builds definitions, so the registry is the one list.
    const fn new(
        key: &'static str,
        default: DefaultValue,
        levels: &'static [Level],
        changed_by: ChangedBy,
        lockable: bool,
        words: Words,
        limits: Limits,
    ) -> Self {
        Self {
            key,
            name: words.name,
            default,
            levels,
            changed_by,
            help: words.help,
            lockable,
            bounds: limits.bounds,
            list: limits.list,
            format: limits.format,
        }
    }

    /// The kind of text the setting holds, and the choices when it has a list.
    #[must_use]
    pub const fn format(&self) -> Format {
        self.format
    }

    /// Whether `value` is one the setting accepts: the type, the bounds of a number, the limits of
    /// a list and the format of a text. The levels and the lock are not part of the value.
    #[must_use]
    pub fn accepts(&self, value: &SettingValue) -> bool {
        match (self.default, value) {
            (DefaultValue::Bool(_), SettingValue::Bool(_)) => true,
            (DefaultValue::Integer(_), SettingValue::Integer(value)) => self
                .bounds
                .is_none_or(|(least, most)| (least..=most).contains(value)),
            (DefaultValue::Text(_), SettingValue::Text(value)) => {
                self.list_fits(value) && self.format.accepts(value)
            }
            _ => false,
        }
    }

    /// Whether `value` has the type the setting declares, whatever its content.
    #[must_use]
    pub fn has_type_of(&self, value: &SettingValue) -> bool {
        matches!(
            (self.default, value),
            (DefaultValue::Bool(_), SettingValue::Bool(_))
                | (DefaultValue::Integer(_), SettingValue::Integer(_))
                | (DefaultValue::Text(_), SettingValue::Text(_))
        )
    }

    /// Whether a change of this setting is a change of how people sign in or what keeps the
    /// organization safe, so it also goes into the security log (CFG-06, SEC-15).
    #[must_use]
    pub fn touches_security(&self) -> bool {
        SECURITY_TOPICS.contains(&self.topic())
    }

    /// The message that names the setting in the settings form.
    #[must_use]
    pub const fn name(&self) -> MessageKey {
        self.name
    }

    /// For a text setting that holds a list separated by commas: the most items it may have and
    /// the most characters of an item. A longer list or item is refused when it is stored, and cut
    /// away when it is read.
    #[must_use]
    pub const fn list_limits(&self) -> Option<(usize, usize)> {
        self.list
    }

    /// Whether `value` (a list separated by commas) is within the limits of the setting, if it
    /// has any.
    #[must_use]
    pub fn list_fits(&self, value: &str) -> bool {
        self.list.is_none_or(|(most_items, longest)| {
            list_items(value).count() <= most_items
                && list_items(value).all(|item| item.chars().count() <= longest)
        })
    }

    /// `value` with only the items that fit: the first ones, up to the most items, without those
    /// that are too long.
    #[must_use]
    pub fn cap_list(&self, value: &str) -> String {
        let Some((most_items, longest)) = self.list else {
            return value.to_owned();
        };
        list_items(value)
            .filter(|item| item.chars().count() <= longest)
            .take(most_items)
            .collect::<Vec<_>>()
            .join(",")
    }

    /// The smallest and the largest value a whole-number setting may have, if it is bounded. A
    /// value outside is refused when it is stored, and cut to the bounds when it is read (a
    /// session that lasts a second, or a year, is a mistake).
    #[must_use]
    pub const fn bounds(&self) -> Option<(i64, i64)> {
        self.bounds
    }

    /// `value` cut to the bounds of the setting, if it has any.
    #[must_use]
    pub fn clamp(&self, value: i64) -> i64 {
        self.bounds
            .map_or(value, |(least, most)| value.clamp(least, most))
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
            name: $name_message:expr,
            default: $default:expr,
            levels: [$($level:ident),+ $(,)?],
            changed_by: $changed_by:ident,
            help: $help:expr,
            lockable: $lockable:literal
            $(, bounds: ($least:literal, $most:literal))?
            $(, list: ($items:literal, $length:literal))?
            $(, format: $format:expr)? $(,)?
        }
    )+) => {
        $(
            $(#[$doc])*
            pub const $name: Setting<$kind> = Setting::new(SettingDefinition::new(
                $key,
                $kind::default_value($default),
                &[$(Level::$level),+],
                ChangedBy::$changed_by,
                $lockable,
                $crate::settings::Words {
                    name: $name_message,
                    help: $help,
                },
                $crate::settings::Limits {
                    // The last entry is the bounds when the setting declares them, else `None`.
                    bounds: {
                        let all: [Option<(i64, i64)>; _] = [None $(, Some(($least, $most)))?];
                        all[all.len() - 1]
                    },
                    // The same for the limits of a list.
                    list: {
                        let all: [Option<(usize, usize)>; _] = [None $(, Some(($items, $length)))?];
                        all[all.len() - 1]
                    },
                    // The format of a text: free text unless the setting says otherwise.
                    format: {
                        let all: [$crate::settings::Format; _] =
                            [$crate::settings::Format::Free $(, $format)?];
                        all[all.len() - 1]
                    },
                },
            ));
        )+

        /// Every setting, in the order of the declarations.
        pub const ALL: &[&SettingDefinition] = &[$(&$name.definition),+];
    };
}
pub(crate) use settings;

/// One stored value of a setting for the whole organization or for one project (ADR-0018). The key
/// must be one of the registry ([`find`]); `bb-store` is the only place that reads or writes these
/// (ARCH-SETTINGS). A person's own values are [`StoredPersonSetting`].
#[derive(Clone, Debug, PartialEq, Eq, crate::personal::Entity)]
pub struct StoredSetting {
    /// The key of the setting, as declared in the registry.
    #[not_personal]
    pub key: crate::technical::TechnicalName,
    /// The level the value is set on: the organization or a project.
    #[not_personal]
    pub level: Level,
    /// The project the value is for; `None` on the organization level.
    #[not_personal]
    pub scope: Option<crate::id::EntityId>,
    /// The value. A free text of the organization could name a person, so it is marked.
    #[personal(category = "content", basis = "contract")]
    pub value: SettingValue,
    /// Whether the value is locked: lower levels cannot override it.
    #[not_personal]
    pub locked: bool,
    /// When it was last changed.
    #[not_personal]
    pub updated_at: jiff::Timestamp,
}

/// The record name of the sign-up policy: it sets `auth.sign-up-enabled` and `auth.sign-up-domains`
/// together, as one record with one version, through its own route only.
pub const SIGN_UP_POLICY: &str = "auth.sign-up-policy";

/// Whether a setting is changed only through its own screen and route, not through the generic
/// ones (`change_setting`, `reset_setting`, `lock_setting`), and so is not listed in the generic
/// settings screen: the settings of a policy that is one record with one version.
#[must_use]
pub fn changed_only_on_its_own_screen(key: &str) -> bool {
    [
        AUTH_SIGN_UP_ENABLED.definition(),
        AUTH_SIGN_UP_DOMAINS.definition(),
    ]
    .iter()
    .any(|definition| definition.key() == key)
}

/// The version of a record that is made of several settings (the sign-up policy). It lives in a
/// row of its own, which no change or reset of a setting writes or deletes. A policy without a row
/// is at version 0.
#[derive(Clone, Debug, PartialEq, Eq, crate::personal::Entity)]
pub struct PolicyVersion {
    /// The record name of the policy ([`SIGN_UP_POLICY`]).
    #[not_personal]
    pub policy: crate::technical::TechnicalName,
    /// Counts up with every change of the policy.
    #[not_personal]
    pub version: i64,
}

/// One value of a setting that a person chose for themselves in *My settings* (CFG-04). It is the
/// person's own data: it goes with them when they download their data and is gone when they are
/// removed. The key must be one of the registry ([`find`]); `bb-store` is the only place that reads
/// or writes these (ARCH-SETTINGS).
#[derive(Clone, Debug, PartialEq, Eq, crate::personal::Entity)]
pub struct StoredPersonSetting {
    /// The person the value belongs to.
    #[personal(category = "identity", basis = "contract")]
    pub person: crate::id::UserId,
    /// The key of the setting, as declared in the registry.
    #[not_personal]
    pub key: crate::technical::TechnicalName,
    /// What the person chose: time zone, region, appearance, shortcut keys, start page.
    #[personal(category = "content", basis = "contract")]
    pub value: SettingValue,
    /// When the person last changed it.
    #[personal(category = "activity", basis = "contract")]
    pub updated_at: jiff::Timestamp,
}

/// The items of a list written as text with commas between the items: spaces around an item are
/// dropped, and so are empty items. The one reading of a comma list (the limits of a list
/// setting, the approved domains, the topics of a settings query).
pub fn list_items(value: &str) -> impl Iterator<Item = &str> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
}

/// Every setting of the registry, and in tests the ones that only exist for them.
pub fn registry() -> impl Iterator<Item = &'static SettingDefinition> {
    #[cfg(any(test, feature = "test-settings"))]
    let extra: &[&SettingDefinition] = for_tests::ALL;
    #[cfg(not(any(test, feature = "test-settings")))]
    let extra: &[&SettingDefinition] = &[];
    ALL.iter().copied().chain(extra.iter().copied())
}

/// The registry entry for `key`, if there is one.
#[must_use]
pub fn find(key: &str) -> Option<&'static SettingDefinition> {
    registry().find(|definition| definition.key() == key)
}

#[cfg(test)]
mod tests;
