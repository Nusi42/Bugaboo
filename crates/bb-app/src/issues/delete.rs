//! Deleting an issue into the trash, and taking it out again (*Undo*, ISS-01). Nothing is removed:
//! an issue in the trash is not found, not listed and not counted, and keeps its readable ID.

use bb_core::ErrorCode;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::id::UserId;
use bb_core::version::counted_up;
use bb_policy::{Authorized, Place};
use serde_json::json;

use super::fields::member;
use super::types::{DeleteIssue, IssueView, RestoreIssue};
use crate::auth::{AppError, AuthStore, Ctx, Done, by, entity, internal};
use crate::projects::Why;

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
    let found = ctx.find_issue(&input.issue).await?;
    let place = found.as_ref().map_or(Place::Organization, |found| {
        Place::Project(found.issue.project)
    });
    let held = ctx.evaluation_of(&actor, place).await?;
    let Some(issue) = found
        .map(|found| found.issue)
        .filter(|issue| issue.deleted_at.is_none() && held.sees_issue(issue.reported_by))
    else {
        return Err(AppError::Refused(ErrorCode::NotFound));
    };
    if !held.may_delete_issue(issue.reported_by) {
        return Err(ErrorCode::Forbidden.into());
    }
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
                if S::set_issue_deleted(transaction, id, Some(now)).await? {
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
/// `resource.not_found` for an issue that does not exist, is not in the trash or that the person
/// may not see, `access.forbidden` without *Delete issues*.
pub async fn restore_issue<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RestoreIssue,
) -> Result<Authorized<IssueView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let found = ctx.find_issue(&input.issue).await?;
    let place = found.as_ref().map_or(Place::Organization, |found| {
        Place::Project(found.issue.project)
    });
    let held = ctx.evaluation_of(&actor, place).await?;
    let Some(issue) = found
        .map(|found| found.issue)
        .filter(|issue| issue.deleted_at.is_some() && held.sees_issue(issue.reported_by))
    else {
        return Err(AppError::Refused(ErrorCode::NotFound));
    };
    if !held.may_delete_issue(issue.reported_by) {
        return Err(ErrorCode::Forbidden.into());
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
    let why = Why::default();
    let stopped = why.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if S::set_issue_deleted(transaction, id, None).await? {
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
        .assignees(id)
        .await
        .map_err(|error| internal::<S>(&error))?
        .into_iter()
        .map(|assignee| assignee.person)
        .collect();
    let mut views = ctx.issue_views(&actor, held.clone(), project).await?;
    let view = ctx.issue_view(&mut views, &restored, &assigned_to).await?;
    Ok(held.authorize(view))
}
