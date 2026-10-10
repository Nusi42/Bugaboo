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
use bb_core::event::{Actor as EventActor, DomainEvent, EntityKind, EntityRef, RecordKind};
use bb_core::fields::Field;
use bb_core::id::{GroupId, ProjectId, RoleId, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_policy::{Evaluation, PeopleContext, PersonRef, Place};
use jiff::Timestamp;
use serde::Serialize;
use serde_json::Value;

use super::records::Shown;
use crate::auth::{AppError, AuthStore, Ctx, fixed_entity, internal};
use bb_core::settings::SIGN_UP_POLICY;

use crate::member;

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

/// How the change of a field of a record that `bb-app` keeps itself is shown: only what is no
/// personal data and no hidden project. Every other field stays out.
fn shown_by_us(kind: &EntityKind, field: &str) -> Shown {
    let plain: &[&str] = if *kind == EntityKind::from(RecordKind::Project) {
        &[member::project::NAME, member::project::SHORT_NAME]
    } else if *kind == EntityKind::from(RecordKind::Group) {
        &[member::group::NAME]
    } else if *kind == EntityKind::from(RecordKind::Role) {
        &[
            member::role::NAME,
            member::role::PERMISSIONS,
            member::role::TWO_STEP,
        ]
    } else if *kind == EntityKind::from(RecordKind::Setting) {
        &[
            member::sign_up_policy::ENABLED,
            member::sign_up_policy::DOMAINS,
        ]
    } else {
        &[]
    };
    if plain.contains(&field) {
        Shown::Plain
    } else {
        Shown::Hidden
    }
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
        if ours(RecordKind::Setting) {
            // The one setting record with a version: the sign-up policy.
            let policy = fixed_entity(RecordKind::Setting, SIGN_UP_POLICY)?;
            if *record == policy {
                return Ok(Some(
                    self.store()
                        .sign_up_policy_version()
                        .await
                        .map_err(failed)?,
                ));
            }
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
        // The newest event that counted the version up: events about a lock, a member of a group
        // or a level change the log of the record but not what the reader looked at.
        let last = self
            .store()
            .latest_version_event_about(&stale.record)
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
        let (fields, current) = match &last {
            Some(event) => {
                self.shown_changes(&viewer, context, &stale.record, event)
                    .await?
            }
            None => Default::default(),
        };
        Ok(Conflict {
            version: self.version_now(stale).await?,
            changed_by,
            changed_at: last.map(|event| event.time),
            fields,
            current,
        })
    }

    /// The names of the fields an event changed and the values they have after it, as the reader
    /// may see them: what the kind of the record shows, left out where the role of the reader
    /// hides a field of the catalogue, and a person as a `PersonRef`.
    async fn shown_changes(
        &self,
        viewer: &Evaluation,
        context: PeopleContext,
        record: &EntityRef,
        event: &DomainEvent,
    ) -> Result<(Vec<String>, BTreeMap<String, Value>), AppError> {
        let mut fields = Vec::new();
        let mut current = BTreeMap::new();
        for change in event
            .changes
            .iter()
            .filter(|change| change.field != bb_core::version::FIELD)
        {
            let shown = match shown_by_us(&record.kind, &change.field) {
                Shown::Hidden => self.services.records.shows(record, &change.field),
                known => known,
            };
            let mask_hides = Field::from_code(&change.field)
                .is_some_and(|field| !viewer.field_mask().can_see(field));
            if shown == Shown::Hidden || mask_hides {
                continue;
            }
            fields.push(change.field.clone());
            let value = match (&change.after, shown) {
                (Some(value), Shown::Plain) => Some(value.clone()),
                (Some(Value::String(id)), Shown::Person(kind)) => match id.parse::<UserId>() {
                    Ok(person) => {
                        let seen = self.person_ref(viewer, kind, context, person).await?;
                        Some(
                            serde_json::to_value(seen)
                                .map_err(|error| AppError::Internal(error.to_string()))?,
                        )
                    }
                    Err(_) => None,
                },
                (Some(Value::Null), Shown::Person(_)) => Some(Value::Null),
                _ => None,
            };
            if let Some(value) = value {
                current.insert(change.field.clone(), value);
            }
        }
        Ok((fields, current))
    }
}
