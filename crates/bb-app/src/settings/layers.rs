//! The stored values a use case reads settings from: the organization's and, for a person, theirs.

use bb_core::id::UserId;
use bb_core::settings::{Layers, MAIL_ENABLED};

use crate::auth::{AppError, AuthStore, Ctx, internal};

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
