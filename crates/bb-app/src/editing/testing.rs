//! A kind of record that exists only in tests: a *note* in one project, with a version that a test
//! counts up. Locks, drafts and the check of versions work on any record by its kind and ID, so the tests of
//! them (in `bb-app` against the in-memory store, in `bb-server` against both databases) use this
//! kind and need no table of any real record. Only the tests of this crate, the tests of the server
//! and the developer tool `bb-sample` switch the feature on; a build of the whole workspace has it
//! on for every crate (see `bb-store/Cargo.toml`), the program (`cargo build -p bb-server`) does not.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

use bb_core::event::{EntityKind, EntityRef, InvalidKind};
use bb_core::id::{EntityId, ProjectId, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_policy::{Permission, Place};

use super::records::{EditableRecords, RecordFacts, Shown};
use crate::auth::AppError;
use crate::ports::BoxFuture;

/// The kind of the notes: `note`.
const NOTE: &str = "note";

/// The notes of the tests. Cheap to clone; all clones are the same notes. They sit in a project
/// once a test names one, and at the top before that.
#[derive(Clone, Debug)]
pub struct TestNotes {
    project: Arc<Mutex<Option<ProjectId>>>,
    kind: EntityKind,
    versions: Arc<Mutex<BTreeMap<EntityId, i64>>>,
}

impl TestNotes {
    /// No notes yet.
    ///
    /// # Errors
    /// Never in practice: `note` is a valid kind of record.
    pub fn new() -> Result<Self, InvalidKind> {
        Ok(Self {
            project: Arc::default(),
            kind: EntityKind::new(NOTE)?,
            versions: Arc::default(),
        })
    }

    /// Puts the notes into a project: people need their permissions there.
    pub fn set_project(&self, project: ProjectId) {
        *self.project.lock().unwrap_or_else(PoisonError::into_inner) = Some(project);
    }

    fn versions(&self) -> std::sync::MutexGuard<'_, BTreeMap<EntityId, i64>> {
        self.versions.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Creates a note with this ID, at version 1, and names it.
    #[must_use]
    pub fn add(&self, id: EntityId) -> EntityRef {
        self.versions().insert(id, 1);
        EntityRef {
            kind: self.kind.clone(),
            id,
        }
    }

    /// Somebody changes the note: its version counts up.
    pub fn change(&self, note: &EntityRef) {
        if let Some(version) = self.versions().get_mut(&note.id) {
            *version += 1;
        }
    }

    /// The version of the note now.
    #[must_use]
    pub fn version(&self, note: &EntityRef) -> Option<i64> {
        self.versions().get(&note.id).copied()
    }
}

impl EditableRecords for TestNotes {
    fn look_up<'a>(
        &'a self,
        _person: UserId,
        record: &'a EntityRef,
    ) -> BoxFuture<'a, Result<Option<RecordFacts>, AppError>> {
        let found = (record.kind == self.kind)
            .then(|| self.version(record))
            .flatten()
            .map(|version| RecordFacts {
                place: Place::from(*self.project.lock().unwrap_or_else(PoisonError::into_inner)),
                version,
                edit: Permission::ChangeIssues,
            });
        Box::pin(async move { Ok(found) })
    }

    fn has_part(&self, record: &EntityRef, part: &str) -> bool {
        record.kind == self.kind && part == "description"
    }

    fn shows(&self, _record: &EntityRef, field: &str) -> Shown {
        match field {
            "description" | "title" | "status" | "cost-center" => Shown::Plain,
            "assigned-to" => Shown::Person(PeopleKind::AssignedTo),
            _ => Shown::Hidden,
        }
    }

    fn making(&self, kind: &EntityKind) -> Option<Permission> {
        (*kind == self.kind).then_some(Permission::CreateIssues)
    }
}
