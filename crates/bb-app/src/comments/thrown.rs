//! The comments an agent wrote in a run that is undone (ADR-0029, amendment M9): *Throw away*, *Undo
//! the run* and the agent's own `undo_my_run` take them along. Their text goes everywhere it is kept;
//! the issue keeps one event that says which comments were there and were thrown away, with no text.

use std::collections::BTreeMap;

use bb_core::event::{FieldChange, RecordKind};
use bb_core::id::{CommentId, EntityId, IssueId, UserId};
use serde_json::json;

use crate::auth::{AppError, AuthStore, Ctx, by, entity, internal, set_to};
use crate::event_members::value_set;
use crate::member::comment::THROWN_AWAY;

/// How many comments of an agent an event of an issue says were thrown away; `None` for an event
/// that says nothing of it.
pub(crate) fn thrown_count(changes: &[FieldChange]) -> Option<u64> {
    changes
        .iter()
        .any(|change| change.field == THROWN_AWAY)
        .then(|| {
            value_set(changes, THROWN_AWAY)
                .and_then(serde_json::Value::as_array)
                .map_or(0, |named| named.len() as u64)
        })
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Throws the comments away, one change for each issue, as `person` (the event maker records
    /// an agent that asks as the agent).
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn throw_away_comments(
        &self,
        person: UserId,
        thrown: &[(EntityId, IssueId)],
    ) -> Result<(), AppError> {
        let mut by_issue: BTreeMap<IssueId, Vec<CommentId>> = BTreeMap::new();
        for (comment, issue) in thrown {
            let id = CommentId::from_bytes(comment.to_bytes())
                .map_err(|error| AppError::Internal(error.to_string()))?;
            by_issue.entry(*issue).or_default().push(id);
        }
        for (issue, comments) in by_issue {
            let named: Vec<String> = comments.iter().map(ToString::to_string).collect();
            let event = self.event(
                by(person),
                entity(RecordKind::Issue, issue),
                vec![set_to(THROWN_AWAY, json!(named))],
            )?;
            self.store()
                .record_change(&event, move |transaction| {
                    Box::pin(async move { S::throw_away_comments(transaction, &comments).await })
                })
                .await
                .map_err(|error| internal::<S>(&error))?;
        }
        Ok(())
    }
}
