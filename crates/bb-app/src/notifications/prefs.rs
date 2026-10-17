//! Reading settings for the notifications: whether a person wants to hear of a kind of event in
//! the app or by e-mail, and the settings of the mail server (ADR-0033 §1, §3).

use bb_core::id::UserId;
use bb_core::notification::{NotificationKind, Via};
use bb_core::settings::{Bool, Integer, Layers, MAIL_ENABLED, Setting, SettingKind, Text};

use crate::auth::{AppError, AuthStore, Ctx, internal};

/// The value that counts for `setting` in `layers`: the stored one, or the default when a stored
/// value is of another type (a value written by hand; the registry accepts nothing else).
pub(crate) fn value_of<K: SettingKind>(layers: &Layers, setting: &Setting<K>) -> Option<K::Value> {
    let resolved = layers.resolve(setting.definition()).value;
    setting
        .resolve(Some(&resolved))
        .or_else(|_| setting.default_value())
        .ok()
}

/// The yes or no of a setting in `layers`; no when the registry holds no usable value.
pub(crate) fn yes(layers: &Layers, setting: &Setting<Bool>) -> bool {
    value_of(layers, setting).unwrap_or(false)
}

/// The number of a setting in `layers`, kept inside its bounds.
pub(crate) fn number(layers: &Layers, setting: &Setting<Integer>) -> Option<i64> {
    value_of(layers, setting).map(|value| setting.definition().clamp(value))
}

/// The text of a setting in `layers`.
pub(crate) fn text(layers: &Layers, setting: &Setting<Text>) -> Option<String> {
    value_of(layers, setting)
}

/// Whether a person in the layers they have wants to hear of `kind` this way. The organization's
/// value is the default of everybody and a locked one counts for everybody.
pub(crate) fn wants(layers: &Layers, kind: NotificationKind, via: Via) -> bool {
    yes(layers, kind.setting(via))
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The stored values of the organization and, with `person`, of that person.
    pub(crate) async fn setting_layers_of(
        &self,
        person: Option<UserId>,
    ) -> Result<Layers, AppError> {
        self.store()
            .setting_layers(None, person)
            .await
            .map_err(|error| internal::<S>(&error))
    }

    /// Whether the organization has switched e-mail on (PRIV-08).
    pub(crate) async fn mail_is_on(&self) -> Result<bool, AppError> {
        Ok(yes(&self.setting_layers_of(None).await?, &MAIL_ENABLED))
    }
}
