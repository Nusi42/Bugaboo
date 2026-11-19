//! The people a request reads, each once (PRIV-12, PRIV-04): the person with their standing, so a
//! switched-off person is told apart from an active one and a removed one is shown as *Former
//! user* through the one tombstone of `bb-policy`.

use bb_core::ErrorCode;
use bb_core::id::UserId;
use bb_core::person::{KnownPerson, Person};
use bb_policy::FormerUser;

use crate::auth::{AppError, AuthStore, Ctx, internal, locked};

/// The tombstone of a removed person, as `resolve` takes it; none for anybody else (PRIV-04).
pub(crate) fn former_user(known: &KnownPerson) -> Option<FormerUser> {
    known
        .standing
        .as_ref()
        .filter(|standing| standing.removed())
        .and_then(|standing| {
            standing.label.clone().map(|label| FormerUser {
                label,
                held_name: standing.hold_name.clone(),
            })
        })
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The person with this ID and their standing: `resource.not-found` for nobody.
    pub(crate) async fn known_person_or_missing(
        &self,
        person: UserId,
    ) -> Result<KnownPerson, AppError> {
        self.person_with_standing(person)
            .await?
            .ok_or(AppError::Refused(ErrorCode::NotFound))
    }

    /// The person with this ID, looked up once for the request, whatever their standing: a
    /// switched-off or removed person is still the person their records name.
    pub(crate) async fn person_known(&self, id: UserId) -> Result<Option<Person>, AppError> {
        Ok(self
            .person_with_standing(id)
            .await?
            .map(|known| known.person))
    }

    /// The person with this ID when they are active: who may act, be offered or be told. A
    /// switched-off or removed person is nobody here (ADR-0044, decision 5).
    pub(crate) async fn active_person(&self, id: UserId) -> Result<Option<Person>, AppError> {
        Ok(self
            .person_with_standing(id)
            .await?
            .filter(KnownPerson::active)
            .map(|known| known.person))
    }

    /// The person with this ID and their standing, looked up once for the request.
    pub(crate) async fn person_with_standing(
        &self,
        id: UserId,
    ) -> Result<Option<KnownPerson>, AppError> {
        let known = locked(&self.known.people).get(&id).cloned();
        if let Some(known) = known {
            return Ok(known);
        }
        self.preload_people(&[id]).await?;
        Ok(locked(&self.known.people).get(&id).cloned().flatten())
    }

    /// Reads the people with these IDs in one go, so the lookups of each of them one by one
    /// (`person_known`) find them read already.
    pub(crate) async fn preload_people(&self, ids: &[UserId]) -> Result<(), AppError> {
        let wanted: Vec<UserId> = {
            let known = locked(&self.known.people);
            ids.iter()
                .copied()
                .filter(|id| !known.contains_key(id))
                .collect()
        };
        if wanted.is_empty() {
            return Ok(());
        }
        let found = self
            .store()
            .people_of(&wanted)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let mut known = locked(&self.known.people);
        for id in wanted {
            let person = found.iter().find(|known| known.person.id == id).cloned();
            known.insert(id, person);
        }
        Ok(())
    }
}
