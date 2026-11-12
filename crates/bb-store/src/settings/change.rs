//! What a change of a setting is, and its check against the registry before it is stored.

use bb_core::id::{ProjectId, UserId};
use bb_core::settings::{Scope, SettingValue};
use bb_core::technical::TechnicalName;
use jiff::Timestamp;

use crate::error::SettingProblem;

/// A value to store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingChange {
    /// The key, as declared in the registry.
    pub key: String,
    /// Where the value is set: the organization, a project or a person.
    pub scope: Scope,
    /// The value.
    pub value: SettingValue,
    /// Whether lower levels may not override it (only organization values of lockable settings).
    pub locked: bool,
}

/// A stored value as it was read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredValue {
    /// The value.
    pub value: SettingValue,
    /// Whether it is locked (only organization values are).
    pub locked: bool,
    /// When it was last changed.
    pub updated_at: Timestamp,
}

/// Whose settings are being read: lower levels are only looked at when given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Scopes {
    /// The project, if the value is read in the context of one; the projects above it count too.
    pub project: Option<ProjectId>,
    /// The person, if the value is read for one.
    pub person: Option<UserId>,
}

/// Checks a change against the registry.
pub(super) fn validate(change: &SettingChange) -> Result<TechnicalName, SettingProblem> {
    let definition = bb_core::settings::find(&change.key).ok_or(SettingProblem::UnknownKey)?;
    if !definition.levels().contains(&change.scope.level()) {
        return Err(SettingProblem::LevelNotAllowed);
    }
    if !definition.has_type_of(&change.value) {
        return Err(SettingProblem::WrongType);
    }
    if !definition.accepts(&change.value) {
        return Err(SettingProblem::OutOfBounds);
    }
    if change.locked && !(definition.lockable() && change.scope == Scope::Organization) {
        return Err(SettingProblem::NotLockable);
    }
    TechnicalName::new(definition.key()).map_err(|_| SettingProblem::UnknownKey)
}

/// The key of a setting looked up for reading: it must be in the registry.
pub(super) fn known(key: &str) -> Result<TechnicalName, SettingProblem> {
    let definition = bb_core::settings::find(key).ok_or(SettingProblem::UnknownKey)?;
    TechnicalName::new(definition.key()).map_err(|_| SettingProblem::UnknownKey)
}
