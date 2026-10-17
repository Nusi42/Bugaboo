//! Asking "is this offered here" in the places that need the answer: before a change (on what the
//! request read) and again inside the change (on what is stored, after the shared lock of the
//! workflows, so a change of the offers cannot happen between the look and the write).
//!
//! The rules are in `bb_core::offer` (`type_takes_issues`, `is_offered`); this file only reads what
//! they are asked about.

use bb_core::ErrorCode;
use bb_core::id::{IssueTypeId, ProjectId};
use bb_core::issue::IssueType;
use bb_core::offer::{Offer, OfferSubject, TypeRefusal, is_offered, type_takes_issues};
use bb_core::tags::LabelGroup;

use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::projects::Why;

/// The error code for an issue type that takes no new issue.
const fn refusal_code(refusal: TypeRefusal) -> ErrorCode {
    match refusal {
        TypeRefusal::Retired => ErrorCode::IssueTypeRetired,
        TypeRefusal::NotOffered => ErrorCode::IssueTypeNotOffered,
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Refuses an issue type that takes no new issue in the project whose chain from the top down
    /// is given: it is retired (`issue.type_retired`), or the project does not offer it
    /// (`issue.type_not_offered`). An issue keeps a type that is retired or not offered; only a new
    /// issue and a change of type are judged.
    pub(crate) async fn ensure_type_takes_issues(
        &self,
        found: &IssueType,
        chain: &[ProjectId],
    ) -> Result<(), AppError> {
        let offers = self
            .store()
            .offers()
            .await
            .map_err(|error| internal::<S>(&error))?;
        type_takes_issues(found, &offers, chain).map_err(|refusal| refusal_code(refusal).into())
    }
}

/// Inside the transaction that writes an issue (or a label on one) in a project: takes the lock of
/// the workflows shared, before any lock on the codes of a project (the one order of the locks,
/// see `LockClass`), and reads the projects from the top down to the project, which the offers and
/// the workflow are judged on.
///
/// # Errors
/// The error of the store.
pub(crate) async fn lock_chain_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    project: ProjectId,
) -> Result<Vec<ProjectId>, S::Error> {
    S::lock_workflows_shared(transaction).await?;
    Ok(crate::projects::tree_in::<S>(transaction)
        .await?
        .chain(project)
        .unwrap_or_default())
}

/// Inside the transaction that writes an issue of this type in the project whose chain is given
/// (after [`lock_chain_in`]): checks that the type still takes new issues. If it was retired
/// meanwhile, the write is stopped with `issue.type_retired`, and if a place was changed so that
/// the project does not offer it any more, with `issue.type_not_offered`.
///
/// # Errors
/// The error of the store, or the stop of the change.
pub(crate) async fn ensure_type_takes_issues_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    chain: &[ProjectId],
    issue_type: IssueTypeId,
    stopped: &Why,
) -> Result<(), S::Error> {
    let Some(found) = S::issue_types_in(transaction)
        .await?
        .into_iter()
        .find(|each| each.id == issue_type)
    else {
        return Err(stopped.stop::<S>(ErrorCode::IssueTypeRetired));
    };
    let offers = S::offers_in(transaction).await?;
    type_takes_issues(&found, &offers, chain)
        .map_err(|refusal| stopped.stop::<S>(refusal_code(refusal)))
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
    let chain = lock_chain_in::<S>(transaction, project).await?;
    Ok(GroupOffers::new(S::offers_in(transaction).await?, chain))
}
