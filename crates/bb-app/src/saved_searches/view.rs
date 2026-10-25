//! A saved search as one person sees it: how it is shown, who made it, and, for its maker,
//! whom it is shared with. All of a page is made from a handful of reads, never one for each
//! search (PERF-05).

use std::collections::HashMap;

use bb_core::id::{GroupId, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_core::saved_search::{Presentation, ShareTarget};
use bb_policy::{Evaluation, PeopleContext, Place};

use super::reach::{Reaching, Stored};
use super::types::{SavedSearchView, ShareView};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal, locked};

impl<S: AuthStore> Ctx<'_, S> {
    /// Looks up the people with these IDs in one read, for `person_ref` to find them without a
    /// read each.
    async fn preload_makers(&self, makers: &[UserId]) -> Result<(), AppError> {
        let wanted: Vec<UserId> = {
            let known = locked(&self.known.people);
            let mut wanted: Vec<UserId> = makers
                .iter()
                .copied()
                .filter(|maker| !known.contains_key(maker))
                .collect();
            wanted.sort();
            wanted.dedup();
            wanted
        };
        if wanted.is_empty() {
            return Ok(());
        }
        let found = self
            .store()
            .makers_of(&wanted)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let mut known = locked(&self.known.people);
        for id in wanted {
            let person = found.iter().find(|person| person.id == id).cloned();
            known.insert(id, person);
        }
        Ok(())
    }

    /// One saved search as the actor sees it.
    pub(crate) async fn one_view(
        &self,
        actor: &Actor,
        reaching: &Reaching,
        stored: Stored,
    ) -> Result<SavedSearchView, AppError> {
        self.search_views(actor, reaching, vec![stored])
            .await?
            .pop()
            .ok_or_else(|| AppError::Internal("a view of one saved search is missing".to_owned()))
    }

    /// The saved searches as the actor sees them, in the order given.
    pub(crate) async fn search_views(
        &self,
        actor: &Actor,
        reaching: &Reaching,
        found: Vec<Stored>,
    ) -> Result<Vec<SavedSearchView>, AppError> {
        let makers: Vec<UserId> = found.iter().map(|stored| stored.search.maker).collect();
        self.preload_makers(&makers).await?;
        let own_shares = found.iter().any(|stored| {
            stored.search.maker == actor.person.id
                && stored
                    .shares
                    .iter()
                    .any(|share| matches!(share.target, ShareTarget::Group(_)))
        });
        let group_names: HashMap<GroupId, String> = if own_shares {
            self.store()
                .groups()
                .await
                .map_err(|error| internal::<S>(&error))?
                .into_iter()
                .map(|group| (group.id, group.name))
                .collect()
        } else {
            HashMap::new()
        };
        let organization = self.evaluation_of(actor, Place::Organization).await?;
        let mut views = Vec::with_capacity(found.len());
        for stored in found {
            let mine = stored.search.maker == actor.person.id;
            let viewer = viewer_of(&organization, reaching, &stored);
            let maker = self
                .person_ref(
                    &viewer,
                    PeopleKind::Authors,
                    PeopleContext::Ordinary,
                    stored.search.maker,
                )
                .await?;
            let shared_with = if mine {
                shares_shown(&stored, reaching, &group_names)
            } else {
                Vec::new()
            };
            views.push(SavedSearchView {
                id: stored.search.id,
                name: stored.search.name.clone(),
                text: stored.search.text.clone(),
                presentation: Presentation::of(&stored.search, &stored.columns),
                maker,
                mine,
                shared_with,
                updated_at: stored.search.updated_at,
                version: stored.search.version,
            });
        }
        Ok(views)
    }
}

/// The rights the person is shown people by: where the search reaches them through a project they
/// see, as they hold it there, else for the organization. Decided in memory from the tree that
/// was read.
fn viewer_of(organization: &Evaluation, reaching: &Reaching, stored: &Stored) -> Evaluation {
    stored
        .shares
        .iter()
        .filter_map(|share| match share.target {
            ShareTarget::Project(project) if reaching.shown.sees(project) => Some(project),
            _ => None,
        })
        .find_map(|project| {
            reaching
                .shown
                .evaluations_in(&[project])
                .into_iter()
                .next()
                .map(|(_, held)| held)
        })
        .unwrap_or_else(|| organization.clone())
}

/// Whom the maker shared the search with, each named as the maker may know it.
fn shares_shown(
    stored: &Stored,
    reaching: &Reaching,
    group_names: &HashMap<GroupId, String>,
) -> Vec<ShareView> {
    stored
        .shares
        .iter()
        .filter_map(|share| match share.target {
            ShareTarget::Group(id) => group_names.get(&id).map(|name| ShareView::Group {
                id,
                name: name.clone(),
            }),
            ShareTarget::Project(id) => reaching
                .shown
                .label(Place::Project(id))
                .map(|label| ShareView::Project { id, label }),
        })
        .collect()
}
