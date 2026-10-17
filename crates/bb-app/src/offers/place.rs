//! Asking "is this offered here" in the places that need the answer: before a change (on what the
//! request read) and again inside the change (on what is stored, after the shared lock of the
//! workflows, so a change of the offers cannot happen between the look and the write).

use bb_core::ErrorCode;
use bb_core::id::{IssueTypeId, ProjectId};
use bb_core::issue::IssueType;
use bb_core::offer::{Offer, OfferSubject, is_offered};
use bb_core::tags::LabelGroup;
use bb_policy::{Place, Tree};

use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::projects::Why;

impl<S: AuthStore> Ctx<'_, S> {
    /// Refuses an issue type that takes no new issue in the project: it is retired
    /// (`issue.type_retired`), or the project does not offer it (`issue.type_not_offered`). An issue
    /// keeps a type that is retired or not offered; only a new issue and a change of type are judged.
    pub(crate) async fn ensure_type_takes_issues(
        &self,
        found: &IssueType,
        project: ProjectId,
    ) -> Result<(), AppError> {
        if found.retired_at.is_some() {
            return Err(ErrorCode::IssueTypeRetired.into());
        }
        if !self
            .offered_in(OfferSubject::IssueType(found.id), found.project, project)
            .await?
        {
            return Err(ErrorCode::IssueTypeNotOffered.into());
        }
        Ok(())
    }

    /// Whether a project offers an issue type or a group of labels, on what is stored now.
    pub(crate) async fn offered_in(
        &self,
        subject: OfferSubject,
        owner: Option<ProjectId>,
        project: ProjectId,
    ) -> Result<bool, AppError> {
        let offers = self
            .store()
            .offers()
            .await
            .map_err(|error| internal::<S>(&error))?;
        let chain = self.chain_of(project).await?;
        Ok(is_offered(subject, owner, &offers, &chain))
    }
}

/// The place that a subject made in this project is looked after at.
#[must_use]
pub(crate) fn place_of(owner: Option<ProjectId>) -> Place {
    owner.map_or(Place::Organization, Place::Project)
}

/// Inside the transaction that writes an issue of this type in this project: takes the lock of the
/// workflows shared and checks that the type still takes new issues. If it was retired meanwhile,
/// the write is stopped with `issue.type_retired`, and if a place was changed so that the project
/// does not offer it any more, with `issue.type_not_offered`.
///
/// # Errors
/// The error of the store, or the stop of the change.
pub(crate) async fn ensure_type_takes_issues_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    project: ProjectId,
    issue_type: IssueTypeId,
    stopped: &Why,
) -> Result<(), S::Error> {
    S::lock_workflows_shared(transaction).await?;
    let Some(found) = S::issue_types_in(transaction)
        .await?
        .into_iter()
        .find(|each| each.id == issue_type && each.retired_at.is_none())
    else {
        return Err(stopped.stop::<S>(ErrorCode::IssueTypeRetired));
    };
    let tree = Tree::of(&S::project_tree_in(transaction).await?);
    let chain = tree.chain(project).unwrap_or_default();
    let offers = S::offers_in(transaction).await?;
    if is_offered(
        OfferSubject::IssueType(issue_type),
        found.project,
        &offers,
        &chain,
    ) {
        Ok(())
    } else {
        Err(stopped.stop::<S>(ErrorCode::IssueTypeNotOffered))
    }
}

/// What decides whether a project offers a group of labels, read once: the rows of the offers and
/// the projects from the top down to the project.
pub(crate) struct GroupOffers {
    offers: Vec<Offer>,
    chain: Vec<ProjectId>,
}

impl GroupOffers {
    /// The offers and the projects from the top down to the project they are asked for.
    pub(crate) const fn new(offers: Vec<Offer>, chain: Vec<ProjectId>) -> Self {
        Self { offers, chain }
    }

    /// Whether the project offers the group.
    pub(crate) fn offers(&self, group: &LabelGroup) -> bool {
        is_offered(
            OfferSubject::LabelGroup(group.id),
            group.project,
            &self.offers,
            &self.chain,
        )
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The offers of groups of labels in a project, on what is stored now.
    pub(crate) async fn group_offers(&self, project: ProjectId) -> Result<GroupOffers, AppError> {
        Ok(GroupOffers::new(
            self.store()
                .offers()
                .await
                .map_err(|error| internal::<S>(&error))?,
            self.chain_of(project).await?,
        ))
    }
}

/// [`GroupOffers`] inside the transaction that puts a label on an issue of this project: takes the
/// lock of the workflows shared, so the offers cannot change between the look and the write.
///
/// # Errors
/// The error of the store.
pub(crate) async fn group_offers_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    project: ProjectId,
) -> Result<GroupOffers, S::Error> {
    S::lock_workflows_shared(transaction).await?;
    let tree = Tree::of(&S::project_tree_in(transaction).await?);
    Ok(GroupOffers::new(
        S::offers_in(transaction).await?,
        tree.chain(project).unwrap_or_default(),
    ))
}
