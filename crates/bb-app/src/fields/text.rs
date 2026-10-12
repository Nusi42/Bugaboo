//! Saving a long text of an own field under its edit lock (CF-01, ADR-0019): the text, the lock
//! token and the version of the issue the person started from are checked together, inside the
//! change that writes the value, so nothing slips in between. A refused save keeps the text as the
//! person's draft (`Ctx::save_text`).

use bb_core::ErrorCode;
use bb_core::access::FieldLevel;
use bb_core::edit::{DraftTarget, EditWhat};
use bb_core::event::{FieldChange, RecordKind};
use bb_core::fields::FieldKey;
use bb_core::id::{FieldId, LockToken};
use bb_core::own_fields::{FieldFor, edit_part};
use bb_core::own_fields_value::FieldValue;
use bb_core::text::{LONG_TEXT_MAX, clean_long};
use bb_core::version::{counted_up, next};
use bb_policy::{Authorized, Place};

use std::sync::Arc;

use bb_core::issue::Issue;
use bb_policy::Evaluation;

use super::pattern::Linear;
use super::types::Definition;
use crate::auth::{AppError, AuthStore, Ctx, by, entity, internal};
use crate::editing::{DraftText, TextSave, Written};
use crate::fields::Item;
use crate::issues::IssueView;

/// Save the long text of an own field of an issue.
pub struct SaveFieldText {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID (also an old one) or its internal ID.
    pub issue: String,
    /// The long text field.
    pub field: FieldId,
    /// The token the edit lock was taken with.
    pub token: LockToken,
    /// The version of the issue the person started from.
    pub version: i64,
    /// The text, in Markdown.
    pub text: String,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The issue and the long text field a save is for, once the person may change both.
    async fn text_target(
        &self,
        actor: &crate::access::Actor,
        input: &SaveFieldText,
    ) -> Result<(Arc<Evaluation>, Issue, Definition), AppError> {
        let (held, current) = self.issue_to_change(actor, &input.issue).await?;
        let key = FieldKey::Own(input.field);
        let definition = self
            .field_catalogue()
            .await?
            .iter()
            .find(|definition| definition.field.id == input.field)
            .cloned()
            .filter(|_| {
                held.field_level_for(Some(current.issue_type), key) >= FieldLevel::CanChange
            })
            .ok_or(AppError::Refused(ErrorCode::Forbidden))?;
        let field = &definition.field;
        let item = Item {
            kind: RecordKind::Issue,
            id: Some(current.id.into()),
            chain: self.chain_of(current.project).await?,
            issue_type: Some(current.issue_type),
            reported_by: Some(current.reported_by),
        };
        if !field.kind.is_long_text() {
            return Err(ErrorCode::InvalidRequest.into());
        }
        if field.applies_to != FieldFor::Issues
            || field.retired_at.is_some()
            || !definition.scope.reaches(&item.chain, item.issue_type)
        {
            return Err(ErrorCode::FieldNotHere.into());
        }
        Ok((held, current, definition))
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The answer for a save that is refused before it is tried, with the text kept as the draft
    /// of the person (failing to keep it never hides the reason).
    async fn refused_keeping_draft(
        &self,
        actor: &crate::access::Actor,
        save: &TextSave,
        refused: AppError,
    ) -> AppError {
        self.keep_draft(
            actor.person.id,
            &DraftTarget::Editing(save.what.clone()),
            DraftText {
                text: save.text.clone(),
                base_version: Some(save.version),
            },
        )
        .await
        .ok();
        refused
    }
}

/// Saves the long text of an own field of an issue, under the lock the person holds for it.
///
/// # Errors
/// `resource.not_found` for an issue that is not there or that the person may not see,
/// `access.forbidden` without the right to change the issue or the field, `field.not_here` for a
/// field that is retired or not used for the issue, `request.invalid` for a field that holds no
/// long text or a text that cannot be one, the code of the rule a text breaks, `edit.lock_lost`,
/// `edit.taken_over` and `edit.version_changed` (the text is kept as the draft of the person).
pub async fn save_field_text<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SaveFieldText,
) -> Result<Authorized<IssueView>, AppError> {
    // A lock belongs to an open editor of a person, never to an access key.
    let actor = ctx.session_actor(&input.session_token).await?;
    let (held, current, definition) = ctx.text_target(&actor, &input).await?;
    let (key, field) = (FieldKey::Own(input.field), &definition.field);
    let text = clean_long(&input.text, LONG_TEXT_MAX)
        .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?
        .to_owned();
    let new = (!text.is_empty()).then(|| FieldValue::Text(text.clone()));
    let save = TextSave {
        what: EditWhat {
            record: entity(RecordKind::Issue, current.id),
            part: edit_part(input.field).map_err(|error| AppError::Internal(error.to_string()))?,
        },
        token: input.token,
        text,
        version: input.version,
    };
    // A save that is refused keeps the text as the person's draft, whatever the reason.
    if let Err(violation) = field.rules.check(field.kind, new.as_ref(), &Linear) {
        return Err(ctx
            .refused_keeping_draft(&actor, &save, violation.error().into())
            .await);
    }
    if current.version != input.version {
        let stale = AppError::stale(
            entity(RecordKind::Issue, current.id),
            Place::Project(current.project),
            actor.person.id,
        );
        return Err(ctx.refused_keeping_draft(&actor, &save, stale).await);
    }
    let stored = ctx
        .load_values(RecordKind::Issue, &[current.id.into()])
        .await?;
    let old = stored
        .value(current.id.into(), field.id)
        .and_then(|value| value.to_json(field.kind));
    let record = entity(RecordKind::Issue, current.id);
    let event = ctx.event(
        by(actor.person.id),
        record,
        vec![
            FieldChange {
                field: key.code().into_owned(),
                before: old,
                after: new.as_ref().and_then(|value| value.to_json(field.kind)),
            },
            counted_up(current.version),
        ],
    )?;
    let (id, version, at) = (current.id, current.version, ctx.now());
    let (kind, cells) = (
        bb_core::event::EntityKind::from(RecordKind::Issue),
        new.as_ref().map(FieldValue::to_cells).unwrap_or_default(),
    );
    let touched = Issue {
        updated_at: at,
        ..current.clone()
    };
    let field_id = input.field;
    ctx.save_text(
        actor.person.id,
        Place::Project(current.project),
        &save,
        &event,
        move |transaction| {
            Box::pin(async move {
                if !S::update_issue(transaction, &touched, version).await? {
                    return Ok(Written::Stale);
                }
                S::put_field_value(transaction, kind, id.into(), field_id, &cells).await?;
                Ok(Written::Saved(()))
            })
        },
    )
    .await?;
    let project = ctx
        .store()
        .project(current.project)
        .await
        .map_err(|error| internal::<S>(&error))?
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let mut views = ctx.issue_views(&actor, held.clone(), project).await?;
    let mut saved = current;
    saved.version = next(version);
    saved.updated_at = at;
    let assigned: Vec<bb_core::id::UserId> = ctx
        .store()
        .assignees(saved.id)
        .await
        .map_err(|error| internal::<S>(&error))?
        .into_iter()
        .map(|assignee| assignee.person)
        .collect();
    let view = ctx.issue_view(&mut views, &saved, &assigned).await?;
    Ok(held.authorize(view))
}
