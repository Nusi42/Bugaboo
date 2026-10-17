//! What a subject of the offers is, as far as the offers are concerned: an issue type or a group of
//! labels, built from the record in hand and never read again.

use bb_core::event::{EntityRef, RecordKind};
use bb_core::id::{EntityId, ProjectId};
use bb_core::issue::IssueType;
use bb_core::offer::{OfferSubject, reaches};
use bb_core::tags::LabelGroup;
use bb_policy::{Permission, Place};

use crate::auth::{AppError, AuthStore, Ctx, entity};
use crate::projects::Shown;

/// What a subject is, as far as the offers are concerned.
pub(crate) struct Subject {
    pub(crate) id: OfferSubject,
    /// The project it was created in; none for the whole organization.
    pub(crate) owner: Option<ProjectId>,
    /// The version it is at.
    pub(crate) version: i64,
    /// What a person needs to hold to look after it.
    pub(crate) permission: Permission,
    /// The kind of record its events are about.
    pub(crate) record: RecordKind,
}

impl Subject {
    /// An issue type.
    pub(crate) const fn of_type(found: &IssueType) -> Self {
        Self {
            id: OfferSubject::IssueType(found.id),
            owner: found.project,
            version: found.version,
            permission: Permission::ManageIssueTypes,
            record: RecordKind::IssueType,
        }
    }

    /// A group of labels.
    pub(crate) const fn of_group(found: &LabelGroup) -> Self {
        Self {
            id: OfferSubject::LabelGroup(found.id),
            owner: found.project,
            version: found.version,
            permission: Permission::ManageLabels,
            record: RecordKind::LabelGroup,
        }
    }

    /// The record of its events.
    pub(crate) fn entity(&self) -> EntityRef {
        let id: EntityId = match self.id {
            OfferSubject::IssueType(id) => id.into(),
            OfferSubject::LabelGroup(id) => id.into(),
        };
        entity(self.record, id)
    }

    /// The place its owner is judged at.
    pub(crate) fn place(&self) -> Place {
        Place::from(self.owner)
    }

    /// Whether a row at this place may be set for the subject by the person: the subject reaches
    /// the place, and the person looks after such subjects there.
    pub(crate) fn may_offer_at(&self, shown: &Shown, place: Place) -> bool {
        reaches(self.owner, &shown.chain(place)) && shown.holds(place, self.permission)
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The subject, or `resource.not_found` for one that is not there or that the person may not
    /// know (`known.rs`).
    pub(crate) async fn offer_subject(
        &self,
        actor: &crate::access::Actor,
        id: OfferSubject,
    ) -> Result<Subject, AppError> {
        Ok(match id {
            OfferSubject::IssueType(id) => Subject::of_type(&self.known_type(actor, id).await?),
            OfferSubject::LabelGroup(id) => Subject::of_group(&self.known_group(actor, id).await?),
        })
    }

    /// The place a project address names: the project, or the whole organization for none.
    pub(crate) async fn offer_place(&self, project: Option<&str>) -> Result<Place, AppError> {
        let (found, _) = self.place_of_request(project).await?;
        Ok(Place::from(found.map(|found| found.id)))
    }
}
