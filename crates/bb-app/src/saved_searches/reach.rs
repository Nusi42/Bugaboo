//! Who a saved search is for: its maker, and the people a share reaches (ADR-0034, decisions 2
//! and 4). A share with a group reaches the people in it; a share with a project reaches the people
//! who see it. Being reached never adds a right: what the search finds is the receiver's own.

use bb_core::ErrorCode;
use bb_core::id::{GroupId, ProjectId, SavedSearchId, UserId};
use bb_core::saved_search::{Presentation, SavedSearch, SearchColumn, SearchShare, ShareTarget};
use bb_policy::{Evaluation, Permission};

use super::judge::Judge;
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

    /// Whether one of the places a record is shared with reaches the person.
    pub(crate) fn includes(&self, targets: impl IntoIterator<Item = ShareTarget>) -> bool {
        targets.into_iter().any(|target| match target {
            ShareTarget::Group(group) => self.groups.contains(&group),
            ShareTarget::Project(project) => self.projects.contains(&project),
        })
    }

    /// How a record that one person makes and shares reaches this person: not at all when it is in
    /// the trash or neither theirs nor shared with a place of theirs, as its maker, or by a share.
    /// The first half of the one rule of who may open a saved search or a dashboard (ADR-0034,
    /// decision 4; ADR-0050, decision 2); a saved search reached by a share must also check cleanly.
    pub(crate) fn reached(
        &self,
        deleted: bool,
        maker: UserId,
        targets: impl IntoIterator<Item = ShareTarget>,
    ) -> Reached {
        if deleted {
            Reached::Not
        } else if maker == self.shown.person() {
            Reached::AsMaker
        } else if self.includes(targets) {
            Reached::ByShare
        } else {
            Reached::Not
        }
    }
}

/// How a shared record reaches a person ([`Reaching::reached`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reached {
    /// It is not there for them.
    Not,
    /// They made it.
    AsMaker,
    /// It is shared with a group of theirs or a project they see.
    ByShare,
}

/// What a person may share with others: saved searches or dashboards, each by a permission of its
/// own (ADR-0034, decision 2; ADR-0050, decision 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shareable {
    /// Saved searches, with *Share searches*.
    Searches,
    /// Dashboards, with *Share dashboards*.
    Dashboards,
}

impl Shareable {
    /// The permission that shares it.
    pub(crate) const fn permission(self) -> Permission {
        match self {
            Self::Searches => Permission::ShareSearches,
            Self::Dashboards => Permission::ShareDashboards,
        }
    }
}

/// Whether a person may share a saved search or a dashboard with a group (ADR-0034, decision 2):
/// they are in it and hold the permission that shares it for the whole organization, or they hold
/// *Manage groups*. `organization` is what they hold at the organization.
pub(crate) fn may_share_with_group(
    organization: &Evaluation,
    in_group: bool,
    what: Shareable,
) -> bool {
    organization.allows(Permission::ManageGroups)
        || (in_group && organization.allows(what.permission()))
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
    /// for them is one and the same answer; a search that is not there and one that does not reach the
    /// person run the same reads (ADR-0034, decision 4).
    pub(crate) async fn open_search(
        &self,
        actor: &Actor,
        id: SavedSearchId,
    ) -> Result<Opened, AppError> {
        let stored = self.stored_search(id).await?;
        let reaching = self.reaching(actor).await?;
        let mut judge = self.judge(actor, &reaching).await?;
        let clean = match stored {
            Some(found) if self.may_open(&mut judge, &reaching, &found).await? => Some(found),
            _ => None,
        };
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
        reaching: &Reaching,
        found: &Stored,
    ) -> Result<bool, AppError> {
        let targets = found.shares.iter().map(|share| share.target);
        match reaching.reached(
            found.search.deleted_at.is_some(),
            found.search.maker,
            targets,
        ) {
            Reached::Not => Ok(false),
            Reached::AsMaker => Ok(true),
            Reached::ByShare => self.reads_cleanly(judge, found).await,
        }
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
