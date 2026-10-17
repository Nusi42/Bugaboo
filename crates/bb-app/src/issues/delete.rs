//! Deleting an issue into the trash, and taking it out again (*Undo*, ISS-01). Nothing is removed:
//! an issue in the trash is not found, not listed and not counted, and keeps its readable ID.

use bb_core::ErrorCode;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::id::UserId;
use bb_core::issue::Issue;
use bb_core::version::counted_up;
use bb_policy::Authorized;
use serde_json::json;

use super::fields::member;
use super::types::{DeleteIssue, IssueView, RestoreIssue};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, Done, by, entity, internal};
use crate::projects::Why;

impl<S: AuthStore> Ctx<'_, S> {
    /// The issue a person asks to put into the trash: they see it and may delete it. What the
    /// preview of a change to many issues judges, and what deleting judges first.
    ///
    /// # Errors
    /// The errors of [`delete_issue`].
    pub(crate) async fn issue_to_delete(
        &self,
        actor: &Actor,
        issue: &str,
    ) -> Result<Issue, AppError> {
        let (issue, held) = self.visible_issue(actor, issue).await?;
        if held.may_delete_issue(issue.reported_by) {
            Ok(issue)
        } else {
            Err(ErrorCode::Forbidden.into())
        }
    }
}

/// Puts an issue into the trash. The person needs *Delete issues* in the project and has to see
/// the issue.
///
/// # Errors
/// `resource.not_found` for an issue that does not exist, is in the trash already or that the
/// person may not see, `access.forbidden` without *Delete issues*.
pub async fn delete_issue<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: DeleteIssue,
) -> Result<Done, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let issue = ctx.issue_to_delete(&actor, &input.issue).await?;
    let now = ctx.now();
    let id = issue.id;
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Issue, id),
        vec![
            FieldChange {
                field: member::DELETED_AT.to_owned(),
                before: None,
                after: Some(json!(now.to_string())),
            },
            counted_up(issue.version),
        ],
    )?;
    let why = Why::default();
    let stopped = why.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if S::set_issue_deleted(transaction, id, Some(now), now).await? {
                    Ok(())
                } else {
                    Err(stopped.stop::<S>(ErrorCode::NotFound))
                }
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::NotFound))?;
    Ok(Done)
}

/// Takes an issue out of the trash (*Undo*). The person needs *Delete issues* in the project, as
/// for putting it there, and has to see the issue.
///
/// # Errors
/// `resource.not_found` for an issue that does not exist, is not in the trash, that the person
/// may not see or that they may not take out: whoever lacks *Delete issues* is not told that an
/// issue is in the trash (SEC-06).
pub async fn restore_issue<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RestoreIssue,
) -> Result<Authorized<IssueView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (issue, held) = ctx.trashed_issue(&actor, &input.issue).await?;
    if !held.may_delete_issue(issue.reported_by) {
        return Err(ErrorCode::NotFound.into());
    }
    let id = issue.id;
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Issue, id),
        vec![
            FieldChange {
                field: member::DELETED_AT.to_owned(),
                before: issue.deleted_at.map(|at| json!(at.to_string())),
                after: None,
            },
            counted_up(issue.version),
        ],
    )?;
    let (why, now) = (Why::default(), ctx.now());
    let stopped = why.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if S::set_issue_deleted(transaction, id, None, now).await? {
                    Ok(())
                } else {
                    Err(stopped.stop::<S>(ErrorCode::NotFound))
                }
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::NotFound))?;
    let restored = ctx
        .store()
        .issue(id)
        .await
        .map_err(|error| internal::<S>(&error))?
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let project = ctx
        .store()
        .project(restored.project)
        .await
        .map_err(|error| internal::<S>(&error))?
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let assigned_to: Vec<UserId> = ctx
        .store()
        .assigned_people(id)
        .await
        .map_err(|error| internal::<S>(&error))?
        .into_iter()
        .map(|assigned_person| assigned_person.person)
        .collect();
    let mut views = ctx.issue_views(&actor, held.clone(), project).await?;
    let view = ctx.issue_view(&mut views, &restored, &assigned_to).await?;
    Ok(held.authorize(view))
}
