//! The names of releases in a search (ADR-0049, decision 4): a release of a project the person
//! sees, also a retired one, by the one rule of the same name. The releases are read once for the
//! request, and only when a search names one or groups by one.

use bb_core::names::{name_key, same_name};

use super::{Ask, NameCache};
use crate::auth::{AppError, AuthStore, Ctx};
use crate::search::readable::Looked;

impl<S: AuthStore> Ctx<'_, S> {
    /// Whether the name stands for a release the person may know: one of a project they see, also
    /// a retired one, by the one rule of the same name (ADR-0049, decision 4). The releases are read
    /// once for the request, and only when a search names one.
    pub(super) async fn release_named(
        &self,
        looked: &Looked,
        each: &Ask,
        cache: &mut NameCache,
    ) -> Result<bool, AppError> {
        let key = name_key(&each.text);
        if let Some((_, found)) = cache.names.releases.iter().find(|(name, _)| *name == key) {
            return Ok(!found.is_empty());
        }
        let ids: Vec<_> = self
            .releases_known_to(looked)
            .await?
            .iter()
            .filter(|(name, _)| same_name(name, &each.text))
            .map(|(_, id)| *id)
            .collect();
        let found = !ids.is_empty();
        cache.names.releases.push((key, ids));
        Ok(found)
    }

    /// The name of a release the person may know, for the name of a group by release.
    pub(crate) async fn release_name_of(
        &self,
        looked: &Looked,
        id: bb_core::id::ReleaseId,
    ) -> Result<Option<String>, AppError> {
        Ok(self
            .releases_known_to(looked)
            .await?
            .iter()
            .find(|(_, each)| *each == id)
            .map(|(name, _)| name.clone()))
    }

    /// The releases the person may know where the search looks (those of the projects they see,
    /// and the AI may see, also the retired ones), by name: read once for the request, for its
    /// names, the IDs a release field reads and the names of its groups.
    pub(crate) async fn releases_known_to<'l>(
        &self,
        looked: &'l Looked,
    ) -> Result<&'l [(String, bb_core::id::ReleaseId)], AppError> {
        if let Some(known) = looked.releases.get() {
            return Ok(known);
        }
        let read = self
            .releases_seen(&looked.seen)
            .await?
            .into_iter()
            .map(|release| (release.name, release.id))
            .collect();
        Ok(looked.releases.get_or_init(|| read))
    }
}
