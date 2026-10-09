//! The structured answer to a stale version (`edit.version_changed`, ADR-0019 amendment M3): who
//! changed the record, when, which fields, and the values the reader may see now.
//!
//! One place makes it for every route. A use case that finds that the version a person sent is not
//! the version the record has now answers [`AppError::Stale`] (it names the record, where it sits
//! and who asks, and nothing else); `AuthApi` turns that into a [`Conflict`] before it reaches
//! the HTTP layer, and `bb-api` writes the `Conflict` into the problem answer. So a route carries
//! the structured answer without code of its own, and the browser writes the sentence ("Ben
//! changed Priority to Critical 2 minutes ago.") from it, with the reader's field mask and *Who sees
//! people*: an error message has no placeholder (architecture §7, *Errors*).
//!
//! Who changed it comes from the last domain event about the record (ADR-0007), and is shown as the
//! kind *Last changed by* says for the reader (a name, a role, or *Someone*). A value is left out
//! when the reader's role hides the field.

use std::collections::BTreeMap;

use bb_core::ErrorCode;
use bb_core::event::{Actor as EventActor, DomainEvent, EntityRef, RecordKind};
use bb_core::fields::Field;
use bb_core::id::{GroupId, ProjectId, RoleId, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_policy::{Evaluation, PeopleContext, PersonRef, Place};
use jiff::Timestamp;
use serde::Serialize;
use serde_json::Value;

use crate::auth::{AppError, AuthStore, Ctx, internal};

/// A record that a change found at another version than the one the person looked at. It holds
/// only what is needed to find out what happened; `AuthApi` turns it into a [`Conflict`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stale {
    /// The record.
    pub record: EntityRef,
    /// Where it sits: its people and fields are shown as the reader may see them there.
    pub place: Place,
    /// The person who asked.
    pub viewer: UserId,
}

/// What changed a record since a person looked at it, as that person may know it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Conflict {
    /// The version the record has now: what *Use mine* sends again.
    pub version: Option<i64>,
    /// Who changed it last, as the reader may see them.
    pub changed_by: Option<PersonRef>,
    /// When.
    pub changed_at: Option<Timestamp>,
    /// The fields that change, by name, without the ones the reader may not see.
    pub fields: Vec<String>,
    /// The values those fields have now.
    pub current: BTreeMap<String, Value>,
}

impl AppError {
    /// A stale version of `record`, found by a change of `viewer` at `place`.
    #[must_use]
    pub fn stale(record: EntityRef, place: Place, viewer: UserId) -> Self {
        Self::Stale(Box::new(Stale {
            record,
            place,
            viewer,
        }))
    }

    /// The same error, with the record it is about when it is a plain version conflict: the
    /// place for a change that only learns of the conflict inside its transaction.
    #[must_use]
    pub fn about(self, record: &EntityRef, place: Place, viewer: UserId) -> Self {
        match self {
            Self::Refused(ErrorCode::VersionChanged) => Self::stale(record.clone(), place, viewer),
            other => other,
        }
    }
}

/// Whether the reader's mask lets them see the field with this name. A name that is no field of the
/// catalogue (the name of a project, a setting) is not hidden by a field level.
fn visible(viewer: &Evaluation, name: &str) -> bool {
    Field::from_code(name).is_none_or(|field| viewer.field_mask().can_see(field))
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The version the record has now, if it is still there and the person may know about it.
    async fn version_now(&self, stale: &Stale) -> Result<Option<i64>, AppError> {
        let record = &stale.record;
        let ours = |kind: RecordKind| record.kind == kind.into();
        let failed = |error| internal::<S>(&error);
        if ours(RecordKind::Project) {
            let found = match ProjectId::from_bytes(record.id.to_bytes()) {
                Ok(id) => self.store().project(id).await.map_err(failed)?,
                Err(_) => None,
            };
            return Ok(found.map(|project| project.version));
        }
        if ours(RecordKind::Group) {
            let found = match GroupId::from_bytes(record.id.to_bytes()) {
                Ok(id) => self.store().group(id).await.map_err(failed)?,
                Err(_) => None,
            };
            return Ok(found.map(|group| group.version));
        }
        if ours(RecordKind::Role) {
            let found = match RoleId::from_bytes(record.id.to_bytes()) {
                Ok(id) => self.store().role(id).await.map_err(failed)?,
                Err(_) => None,
            };
            return Ok(found.map(|found| found.role.version));
        }
        Ok(self
            .services
            .records
            .look_up(stale.viewer, record)
            .await?
            .map(|facts| facts.version))
    }

    /// Makes the conflict out of a stale record. If anything fails on the way, the person still gets
    /// the plain `edit.version_changed`: a conflict is never turned into another error.
    pub(crate) async fn conflict_of(&self, stale: &Stale) -> AppError {
        match self.try_conflict(stale).await {
            Ok(conflict) => AppError::Conflict(Box::new(conflict)),
            Err(_) => AppError::Refused(ErrorCode::VersionChanged),
        }
    }

    async fn try_conflict(&self, stale: &Stale) -> Result<Conflict, AppError> {
        let viewer = self.evaluate(stale.viewer, stale.place).await?;
        let context = match stale.place {
            Place::Organization => PeopleContext::Management,
            Place::Project(_) => PeopleContext::Ordinary,
        };
        let last = self
            .store()
            .last_change_of(&stale.record)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let changed_by = match last.as_ref().map(|event| event.actor) {
            Some(EventActor::Person(person)) => Some(
                self.person_ref(&viewer, PeopleKind::LastChangedBy, context, person)
                    .await?,
            ),
            Some(EventActor::System) => Some(PersonRef::Someone),
            None => None,
        };
        let (fields, current) = last
            .as_ref()
            .map_or_else(Default::default, |event| shown_changes(&viewer, event));
        Ok(Conflict {
            version: self.version_now(stale).await?,
            changed_by,
            changed_at: last.map(|event| event.time),
            fields,
            current,
        })
    }
}

/// The names of the fields an event changed and the values they have after it, for the fields the
/// reader may see.
fn shown_changes(
    viewer: &Evaluation,
    event: &DomainEvent,
) -> (Vec<String>, BTreeMap<String, Value>) {
    let mut fields = Vec::new();
    let mut current = BTreeMap::new();
    for change in event
        .changes
        .iter()
        .filter(|change| visible(viewer, &change.field))
    {
        fields.push(change.field.clone());
        if let Some(value) = &change.after {
            current.insert(change.field.clone(), value.clone());
        }
    }
    (fields, current)
}
