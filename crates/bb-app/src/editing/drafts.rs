//! Drafts (ISS-11, ADR-0019 amendment M3): the text a person wrote and has not saved, kept
//! privately on the server with the heartbeat, until it is saved, thrown away, or its keep period
//! of 30 days after the last change has passed (M3-Q9).
//!
//! A draft belongs to one person and one target: a part of a record, or the form that makes a new
//! record in a project. Only its author can read it. Nobody can save a draft for something they
//! may not change, so a draft cannot be used to probe for records.

use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::edit::{DRAFT_TEXT_MAX, Draft, DraftTarget, draft_cutoff};
use bb_core::id::UserId;
use bb_policy::{Authorized, Evaluation, Place};
use jiff::Timestamp;

use super::types::{
    DraftSlot, DraftText, DraftView, PurgeDraftsQuery, ReadDraftQuery, SaveDraftQuery,
    ThrowAwayDraftQuery,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, Done, internal};

impl<S: AuthStore> Ctx<'_, S> {
    /// The moment before which a draft is too old to be read or kept.
    fn draft_not_before(&self) -> Result<Timestamp, AppError> {
        draft_cutoff(self.now())
            .ok_or_else(|| AppError::Internal("a keep period that cannot be written".to_owned()))
    }

    /// Whether the actor may keep a draft for the target: they may change the part of the record,
    /// or make a record of that kind in the project.
    async fn allow_draft(
        &self,
        actor: &Actor,
        target: &DraftTarget,
    ) -> Result<Arc<Evaluation>, AppError> {
        match target {
            DraftTarget::Editing(what) => {
                let facts = self.facts_of(actor, &what.record).await?;
                self.require(actor, facts.edit, facts.place).await
            }
            DraftTarget::NewRecord { project, kind } => {
                let permission = self
                    .services
                    .records
                    .making(kind)
                    .ok_or(AppError::Refused(ErrorCode::NotFound))?;
                self.require(actor, permission, Place::Project(*project))
                    .await
            }
        }
    }

    /// Keeps the text as the draft of `author` for the target, in place of an older one.
    ///
    /// # Errors
    /// `request.too_large` for a text over [`DRAFT_TEXT_MAX`] characters.
    pub(super) async fn keep_draft(
        &self,
        author: UserId,
        target: &DraftTarget,
        text: DraftText,
    ) -> Result<Timestamp, AppError> {
        if text.text.chars().count() > DRAFT_TEXT_MAX {
            return Err(ErrorCode::TooLarge.into());
        }
        let saved_at = self.now();
        let draft = Draft {
            author,
            target: target.clone(),
            text: text.text,
            base_version: text.base_version,
            saved_at,
        };
        self.store()
            .save_draft(&draft)
            .await
            .map_err(|error| internal::<S>(&error))?;
        Ok(saved_at)
    }
}

/// Keeps a text as the draft of the person.
///
/// # Errors
/// `resource.not_found` for a record or project the person may not know about, `access.forbidden`
/// when they may not change it, `request.too_large` for a text over the limit.
pub async fn save_draft<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SaveDraftQuery,
) -> Result<Authorized<DraftView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx.allow_draft(&actor, &input.target).await?;
    let base_version = input.draft.base_version;
    let text = input.draft.text.clone();
    let saved_at = ctx
        .keep_draft(actor.person.id, &input.target, input.draft)
        .await?;
    Ok(access.authorize(DraftView {
        text,
        base_version,
        saved_at,
    }))
}

/// The draft of the person for a target: theirs and nobody else's. A draft older than its keep
/// period is as good as deleted, whenever the job that deletes it runs.
///
/// # Errors
/// `resource.not_found` for a record or project the person may not know about, `access.forbidden`
/// when they may not change it.
pub async fn read_draft<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ReadDraftQuery,
) -> Result<Authorized<DraftSlot>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx.allow_draft(&actor, &input.target).await?;
    let draft = ctx
        .store()
        .draft(actor.person.id, input.target, ctx.draft_not_before()?)
        .await
        .map_err(|error| internal::<S>(&error))?
        .map(|draft| DraftView {
            text: draft.text,
            base_version: draft.base_version,
            saved_at: draft.saved_at,
        });
    Ok(access.authorize(DraftSlot { draft }))
}

/// Throws the draft of the person away (*Throw away*). No draft is no error. The person need not
/// still be allowed to change the record: they only delete their own text.
///
/// # Errors
/// Only that the session is not one.
pub async fn throw_away_draft<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ThrowAwayDraftQuery,
) -> Result<Done, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    ctx.store()
        .delete_draft(actor.person.id, &input.target)
        .await
        .map_err(|error| internal::<S>(&error))?;
    Ok(Done)
}

/// Deletes the drafts, and the notes for people who were editing, that outlived the keep period.
/// What the background job of the kind `editing.purge_drafts` does (ADR-0017); safe to run again.
/// Returns how many drafts went.
///
/// # Errors
/// The error of the store.
pub async fn purge_drafts<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    _input: PurgeDraftsQuery,
) -> Result<u64, AppError> {
    ctx.store()
        .purge_drafts(ctx.draft_not_before()?)
        .await
        .map_err(|error| internal::<S>(&error))
}
