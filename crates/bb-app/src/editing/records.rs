//! What the editing use cases need to know about a record without knowing its kind: where it
//! sits, which version it has, and what a person needs to change it.
//!
//! Locks, drafts and the check of versions work on any record by its [`EntityRef`]. A kind of record
//! (issues, comments, …) says what it is by implementing [`EditableRecords`] and handing it to
//! the services (`AuthServices::with_records`); nothing in the editing code names a kind. The
//! project is the one kind that `bb-app` knows itself, because its version is stored since M2.

use std::sync::Arc;

use bb_core::event::{EntityKind, EntityRef, RecordKind};
use bb_core::id::{ProjectId, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_policy::{Permission, Place};

use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::ports::BoxFuture;
use bb_core::ErrorCode;

/// What a record tells the editing use cases about itself for one person.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecordFacts {
    /// Where the record sits: the permissions are judged there.
    pub place: Place,
    /// The version the record has now.
    pub version: i64,
    /// What this person needs to change the record (for an issue: *Change issues*, or *Change own
    /// issues* when they reported it).
    pub edit: Permission,
}

/// How a change of a record is shown to somebody who was told that the record changed
/// meanwhile (`edit.version_changed`, ADR-0019 amendment M3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shown {
    /// Not at all: a field nobody has said how to show stays out, so a new field never leaks
    /// before its kind decides.
    Hidden,
    /// The value as it is, for a field whose value is no personal data (still left out when the
    /// role of the reader hides a field of the catalogue).
    Plain,
    /// The value is a person (the ID in the event): the reader gets a `PersonRef`, shown as the
    /// kind of people information says (*Assigned to*, *Reported by*).
    Person(PeopleKind),
}

/// The kinds of record the editing use cases can work on, besides the ones `bb-app` knows.
pub trait EditableRecords: Send + Sync {
    /// The facts of a record for a person, or `None` when there is no such record or the person
    /// may not know about it: the two look the same.
    ///
    /// # Errors
    /// The store failed.
    fn look_up<'a>(
        &'a self,
        person: UserId,
        record: &'a EntityRef,
    ) -> BoxFuture<'a, Result<Option<RecordFacts>, AppError>>;

    /// Whether `part` is a part of `record` that can be edited under a lock or kept as a draft. A
    /// kind lists its parts; one that says nothing has none, so nothing can be locked or drafted on
    /// it (SEC-06).
    fn has_part(&self, _record: &EntityRef, _part: &str) -> bool {
        false
    }

    /// How the change of the field `field` of `record` is shown in a conflict. A kind that says
    /// nothing hides every field.
    fn shows(&self, _record: &EntityRef, _field: &str) -> Shown {
        Shown::Hidden
    }

    /// What a person needs to make a new record of this kind in a project (the form behind a
    /// draft of a new record), or `None` for a kind that has no form.
    fn making(&self, kind: &EntityKind) -> Option<Permission>;
}

/// Several kinds of record, each told by its own `EditableRecords`: the program hands the services
/// one of these when more than one package knows a kind (issues, comments, …). A record is looked
/// up in each in turn and belongs to the first that knows it; a field is shown as the first that
/// says anything about it shows it; a form is the first that has one.
pub struct ChainedRecords(Vec<Arc<dyn EditableRecords>>);

impl ChainedRecords {
    /// The kinds of all of these, in this order.
    #[must_use]
    pub fn new(kinds: Vec<Arc<dyn EditableRecords>>) -> Self {
        Self(kinds)
    }
}

impl EditableRecords for ChainedRecords {
    fn look_up<'a>(
        &'a self,
        person: UserId,
        record: &'a EntityRef,
    ) -> BoxFuture<'a, Result<Option<RecordFacts>, AppError>> {
        Box::pin(async move {
            for kind in &self.0 {
                if let Some(facts) = kind.look_up(person, record).await? {
                    return Ok(Some(facts));
                }
            }
            Ok(None)
        })
    }

    fn has_part(&self, record: &EntityRef, part: &str) -> bool {
        self.0.iter().any(|kind| kind.has_part(record, part))
    }

    fn shows(&self, record: &EntityRef, field: &str) -> Shown {
        self.0
            .iter()
            .map(|kind| kind.shows(record, field))
            .find(|shown| *shown != Shown::Hidden)
            .unwrap_or(Shown::Hidden)
    }

    fn making(&self, kind: &EntityKind) -> Option<Permission> {
        self.0.iter().find_map(|each| each.making(kind))
    }
}

/// No other kinds: the services start with this, and the program adds its kinds.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoRecords;

impl EditableRecords for NoRecords {
    fn look_up<'a>(
        &'a self,
        _person: UserId,
        _record: &'a EntityRef,
    ) -> BoxFuture<'a, Result<Option<RecordFacts>, AppError>> {
        Box::pin(async { Ok(None) })
    }

    fn making(&self, _kind: &EntityKind) -> Option<Permission> {
        None
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// What the actor may know about the record: its place, version and the permission that
    /// changes it. A record the actor may not know about is `resource.not_found`, exactly like
    /// one that does not exist.
    pub(crate) async fn facts_of(
        &self,
        actor: &Actor,
        record: &EntityRef,
    ) -> Result<RecordFacts, AppError> {
        let not_found = || AppError::Refused(ErrorCode::NotFound);
        if record.kind == EntityKind::from(RecordKind::Project) {
            let id = ProjectId::from_bytes(record.id.to_bytes()).map_err(|_| not_found())?;
            let access = self
                .evaluation_of(actor, Place::Project(id))
                .await
                .map_err(|_| not_found())?;
            if !access.sees_project() {
                return Err(not_found());
            }
            let project = self
                .store()
                .project(id)
                .await
                .map_err(|error| internal::<S>(&error))?
                .ok_or_else(not_found)?;
            return Ok(RecordFacts {
                place: Place::Project(id),
                version: project.version,
                edit: Permission::ChangeProjectSettings,
            });
        }
        let facts = if record.kind == EntityKind::from(RecordKind::Issue) {
            self.issue_facts(actor, record).await?
        } else if record.kind == EntityKind::from(RecordKind::Layout) {
            self.layout_facts(actor, record).await?
        } else {
            self.services
                .records
                .look_up(actor.person.id, record)
                .await?
        }
        .ok_or_else(not_found)?;
        // A record sits in a project: whoever does not see the project does not know the record,
        // whatever its kind says.
        if let Place::Project(_) = facts.place {
            let access = self.evaluation_of(actor, facts.place).await?;
            if !access.sees_project() {
                return Err(not_found());
            }
        }
        Ok(facts)
    }
}
