//! Saving a long text under its lock (ISS-17, ADR-0019): the text, the lock token and the version
//! the person started from are checked together, inside the change that writes the record, so
//! nothing can slip in between the check and the write. When the save is refused, the text is kept
//! as the draft of the person with a plain reason: it is never dropped.

use bb_core::ErrorCode;
use bb_core::edit::{DRAFT_KEPT_BY, DraftTarget, EditWhat, Hold};
use bb_core::event::{DomainEvent, EntityRef};
use bb_core::id::{LockToken, UserId};
use bb_policy::Place;

use super::types::DraftText;
use crate::auth::{AppError, AuthStore, Ctx, Stops, change_failed, internal};
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

/// What the write of a record says about the version the person started from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Written<T> {
    /// The record was still at that version and is written.
    Saved(T),
    /// The record was changed since: nothing is written, and the save is a stale one.
    Stale,
    /// The write found a reason of its own to refuse (a check inside the change failed): nothing
    /// is written, and the save is answered with this code.
    Refused(ErrorCode),
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The version a long text is written against. A lock covers only the text, so a change of
    /// another field of the record since the person started (`sent` is older than `current`) does
    /// not refuse the save: the text goes on the version now. When the text itself (`field`) was
    /// changed since, `sent` is returned, and the write finds the record at another version and
    /// refuses the save as stale (`edit.version-changed`). The write still compares the version
    /// it is given, so a change that lands after this look is refused too (ARCH-VERSIONED).
    ///
    /// # Errors
    /// The error of the store.
    pub async fn version_for_text(
        &self,
        record: &EntityRef,
        field: &str,
        sent: i64,
        current: i64,
    ) -> Result<i64, AppError> {
        if sent >= current {
            return Ok(sent);
        }
        let changed = self
            .store()
            .field_changed_since(record, field, sent)
            .await
            .map_err(|error| internal::<S>(&error))?;
        Ok(if changed { sent } else { current })
    }

    /// Saves a long text: inside one change it checks that the lock is still the editor's, runs
    /// `write` (which writes the record and says whether the record was still at the version the
    /// person started from; any other trouble is its own error), lets go of the lock and deletes
    /// the draft.
    ///
    /// A refused save answers `edit.lock-lost` (the lease ran out or the lock was let go),
    /// `edit.taken-over` (somebody used *Edit anyway*) or the structured `edit.version-changed`,
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
        W: for<'t> FnOnce(&'t mut S::Transaction) -> BoxFuture<'t, Result<Written<T>, S::Error>>
            + Send
            + 'static,
    {
        let why = Why::default();
        let stopped = why.clone();
        let (what, me, now) = (save.what.clone(), person, self.now());
        let hold = Hold {
            holder: person,
            token: save.token,
        };
        let saved = self
            .store()
            .record_change(event, move |transaction| {
                Box::pin(async move {
                    match S::edit_lock_in(transaction, what.clone()).await? {
                        Some(lock) if lock.is_held_by(hold) && !lock.has_expired(now) => {}
                        Some(lock) if !lock.has_expired(now) => {
                            return Err(stopped.stop::<S>(ErrorCode::EditTakenOver));
                        }
                        _ => return Err(stopped.stop::<S>(ErrorCode::EditLockLost)),
                    }
                    let done = match write(transaction).await? {
                        Written::Saved(done) => done,
                        Written::Stale => {
                            return Err(stopped.stop::<S>(ErrorCode::VersionChanged));
                        }
                        Written::Refused(code) => return Err(stopped.stop::<S>(code)),
                    };
                    S::remove_edit_lock_held(transaction, &what, hold).await?;
                    S::delete_draft_in(transaction, me, &DraftTarget::Editing(what)).await?;
                    Ok(done)
                })
            })
            .await;
        match saved {
            Ok(done) => Ok(done),
            Err(error) => {
                // Only a reason this function gave is a refusal of the save; whatever else the write
                // or the store did is a failure of its own, not a stale version.
                let refused = match why.said() {
                    Some(ErrorCode::EditTakenOver) => {
                        self.taken_over_by(me, place, &save.what).await
                    }
                    Some(code) => AppError::Refused(code).about(&save.what.record, place, me),
                    None => change_failed::<S>(&error, Stops::NONE),
                };
                if refused
                    .code()
                    .is_some_and(|code| DRAFT_KEPT_BY.contains(&code))
                {
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
