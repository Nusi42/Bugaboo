//! The structured answer to a stale version (`edit.version-changed`, ADR-0019 amendment M3): who
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
use bb_core::access::FieldLevel;
use bb_core::event::{Actor as EventActor, DomainEvent, EntityKind, EntityRef, RecordKind};
use bb_core::fields::{Field, FieldKey};
use bb_core::id::{
    FieldId, GroupId, IssueId, IssueTypeId, LabelGroupId, LabelId, LayoutId, ProjectId,
    RelationKindId, RoleId, SavedSearchId, TagId, UserId, WorkflowId,
};
use bb_core::own_fields::FieldType;
use bb_core::people_kinds::PeopleKind;
use bb_policy::{Evaluation, PeopleContext, PersonRef, Place};
use jiff::Timestamp;
use serde::Serialize;
use serde_json::Value;

use super::records::Shown;
use crate::auth::{AppError, AuthStore, Ctx, fixed_entity, internal};
use bb_core::settings::SIGN_UP_POLICY;

use crate::fields::member as fields_member;
use crate::issues::member as issue_member;
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

/// The record with this ID, looked up by `look`, or `None` when the ID is no ID of that kind.
async fn found<I, T, E, Looking: Future<Output = Result<Option<T>, E>>>(
    id: Option<I>,
    look: impl FnOnce(I) -> Looking,
) -> Result<Option<T>, E> {
    match id {
        Some(id) => look(id).await,
        None => Ok(None),
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
    } else if *kind == EntityKind::from(RecordKind::Issue) {
        // What a change of an issue holds and is no list of people: who is assigned stays out
        // until the conflict can show a list of people (M3-F32).
        &[
            issue_member::ISSUE_TYPE,
            issue_member::TITLE,
            issue_member::DESCRIPTION,
            issue_member::STATUS,
            issue_member::PRIORITY,
        ]
    } else if *kind == EntityKind::from(RecordKind::IssueType) {
        &[
            issue_member::issue_type::NAME,
            issue_member::issue_type::ICON,
            issue_member::issue_type::RETIRED,
        ]
    } else if *kind == EntityKind::from(RecordKind::Comment) {
        &[member::comment::TEXT]
    } else if *kind == EntityKind::from(RecordKind::Field) {
        &[fields_member::NAME, fields_member::RETIRED]
    } else if *kind == EntityKind::from(RecordKind::RelationKind) {
        &[
            member::relation_kind::FORWARD,
            member::relation_kind::BACKWARD,
            member::relation_kind::RETIRED,
        ]
    } else if *kind == EntityKind::from(RecordKind::LabelGroup) {
        &[
            member::label_group::NAME,
            member::label_group::PICK_ONE,
            member::label_group::RETIRED,
        ]
    } else if *kind == EntityKind::from(RecordKind::Label) {
        &[
            member::label::NAME,
            member::label::COLOR,
            member::label::RETIRED,
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
    /// The kind of an own field that a change of an issue or a project names, when the reader may
    /// see the field on that record (on its issue type, for an issue); `None` when the key is no
    /// own field of the record or the reader may not see it (CF-06).
    async fn own_field_shown(
        &self,
        viewer: &Evaluation,
        record: &EntityRef,
        key: FieldKey,
    ) -> Result<Option<FieldType>, AppError> {
        let Some(field) = key.own() else {
            return Ok(None);
        };
        if record.kind != EntityKind::from(RecordKind::Issue)
            && record.kind != EntityKind::from(RecordKind::Project)
        {
            return Ok(None);
        }
        let issue_type = self.issue_type_of(record).await?;
        if viewer.field_level_for(issue_type, key) < FieldLevel::CanSee {
            return Ok(None);
        }
        Ok(self
            .field_catalogue()
            .await?
            .iter()
            .find(|definition| definition.field.id == field)
            .map(|definition| definition.field.kind))
    }

    /// The issue type of the record when it is an issue that is still there: the mask of a reader
    /// is the mask of this type (ADR-0027).
    async fn issue_type_of(&self, record: &EntityRef) -> Result<Option<IssueTypeId>, AppError> {
        if record.kind != EntityKind::from(RecordKind::Issue) {
            return Ok(None);
        }
        let Ok(id) = IssueId::from_bytes(record.id.to_bytes()) else {
            return Ok(None);
        };
        Ok(self
            .store()
            .issue(id)
            .await
            .map_err(|error| internal::<S>(&error))?
            .map(|found| found.issue_type))
    }

    /// The version the record has now, if it is still there and the person may know about it: one
    /// table, by kind of record, for every route that takes a version (the kinds `bb-app` keeps
    /// itself) and the kinds the program adds (`EditableRecords`).
    async fn version_now(&self, stale: &Stale) -> Result<Option<i64>, AppError> {
        let record = &stale.record;
        let bytes = record.id.to_bytes();
        let store = self.store();
        let failed = |error| internal::<S>(&error);
        let Some(kind) = RecordKind::from_code(record.kind.as_str()) else {
            return self.version_by_kind(stale).await;
        };
        let version = match kind {
            RecordKind::Project => found(ProjectId::from_bytes(bytes).ok(), |id| store.project(id))
                .await
                .map_err(failed)?
                .map(|project| project.version),
            RecordKind::Group => found(GroupId::from_bytes(bytes).ok(), |id| store.group(id))
                .await
                .map_err(failed)?
                .map(|group| group.version),
            RecordKind::Setting => {
                // The one setting record with a version: the sign-up policy.
                if *record == fixed_entity(RecordKind::Setting, SIGN_UP_POLICY)? {
                    Some(store.sign_up_policy_version().await.map_err(failed)?)
                } else {
                    None
                }
            }
            RecordKind::IssueType => found(IssueTypeId::from_bytes(bytes).ok(), |id| {
                store.issue_type(id)
            })
            .await
            .map_err(failed)?
            .map(|found| found.version),
            RecordKind::Issue => found(IssueId::from_bytes(bytes).ok(), |id| store.issue(id))
                .await
                .map_err(failed)?
                .filter(|issue| issue.deleted_at.is_none())
                .map(|issue| issue.version),
            RecordKind::Role => found(RoleId::from_bytes(bytes).ok(), |id| store.role(id))
                .await
                .map_err(failed)?
                .map(|found| found.role.version),
            RecordKind::Field
            | RecordKind::RelationKind
            | RecordKind::Layout
            | RecordKind::Workflow
            | RecordKind::LabelGroup
            | RecordKind::Label
            | RecordKind::Tag => self.version_in_list(kind, bytes).await?,
            RecordKind::Comment => self
                .comment_facts(stale.viewer, record)
                .await?
                .map(|facts| facts.version),
            // A saved search is private to its maker: nobody else is told its version.
            RecordKind::SavedSearch => found(SavedSearchId::from_bytes(bytes).ok(), |id| {
                store.saved_search(id)
            })
            .await
            .map_err(failed)?
            .filter(|search| search.maker == stale.viewer && search.deleted_at.is_none())
            .map(|search| search.version),
            _ => return self.version_by_kind(stale).await,
        };
        Ok(version)
    }

    /// The version of a field, a kind of relation, a layout, a group of labels, a label or a tag.
    async fn version_in_list(
        &self,
        kind: RecordKind,
        bytes: [u8; 16],
    ) -> Result<Option<i64>, AppError> {
        let store = self.store();
        let failed = |error| internal::<S>(&error);
        Ok(match kind {
            RecordKind::Field => {
                let id = FieldId::from_bytes(bytes).ok();
                let definitions = self.field_catalogue().await?;
                definitions
                    .iter()
                    .find(|definition| Some(definition.field.id) == id)
                    .map(|definition| definition.field.version)
            }
            RecordKind::RelationKind => {
                let id = RelationKindId::from_bytes(bytes).ok();
                store
                    .relation_kinds()
                    .await
                    .map_err(failed)?
                    .iter()
                    .find(|kind| Some(kind.id) == id)
                    .map(|kind| kind.version)
            }
            RecordKind::Layout => found(LayoutId::from_bytes(bytes).ok(), |id| {
                store.stored_layout(id)
            })
            .await
            .map_err(failed)?
            .map(|stored| stored.layout.version),
            RecordKind::Workflow => found(WorkflowId::from_bytes(bytes).ok(), |id| {
                store.stored_workflow(id)
            })
            .await
            .map_err(failed)?
            .map(|stored| stored.workflow.version),
            RecordKind::LabelGroup => {
                let id = LabelGroupId::from_bytes(bytes).ok();
                store
                    .label_groups()
                    .await
                    .map_err(failed)?
                    .iter()
                    .find(|group| Some(group.id) == id)
                    .map(|group| group.version)
            }
            RecordKind::Label => {
                let id = LabelId::from_bytes(bytes).ok();
                store
                    .labels()
                    .await
                    .map_err(failed)?
                    .iter()
                    .find(|label| Some(label.id) == id)
                    .map(|label| label.version)
            }
            RecordKind::Tag => {
                let id = TagId::from_bytes(bytes).ok();
                store
                    .tags()
                    .await
                    .map_err(failed)?
                    .iter()
                    .find(|tag| Some(tag.id) == id)
                    .map(|tag| tag.version)
            }
            _ => None,
        })
    }

    /// The version of a kind of record the program adds to the services (`EditableRecords`).
    async fn version_by_kind(&self, stale: &Stale) -> Result<Option<i64>, AppError> {
        Ok(self
            .services
            .records
            .look_up(stale.viewer, &stale.record)
            .await?
            .map(|facts| facts.version))
    }

    /// Makes the conflict out of a stale record. If anything fails on the way, the person still gets
    /// the plain `edit.version-changed`: a conflict is never turned into another error.
    pub(crate) async fn conflict_of(&self, stale: &Stale) -> AppError {
        match self.try_conflict(stale).await {
            Ok(conflict) => AppError::Conflict(Box::new(conflict)),
            Err(error) => {
                tracing::warn!(cause = %error, "the conflict answer could not be made; the plain refusal is sent");
                AppError::Refused(ErrorCode::VersionChanged)
            }
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
        let mask = viewer.field_mask_for(self.issue_type_of(record).await?);
        let mut fields = Vec::new();
        let mut current = BTreeMap::new();
        for change in event
            .changes
            .iter()
            .filter(|change| change.field != bb_core::version::FIELD)
        {
            // The value of an own field goes through the reader's mask; a person, group or issue in
            // it is not shown by its ID, only that the field changed.
            if let Some(key) = FieldKey::from_code(&change.field).filter(|key| key.own().is_some())
            {
                if let Some(kind) = self.own_field_shown(viewer, record, key).await? {
                    fields.push(change.field.clone());
                    if let (Some(value), false) = (
                        &change.after,
                        matches!(
                            kind,
                            FieldType::Person
                                | FieldType::People
                                | FieldType::Group
                                | FieldType::Issue
                        ),
                    ) {
                        current.insert(change.field.clone(), value.clone());
                    }
                }
                continue;
            }
            let shown = match shown_by_us(&record.kind, &change.field) {
                Shown::Hidden => self.services.records.shows(record, &change.field),
                known => known,
            };
            let mask_hides =
                Field::from_code(&change.field).is_some_and(|field| !mask.can_see(field));
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
