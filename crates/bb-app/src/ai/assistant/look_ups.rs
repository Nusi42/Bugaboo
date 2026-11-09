//! How many tools the assistant may run for one question (AI-10, WP-15): the most the organization
//! set, lowered by the number the person chose for themselves. Read through the settings registry,
//! so the browser and the server agree.

use bb_core::ai::LookUps;
use bb_core::id::UserId;
use bb_core::settings::{ASSISTANT_LOOK_UPS, ASSISTANT_LOOK_UPS_MOST};

use crate::auth::{AppError, AuthStore, Ctx};

impl<S: AuthStore> Ctx<'_, S> {
    /// The look-ups one question of the person may use: never more than the most of the
    /// organization, and the person's own number when it is lower.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(super) async fn look_ups_of(&self, person: UserId) -> Result<LookUps, AppError> {
        let layers = self.setting_layers_of(Some(person)).await?;
        let organization =
            LookUps::of_code(&layers.text(&ASSISTANT_LOOK_UPS_MOST)).unwrap_or_default();
        Ok(organization.lowered_by(LookUps::of_code(&layers.text(&ASSISTANT_LOOK_UPS))))
    }
}
