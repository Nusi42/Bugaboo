//! Saving the description of an issue (ISS-17, ADR-0019): long text goes through the edit lock.
//! The text, the token of the lock and the version the person started from are checked together
//! with the write (`Ctx::save_text`), so a refused save keeps the text as the draft of the person
//! and nothing is lost. Short values keep using [`change_issue`](super::change_issue).

use bb_core::ErrorCode;
use bb_core::edit::EditWhat;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::fields::Field;
use bb_core::id::UserId;
use bb_core::technical::TechnicalName;
use bb_core::version::{counted_up, next as next_version};
use bb_policy::{Authorized, Place};
use serde_json::json;

use super::fields::{ensure_may_change, member};
use super::types::{IssueView, SaveDescription};
use crate::auth::{AppError, AuthStore, Ctx, by, entity, internal};
use crate::editing::{TextSave, Written};
use crate::input::clean_long_text;

/// Saves the description of an issue under the lock the person holds, on the version they started
/// from.
///
/// # Errors
/// `resource.not_found` for an issue that does not exist, is in the trash or that the person may
/// not see; `access.forbidden` without the right to change the issue or its description;
/// `request.invalid` for a text that is too long; `edit.lock_lost`, `edit.taken_over` and
/// `edit.version_changed` for a lock that is gone, was taken over, or an issue that changed since
/// (the text is kept as the draft of the person in all three).
pub async fn save_description<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SaveDescription,
) -> Result<Authorized<IssueView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (current, held) = ctx.visible_issue(&actor, &input.issue).await?;
    if !held.may_change_issue(current.reported_by) {
        return Err(ErrorCode::Forbidden.into());
    }
    ensure_may_change(&held.field_mask(), &[Field::Description])?;
    let text = clean_long_text(&input.text)?;
    let assigned: Vec<UserId> = ctx
        .store()
        .assignees(current.id)
        .await
        .map_err(|error| internal::<S>(&error))?
        .into_iter()
        .map(|assignee| assignee.person)
        .collect();
    let project = ctx
        .store()
        .project(current.project)
        .await
        .map_err(|error| internal::<S>(&error))?
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let mut views = ctx.issue_views(&actor, held.clone(), project).await?;
    let mut next = current.clone();
    next.description.clone_from(&text);
    next.updated_at = ctx.now();
    let mut changes = Vec::new();
    if current.description != text {
        changes.push(FieldChange {
            field: member::DESCRIPTION.to_owned(),
            before: Some(json!(current.description)),
            after: Some(json!(text)),
        });
    }
    let id = current.id;
    let record = entity(RecordKind::Issue, id);
    // The lock covers the text only: a change of another field since the person started is no
    // conflict, so the text goes on the version the issue has now.
    let version = ctx
        .version_for_text(&record, member::DESCRIPTION, input.version, current.version)
        .await?;
    changes.push(counted_up(version));
    let event = ctx.event(by(actor.person.id), record.clone(), changes)?;
    let what = EditWhat {
        record,
        part: TechnicalName::new(member::DESCRIPTION)
            .map_err(|error| AppError::Internal(error.to_string()))?,
    };
    let save = TextSave {
        what,
        token: input.lock_token,
        text,
        version: input.version,
    };
    let written = next.clone();
    ctx.save_text(
        actor.person.id,
        Place::Project(current.project),
        &save,
        &event,
        move |transaction| {
            Box::pin(async move {
                let Some(stored) = S::issue_in(transaction, id).await? else {
                    return Ok(Written::Stale);
                };
                if stored.deleted_at.is_some() || stored.version != version {
                    return Ok(Written::Stale);
                }
                if S::update_issue(transaction, &written, version).await? {
                    Ok(Written::Saved(()))
                } else {
                    Ok(Written::Stale)
                }
            })
        },
    )
    .await?;
    next.version = next_version(version);
    let view = ctx.issue_view(&mut views, &next, &assigned).await?;
    Ok(held.authorize(view))
}
