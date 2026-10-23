//! Putting an issue under another one, and taking it out again (ISS-03). Any issue can sit under
//! a parent in any project the person may change; an issue never sits under itself or under one of
//! its own sub-issues, however deep (`bb_core::relation::check_parent`).
//!
//! This is a change of the sub-issue: its version counts up and it is made on the version the
//! person looked at, so a stale *Part of* is told who changed the issue. The event is about the
//! sub-issue; the history of the parent reads the parent from it.

use std::collections::HashMap;

use bb_core::ErrorCode;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::id::IssueId;
use bb_core::issue::Issue;
use bb_core::relation::{IssueParent, SUB_ISSUE_DEPTH_MAX, SUB_ISSUES_MAX, check_parent};
use bb_core::version::counted_up;
use bb_policy::{Authorized, Place};
use serde_json::json;

use super::types::{IssueRelations, SetParent};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, by, entity, internal};
use crate::issues::InTrash;
use crate::issues::member as issue_member;
use crate::projects::Why;

/// The chain above `parent` as it is now, read inside the lock, so two changes at the same moment
/// cannot make a circle between them.
async fn chain_above<S: AuthStore>(
    transaction: &mut S::Transaction,
    parent: IssueId,
) -> Result<HashMap<IssueId, IssueId>, S::Error> {
    let mut chain = HashMap::new();
    let mut at = parent;
    for _ in 0..=SUB_ISSUE_DEPTH_MAX {
        match S::parent_of_in(transaction, at).await? {
            Some(row) => {
                chain.insert(at, row.parent);
                at = row.parent;
            }
            None => break,
        }
    }
    Ok(chain)
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The issue a request wants to put another under, if it names one: the person must see it and
    /// be allowed to change it.
    async fn named_parent(
        &self,
        actor: &Actor,
        address: Option<&str>,
    ) -> Result<Option<Issue>, AppError> {
        let Some(address) = address else {
            return Ok(None);
        };
        let (parent, held) = self.visible_issue(actor, address).await?;
        if !held.may_change_issue(parent.reported_by) {
            return Err(ErrorCode::Forbidden.into());
        }
        Ok(Some(parent))
    }

    /// The parent an issue has, and the parent as the person knows it. A parent the person may not
    /// see is none to them: taking it out changes nothing they could know of, so it neither counts
    /// the version up nor tells them it is there (SEC-06).
    async fn parents_of(
        &self,
        actor: &Actor,
        child: IssueId,
    ) -> Result<(Option<IssueId>, Option<IssueId>), AppError> {
        let stored = self
            .store()
            .parent_of(child)
            .await
            .map_err(|error| internal::<S>(&error))?
            .map(|row| row.parent);
        let known = match stored {
            Some(id) => self.seen_issue_by_id(actor, id).await?.map(|_| id),
            None => None,
        };
        Ok((stored, known))
    }
}

/// Puts the issue under `parent`, or takes it out from under its parent (`None`). Putting it where
/// it is already, or taking out one that sits under none, changes nothing and answers with the
/// issue as it is.
///
/// # Errors
/// `resource.not-found` for an issue or a parent that does not exist or that the person may not
/// see, `access.forbidden` without the right to change both, `issue.parent-circle` for a parent
/// that would make a circle or a chain that is too long, `edit.version-changed` when somebody
/// changed the sub-issue since it was looked at.
pub async fn set_parent<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SetParent,
) -> Result<Authorized<IssueRelations>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (child, held) = ctx.visible_issue(&actor, &input.issue).await?;
    if !held.may_change_issue(child.reported_by) {
        return Err(ErrorCode::Forbidden.into());
    }
    let above = ctx.named_parent(&actor, input.parent.as_deref()).await?;
    let record = entity(RecordKind::Issue, child.id);
    let place = Place::Project(child.project);
    if child.version != input.version {
        return Err(AppError::stale(record, place, actor.person.id));
    }
    let now = ctx.now();
    let (current, known) = ctx.parents_of(&actor, child.id).await?;
    let wanted = above.as_ref().map(|parent| parent.id);
    if known == wanted {
        let view = ctx.standing(&actor, &held, &child).await?.header;
        return Ok(held.authorize(view));
    }
    let shown = |id: Option<IssueId>| id.map(|id| json!(id.to_string()));
    let event = ctx.event(
        by(actor.person.id),
        record.clone(),
        vec![
            FieldChange {
                field: issue_member::PARENT.to_owned(),
                before: shown(current),
                after: shown(wanted),
            },
            counted_up(child.version),
        ],
    )?;
    let (child_id, version, by_whom) = (child.id, child.version, actor.person.id);
    let why = Why::default();
    let stopped = why.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_issue_parents(transaction).await?;
                if let Some(parent) = wanted {
                    // An issue holds only so many sub-issues, counted on what is stored now.
                    if S::sub_issues_of_in(transaction, parent).await?.len() >= SUB_ISSUES_MAX {
                        return Err(stopped.stop::<S>(ErrorCode::LimitReached));
                    }
                    let chain = chain_above::<S>(transaction, parent).await?;
                    if check_parent(child_id, parent, |issue| chain.get(&issue).copied()).is_err() {
                        return Err(stopped.stop::<S>(ErrorCode::ParentCircle));
                    }
                }
                if S::count_issue_up(transaction, child_id, Some(version), InTrash::NotFound, now)
                    .await?
                    .is_none()
                {
                    return Err(stopped.stop::<S>(ErrorCode::VersionChanged));
                }
                match wanted {
                    Some(parent) => {
                        S::set_issue_parent(
                            transaction,
                            &IssueParent {
                                child: child_id,
                                parent,
                                set_by: by_whom,
                                set_at: now,
                            },
                        )
                        .await
                    }
                    None => S::clear_issue_parent(transaction, child_id)
                        .await
                        .map(|_| ()),
                }
            })
        })
        .await
        .map_err(|error| {
            why.answer::<S>(&error, ErrorCode::VersionChanged).about(
                &record,
                place,
                actor.person.id,
            )
        })?;
    let mut changed = child;
    changed.version = bb_core::version::next(version);
    changed.updated_at = now;
    let view = ctx.standing(&actor, &held, &changed).await?.header;
    Ok(held.authorize(view))
}
