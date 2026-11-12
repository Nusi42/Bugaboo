//! What a setting accepts beyond its type: the format of a text, and for a choice of a theme an own
//! theme that the scope may use (ADR-0018, ADR-0026 update M8).

use bb_core::ErrorCode;
use bb_core::settings::{Format, Scope, SettingDefinition, SettingValue};

use crate::auth::{AppError, AuthStore, Ctx};
use crate::themes::ChoiceInForce;

/// Refuses a value the setting does not accept. A text its format refuses has the format's answer
/// (a list of hosts with an address in it, a zone the server does not know), in plain words;
/// anything else is an invalid request.
pub(super) fn refuse_unaccepted(
    definition: &SettingDefinition,
    value: &SettingValue,
) -> Result<(), AppError> {
    if definition.accepts(value) {
        return Ok(());
    }
    let format_refused =
        matches!(value, SettingValue::Text(text) if !definition.format().accepts(text));
    Err(if format_refused {
        definition.format().refusal()
    } else {
        ErrorCode::InvalidRequest
    }
    .into())
}

impl<S: AuthStore> Ctx<'_, S> {
    /// For a choice of a theme that names an own theme: the theme, when the scope may use it, or
    /// else the built-in theme it extended, which counts in its place (ADR-0026, update M8).
    /// `None` for any other value.
    pub(super) async fn theme_in_force(
        &self,
        scope: Scope,
        definition: &SettingDefinition,
        value: &SettingValue,
    ) -> Result<Option<ChoiceInForce>, AppError> {
        let (Format::Themes(_), SettingValue::Text(text)) = (definition.format(), value) else {
            return Ok(None);
        };
        let person = match scope {
            Scope::Person(person) => Some(person),
            Scope::Organization | Scope::Project(_) => None,
        };
        self.theme_choice_in_force(person, text).await
    }
}
