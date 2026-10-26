//! Who a saved search is for: its maker, and the people a share reaches (ADR-0034, decisions 2
//! and 4). A share with a group reaches the people in it; a share with a project reaches the people
//! who see it. Being reached never adds a right: what the search finds is the receiver's own.

use bb_core::ErrorCode;
use bb_core::id::{GroupId, ProjectId, SavedSearchId};
use bb_core::saved_search::{Presentation, SavedSearch, SearchColumn, SearchShare, ShareTarget};
use bb_policy::{Evaluation, Permission};

use super::judge::{Groups, Judge};
use super::store::Reach;
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::projects::Leveled;

/// The groups a person is in and the projects they see: the places a share reaches them by.
pub(crate) struct Reaching {
    groups: Vec<GroupId>,
    projects: Vec<ProjectId>,
    /// What the person sees of the tree, kept for the names and rights of the places in a view.
    pub(crate) shown: Leveled,
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
        let shown = self.shown_with_levels_to(actor).await?;
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
    /// exist, is in the trash, is not theirs and not shared with them, or does not check cleanly
    /// for them is one and the same answer, and the same reads (ADR-0034, decision 4).
    pub(crate) async fn open_search(
        &self,
        actor: &Actor,
        id: SavedSearchId,
    ) -> Result<Opened, AppError> {
        let stored = self.stored_search(id).await?;
        let reaching = self.reaching(actor).await?;
        let mut judge = self.judge(actor, &reaching, Groups::Now).await?;
        let mut clean = None;
        match stored {
            Some(found) if self.may_open(&mut judge, actor, &reaching, &found).await? => {
                clean = Some(found);
            }
            Some(_) => {}
            None => {
                // Nothing there: the same reads are done on a text with no names, so the answer
                // does not come sooner than one for a search that is there and does not check.
                self.canonical_text(&mut judge, "", &Presentation::default())
                    .await?;
            }
        }
        drop(judge);
        let stored = clean.ok_or(AppError::Refused(ErrorCode::NotFound))?;
        Ok(Opened { stored, reaching })
    }

    /// Whether the person may open a saved search: it is not in the trash, and it is theirs, or
    /// it reaches them and checks cleanly for them. The one rule that open, copy, change, share,
    /// delete and restore all ask (ADR-0034, decision 4).
    pub(crate) async fn may_open(
        &self,
        judge: &mut Judge<'_>,
        actor: &Actor,
        reaching: &Reaching,
        found: &Stored,
    ) -> Result<bool, AppError> {
        if found.search.deleted_at.is_some() {
            return Ok(false);
        }
        if found.search.maker == actor.person.id {
            return Ok(true);
        }
        Ok(reaching.includes(&found.shares) && self.reads_cleanly(judge, found).await?)
    }

    /// Whether a saved search of somebody else checks cleanly for the person `judge` is for.
    pub(crate) async fn reads_cleanly(
        &self,
        judge: &mut Judge<'_>,
        found: &Stored,
    ) -> Result<bool, AppError> {
        let presentation = Presentation::of(&found.search, &found.columns);
        Ok(self
            .canonical_text(judge, &found.search.text, &presentation)
            .await?
            .is_some())
    }
}
