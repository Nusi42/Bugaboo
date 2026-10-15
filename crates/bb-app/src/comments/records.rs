//! Comments as records that can be locked, drafted and watched (ADR-0019): the facts the editing
//! use cases need. A comment is a kind that `bb-app` keeps itself, like an issue or a layout, so
//! no program has to plug it in (`editing/records.rs` calls this).

use bb_core::event::{EntityKind, EntityRef, RecordKind};
use bb_core::id::{CommentId, UserId};
use bb_policy::{Permission, Place};

use super::store::CommentStore;
use crate::access::evaluation_in;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::editing::RecordFacts;

impl<S: AuthStore> Ctx<'_, S> {
    /// The facts of a comment for `person`; `None` when there is no such comment or the person may
    /// not know it as a record to edit: only the author does, so nobody else can lock it, draft it
    /// or watch its version.
    pub(crate) async fn comment_facts(
        &self,
        person: UserId,
        record: &EntityRef,
    ) -> Result<Option<RecordFacts>, AppError> {
        if record.kind != EntityKind::from(RecordKind::Comment) {
            return Ok(None);
        }
        let Ok(id) = CommentId::from_bytes(record.id.to_bytes()) else {
            return Ok(None);
        };
        let store = self.store();
        let failed = |error| internal::<S>(&error);
        let Some(comment) = CommentStore::comment(store, id)
            .await
            .map_err(failed)?
            .filter(|comment| comment.deleted_at.is_none() && comment.author == person)
        else {
            return Ok(None);
        };
        let Some(issue) = store.issue(comment.issue).await.map_err(failed)? else {
            return Ok(None);
        };
        let place = Place::Project(issue.project);
        let Some(held) = evaluation_in(store, person, place).await? else {
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
    }
}
