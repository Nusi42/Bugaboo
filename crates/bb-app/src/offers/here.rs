//! Reading where things are offered in one project, for the lists that take a project: the issue
//! types and the groups of labels each say whether the project offers them and whether the person
//! may change that there. Every list shows the same beside each subject (`Standing`), whether it
//! is a type or a group.

use bb_core::ErrorCode;
use bb_core::id::ProjectId;
use bb_core::offer::{Offer, is_offered, reaches};
use bb_policy::Place;

use super::subject::Subject;
use super::types::OfferedHere;
use crate::access::ProjectLabel;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::projects::Shown;

/// What a list judges every subject on for one project.
pub(crate) struct Here {
    project: ProjectId,
    chain: Vec<ProjectId>,
    offers: Vec<Offer>,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The project a list is asked about, as a person who sees it, with the rows read once; none
    /// when the list is not asked about a project.
    ///
    /// # Errors
    /// `resource.not_found` for a project the person may not see.
    pub(crate) async fn offers_here(
        &self,
        shown: &Shown,
        address: Option<&str>,
    ) -> Result<Option<Here>, AppError> {
        let (Some(found), chain) = self.place_of_request(address).await? else {
            return Ok(None);
        };
        if !shown.knows(found.id) {
            return Err(ErrorCode::NotFound.into());
        }
        Ok(Some(Here {
            project: found.id,
            chain,
            offers: self
                .store()
                .offers()
                .await
                .map_err(|error| internal::<S>(&error))?,
        }))
    }
}

impl Here {
    /// Whether a subject created in `owner` can reach the project the list is about (the one rule,
    /// `bb_core::offer::reaches`): a list about a project leaves out the others.
    pub(crate) fn reached_by(&self, owner: Option<ProjectId>) -> bool {
        reaches(owner, &self.chain)
    }

    /// Whether the project offers the subject, the row it has of its own, and whether the person
    /// may change it for the project.
    fn offered_here(&self, shown: &Shown, subject: &Subject) -> OfferedHere {
        OfferedHere {
            offered: is_offered(subject.id, subject.owner, &self.offers, &self.chain),
            own: self
                .offers
                .iter()
                .find(|row| row.subject == subject.id && row.project == Some(self.project))
                .map(|row| row.offered),
            may_change: subject.may_offer_at(shown, Place::Project(self.project)),
        }
    }
}

/// What a list says beside a subject, in the words of the person who reads it.
pub(crate) struct Standing {
    /// Where it was created; none for the whole organization.
    pub(crate) created_in: Option<ProjectLabel>,
    /// Whether the person looks after it.
    pub(crate) may_manage: bool,
    /// What the project the list is about does with it.
    pub(crate) here: Option<OfferedHere>,
}

impl Standing {
    /// What the person sees of the subject, and of the project the list is about, if it is.
    pub(crate) fn of(shown: &Shown, here: Option<&Here>, subject: &Subject) -> Self {
        let place = subject.place();
        Self {
            created_in: shown.label(place),
            may_manage: shown.holds(place, subject.permission),
            here: here.map(|here| here.offered_here(shown, subject)),
        }
    }
}
