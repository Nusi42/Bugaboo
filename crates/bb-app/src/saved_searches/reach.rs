//! Who a saved search is for: its maker, and the people a share reaches (ADR-0034, decisions 2
//! and 4). A share with a group reaches the people in it; a share with a project reaches the people
//! who see it. Being reached never adds a right: what the search finds is the receiver's own.

use bb_core::ErrorCode;
use bb_core::id::{GroupId, ProjectId, SavedSearchId};
use bb_core::saved_search::{SavedSearch, SearchColumn, SearchShare, ShareTarget};
use bb_policy::{Evaluation, Permission};

use super::store::Reach;
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::projects::Shown;

/// The groups a person is in and the projects they see: the places a share reaches them by.
pub(crate) struct Reaching {
    groups: Vec<GroupId>,
    projects: Vec<ProjectId>,
    /// What the person sees of the tree, kept for the names and rights of the places in a view.
    pub(crate) shown: Shown,
}

impl Reaching {
    /// The places that reach the person, for the store.
    pub(crate) fn reach(&self) -> Reach<'_> {
        Reach {
            person: self.shown.person(),
            groups: &self.groups,
            projects: &self.projects,
        }
    }

    /// Whether the person is in the group.
    pub(crate) fn is_in(&self, group: GroupId) -> bool {
        self.groups.contains(&group)
    }

    /// Whether one of the shares reaches the person.
    pub(crate) fn includes(&self, shares: &[SearchShare]) -> bool {
        shares.iter().any(|share| match share.target {
            ShareTarget::Group(group) => self.groups.contains(&group),
            ShareTarget::Project(project) => self.projects.contains(&project),
        })
    }
}

/// Whether a person may share a saved search with a group (ADR-0034, decision 2): they are in it
/// and hold *Share searches* for the whole organization, or they hold *Manage groups*.
/// `organization` is what they hold at the organization.
pub(crate) fn may_share_with_group(organization: &Evaluation, in_group: bool) -> bool {
    organization.allows(Permission::ManageGroups)
        || (in_group && organization.allows(Permission::ShareSearches))
}

/// A saved search that is there, with its columns and its shares.
pub(crate) struct Stored {
    pub(crate) search: SavedSearch,
    pub(crate) columns: Vec<SearchColumn>,
    pub(crate) shares: Vec<SearchShare>,
}

/// A saved search the person may open: their own, or one shared with them.
pub(crate) struct Opened {
    pub(crate) stored: Stored,
    pub(crate) reaching: Reaching,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The groups the actor is in and the projects they see.
    pub(crate) async fn reaching(&self, actor: &Actor) -> Result<Reaching, AppError> {
        let groups = self
            .store()
            .groups_of(actor.person.id)
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .map(|group| group.id)
            .collect();
        let shown = self.shown_to(actor).await?;
        let projects = shown.everything().into_iter().map(|p| p.id).collect();
        Ok(Reaching {
            groups,
            projects,
            shown,
        })
    }

    /// The saved search with its columns and shares, in the trash or not, if there is one. The
    /// same reads whether it is there or not.
    pub(crate) async fn stored_search(
        &self,
        id: SavedSearchId,
    ) -> Result<Option<Stored>, AppError> {
        let store = self.store();
        let failed = |error: &S::Error| internal::<S>(error);
        let search = store.saved_search(id).await.map_err(|e| failed(&e))?;
        let columns = store.search_columns(id).await.map_err(|e| failed(&e))?;
        let shares = store.search_shares(id).await.map_err(|e| failed(&e))?;
        Ok(search.map(|search| Stored {
            search,
            columns,
            shares,
        }))
    }

    /// The saved search the actor may open, or `resource.not-found`: a search that does not
    /// exist, is in the trash, or is not theirs and not shared with them is one and the same
    /// answer, and the same reads (ADR-0034, decision 4).
    pub(crate) async fn open_search(
        &self,
        actor: &Actor,
        id: SavedSearchId,
    ) -> Result<Opened, AppError> {
        let stored = self.stored_search(id).await?;
        let reaching = self.reaching(actor).await?;
        let stored = stored
            .filter(|found| found.search.deleted_at.is_none())
            .filter(|found| {
                found.search.maker == actor.person.id || reaching.includes(&found.shares)
            })
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        Ok(Opened { stored, reaching })
    }
}
