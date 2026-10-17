//! Reading where things are offered in one project, for the lists that take a project: the issue
//! types and the groups of labels each say whether the project offers them and whether the person
//! may change that there.

use bb_core::ErrorCode;
use bb_core::id::ProjectId;
use bb_core::offer::{Offer, is_offered};
use bb_policy::{Place, Tree};

use super::change::Subject;
use super::types::OfferedHere;
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};

/// What a list judges every subject on for one project.
pub(crate) struct Here {
    project: ProjectId,
    chain: Vec<ProjectId>,
    offers: Vec<Offer>,
    tree: Tree,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The project a list is asked about, as a person who sees it, with the rows read once; none
    /// when the list is not asked about a project.
    ///
    /// # Errors
    /// `resource.not_found` for a project the person may not see.
    pub(crate) async fn offers_here(
        &self,
        actor: &Actor,
        address: Option<&str>,
    ) -> Result<Option<Here>, AppError> {
        let Some(address) = address else {
            return Ok(None);
        };
        let project = self
            .find_project(address)
            .await?
            .ok_or(AppError::Refused(ErrorCode::NotFound))?
            .project
            .id;
        let held = self.evaluation_of(actor, Place::Project(project)).await?;
        if !held.sees_project() {
            return Err(ErrorCode::NotFound.into());
        }
        Ok(Some(Here {
            project,
            chain: self.chain_of(project).await?,
            offers: self
                .store()
                .offers()
                .await
                .map_err(|error| internal::<S>(&error))?,
            tree: self.project_tree_now().await?,
        }))
    }

    /// Whether the project offers the subject, the row it has of its own, and whether the person
    /// may change it for the project.
    pub(crate) async fn offered_here(
        &self,
        actor: &Actor,
        here: &Here,
        subject: &Subject,
    ) -> Result<OfferedHere, AppError> {
        Ok(OfferedHere {
            offered: is_offered(subject.id, subject.owner, &here.offers, &here.chain),
            own: here
                .offers
                .iter()
                .find(|row| row.subject == subject.id && row.project == Some(here.project))
                .map(|row| row.offered),
            may_change: self
                .may_offer_at(actor, subject, Some(here.project), &here.tree)
                .await?,
        })
    }
}
