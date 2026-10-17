//! Reading settings for the notifications: whether a person wants to hear of a kind of event in
//! the app or by e-mail, and the settings of the mail server (ADR-0033 §1, §3).

use bb_core::id::UserId;
use bb_core::notification::{NotificationKind, Via};
use bb_core::settings::{Layers, MAIL_ENABLED};

use crate::auth::{AppError, AuthStore, Ctx, internal};

/// Whether a person in the layers they have wants to hear of `kind` this way. The organization's
/// value is the default of everybody and a locked one counts for everybody.
pub(crate) fn wants(layers: &Layers, kind: NotificationKind, via: Via) -> bool {
    layers.yes(kind.setting(via))
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
        Ok(self.setting_layers_of(None).await?.yes(&MAIL_ENABLED))
    }
}
