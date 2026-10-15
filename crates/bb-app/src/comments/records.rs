//! Comments as records that can be locked, drafted and watched (ADR-0019): the facts the editing
//! use cases need, and how the change of a comment is shown in a conflict.

use bb_core::comment::TEXT_PART;
use bb_core::event::{EntityKind, EntityRef, RecordKind};
use bb_core::id::{CommentId, UserId};
use bb_policy::{Permission, Place};

use super::store::CommentStore;
use crate::access::evaluation_in;
use crate::auth::{AppError, AuthStore, internal};
use crate::editing::{EditableRecords, RecordFacts, Shown};
use crate::member::comment as member;
use crate::ports::BoxFuture;

/// The comments of a store, as `EditableRecords`. Only the author of a comment knows it as a record
/// to edit: nobody else can lock it, draft it or watch its version.
pub struct CommentRecords<S> {
    store: S,
}

impl<S> CommentRecords<S> {
    /// The comments kept in `store`.
    pub const fn new(store: S) -> Self {
        Self { store }
    }
}

impl<S: AuthStore + Send + Sync> EditableRecords for CommentRecords<S> {
    fn look_up<'a>(
        &'a self,
        person: UserId,
        record: &'a EntityRef,
    ) -> BoxFuture<'a, Result<Option<RecordFacts>, AppError>> {
        Box::pin(async move {
            if record.kind != EntityKind::from(RecordKind::Comment) {
                return Ok(None);
            }
            let Ok(id) = CommentId::from_bytes(record.id.to_bytes()) else {
                return Ok(None);
            };
            let failed = |error| internal::<S>(&error);
            let Some(comment) = CommentStore::comment(&self.store, id)
                .await
                .map_err(failed)?
                .filter(|comment| comment.deleted_at.is_none() && comment.author == person)
            else {
                return Ok(None);
            };
            let Some(issue) = self.store.issue(comment.issue).await.map_err(failed)? else {
                return Ok(None);
            };
            let place = Place::Project(issue.project);
            let Some(held) = evaluation_in(&self.store, person, place).await? else {
                return Ok(None);
            };
            Ok((issue.deleted_at.is_none()
                && held.sees_issue(issue.reported_by)
                && held.allows(Permission::Comment))
            .then_some(RecordFacts {
                place,
                version: comment.version,
                edit: Permission::Comment,
            }))
        })
    }

    fn has_part(&self, record: &EntityRef, part: &str) -> bool {
        record.kind == EntityKind::from(RecordKind::Comment) && part == TEXT_PART
    }

    fn shows(&self, record: &EntityRef, field: &str) -> Shown {
        if record.kind == EntityKind::from(RecordKind::Comment) && field == member::TEXT {
            Shown::Plain
        } else {
            Shown::Hidden
        }
    }

    fn making(&self, _kind: &EntityKind) -> Option<Permission> {
        None
    }
}
