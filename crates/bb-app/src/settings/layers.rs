//! The stored values a use case reads settings from: the organization's and, for a person, theirs.

use std::collections::HashMap;

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

    /// The stored values of many people at once, each with the organization's: the organization's
    /// are read once (or taken from `organization`, when the caller has them), the people's own in
    /// one statement for all of them. The one place that puts the layers of a person together
    /// besides `setting_layers_of`, which reads them for one person. Returns the organization's
    /// layers too, for the next call.
    pub(crate) async fn setting_layers_of_people(
        &self,
        people: &[UserId],
        organization: Option<Layers>,
    ) -> Result<(Layers, HashMap<UserId, Layers>), AppError> {
        let base = match organization {
            Some(base) => base,
            None => self.setting_layers_of(None).await?,
        };
        let mut own: HashMap<UserId, _> = self
            .store()
            .person_settings(people.to_vec())
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .collect();
        let layers = people
            .iter()
            .map(|person| {
                let mut layers = base.clone();
                layers.person = own.remove(person).unwrap_or_default();
                (*person, layers)
            })
            .collect();
        Ok((base, layers))
    }

    /// Whether the organization has switched e-mail on (PRIV-08).
    pub(crate) async fn mail_is_on(&self) -> Result<bool, AppError> {
        Ok(self.setting_layers_of(None).await?.yes(&MAIL_ENABLED))
    }
}
