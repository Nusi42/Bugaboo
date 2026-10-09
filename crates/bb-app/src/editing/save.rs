//! Saving a long text under its lock (ISS-17, ADR-0019): the text, the lock token and the version
//! the person started from are checked together, inside the change that writes the record, so
//! nothing can slip in between the check and the write. When the save is refused, the text is kept
//! as the draft of the person with a plain reason: it is never dropped.

use bb_core::ErrorCode;
use bb_core::edit::{DraftTarget, EditWhat};
use bb_core::event::DomainEvent;
use bb_core::id::{LockToken, UserId};
use bb_policy::Place;

use super::types::DraftText;
use crate::auth::{AppError, AuthStore, Ctx};
use crate::ports::BoxFuture;
use crate::projects::Why;

/// A long text to save for a record that exists.
pub struct TextSave {
    /// The thing that was edited.
    pub what: EditWhat,
    /// The token the lock was taken with.
    pub token: LockToken,
    /// The text.
    pub text: String,
    /// The version of the record the person started from.
    pub version: i64,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Saves a long text: inside one change it checks that the lock is still the editor's, runs
    /// `write` (which writes the record and says whether the record was still at the version the
    /// person started from: `None` when it was not), lets go of the lock and deletes the draft.
    ///
    /// A refused save answers `edit.lock_lost` (the lease ran out or the lock was let go),
    /// `edit.taken_over` (somebody used *Edit anyway*) or the structured `edit.version_changed`,
    /// and keeps the text as the draft of the person.
    ///
    /// # Errors
    /// The three answers above; the error of the store.
    pub async fn save_text<T, W>(
        &self,
        person: UserId,
        place: Place,
        save: &TextSave,
        event: &DomainEvent,
        write: W,
    ) -> Result<T, AppError>
    where
        T: Send + 'static,
        W: for<'t> FnOnce(&'t mut S::Transaction) -> BoxFuture<'t, Result<Option<T>, S::Error>>
            + Send
            + 'static,
    {
        let why = Why::default();
        let stopped = why.clone();
        let (what, token, me, now) = (save.what.clone(), save.token, person, self.now());
        let saved = self
            .store()
            .record_change(event, move |transaction| {
                Box::pin(async move {
                    match S::edit_lock_in(transaction, what.clone()).await? {
                        Some(lock) if lock.token == token && !lock.has_expired(now) => {}
                        Some(lock) if !lock.has_expired(now) => {
                            return Err(stopped.stop::<S>(ErrorCode::EditTakenOver));
                        }
                        _ => return Err(stopped.stop::<S>(ErrorCode::EditLockLost)),
                    }
                    let Some(done) = write(transaction).await? else {
                        return Err(stopped.stop::<S>(ErrorCode::VersionChanged));
                    };
                    S::remove_edit_lock_with_token(transaction, &what, token).await?;
                    S::delete_draft_in(transaction, me, &DraftTarget::Editing(what)).await?;
                    Ok(done)
                })
            })
            .await;
        match saved {
            Ok(done) => Ok(done),
            Err(error) => {
                let refused = why.answer::<S>(&error, ErrorCode::VersionChanged).about(
                    &save.what.record,
                    place,
                    me,
                );
                if matches!(
                    refused.code(),
                    Some(
                        ErrorCode::EditLockLost
                            | ErrorCode::EditTakenOver
                            | ErrorCode::VersionChanged
                    )
                ) {
                    self.keep_draft(
                        me,
                        &DraftTarget::Editing(save.what.clone()),
                        DraftText {
                            text: save.text.clone(),
                            base_version: Some(save.version),
                        },
                    )
                    .await?;
                }
                Err(refused)
            }
        }
    }
}
