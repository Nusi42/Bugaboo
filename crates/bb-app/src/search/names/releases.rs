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
        self.load_releases(looked, cache).await?;
        let ids: Vec<_> = cache
            .release_names
            .iter()
            .flatten()
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
        cache: &mut NameCache,
    ) -> Result<Option<String>, AppError> {
        self.load_releases(looked, cache).await?;
        Ok(cache
            .release_names
            .iter()
            .flatten()
            .find(|(_, each)| *each == id)
            .map(|(name, _)| name.clone()))
    }

    /// Reads the releases of the projects the person sees into the cache, once.
    async fn load_releases(&self, looked: &Looked, cache: &mut NameCache) -> Result<(), AppError> {
        if cache.release_names.is_none() {
            cache.release_names = Some(
                self.releases_seen(&looked.seen)
                    .await?
                    .into_iter()
                    .map(|release| (release.name, release.id))
                    .collect(),
            );
        }
        Ok(())
    }
}
