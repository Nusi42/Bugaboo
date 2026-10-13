//! Moving an issue into another project (ISS-09, ADR-0021 amendment, M3-Q4).
//!
//! The person needs *Move issues* in the project the issue is in and in the project it goes to, and
//! sees both. The issue gets a readable ID of the new project, in that project's mode, and the old
//! one keeps working (`issue_alias`). M3 moves an issue only where it can live on without losing
//! anything: the project offers its issue type, uses every own field the issue has a value in, and
//! allows the tags it has. What cannot move is named, so the preview of a change to many issues
//! says why an issue is skipped.

use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::id::{IssueId, IssueTypeId, ProjectId, UserId};
use bb_core::issue::{Issue, IssueAlias};
use bb_core::issue_key::IssueCode;
use bb_core::project::Project;
use bb_core::version::{counted_up, next as next_version};
use bb_policy::{Authorized, Evaluation, Permission, Place};
use serde_json::json;

use super::ids::{code_plan_in, new_code_in};
use super::types::{IssueView, MoveIssue};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, by, entity, internal};
use crate::issues::member;
use crate::projects::Why;

/// A move that the person may make, judged but not written yet.
pub(crate) struct MovePlan {
    /// The issue as it is.
    pub(crate) issue: Issue,
    /// The project it goes to.
    target: Project,
    /// What the person may do in the project it goes to.
    held: Arc<Evaluation>,
    /// The issue is in that project already: there is nothing to do.
    pub(crate) already_there: bool,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Whether a project offers an issue type to its issues. Until types are limited to projects
    /// (M3-F22) a project offers every type that is not retired; this is the one place that says
    /// so, for a move and for the preview of one.
    pub(crate) async fn offers_issue_type(
        &self,
        _project: ProjectId,
        issue_type: IssueTypeId,
    ) -> Result<bool, AppError> {
        Ok(self
            .store()
            .issue_type(issue_type)
            .await
            .map_err(|error| internal::<S>(&error))?
            .is_some_and(|found| found.retired_at.is_none()))
    }

    /// Judges moving an issue into a project: what the preview of a change to many issues shows,
    /// and what moving judges first.
    ///
    /// # Errors
    /// The errors of [`move_issue`].
    pub(crate) async fn plan_move(
        &self,
        actor: &Actor,
        issue: &str,
        project: &str,
    ) -> Result<MovePlan, AppError> {
        let (issue, from) = self.visible_issue(actor, issue).await?;
        if !from.allows(Permission::MoveIssues) {
            return Err(ErrorCode::Forbidden.into());
        }
        let target = self
            .find_project(project)
            .await?
            .ok_or(AppError::Refused(ErrorCode::NotFound))?
            .project;
        let held = self.evaluation_of(actor, Place::Project(target.id)).await?;
        if !held.sees_project() {
            return Err(ErrorCode::NotFound.into());
        }
        if !held.allows(Permission::MoveIssues) {
            return Err(ErrorCode::Forbidden.into());
        }
        let already_there = target.id == issue.project;
        if !already_there {
            if !self.offers_issue_type(target.id, issue.issue_type).await? {
                return Err(ErrorCode::IssueTypeRetired.into());
            }
            let chain = self.chain_of(target.id).await?;
            if !self.fields_follow(&issue, &chain).await? {
                return Err(ErrorCode::FieldNotHere.into());
            }
            if !self.tags_follow(issue.id, &chain).await? {
                return Err(ErrorCode::TagNotHere.into());
            }
        }
        Ok(MovePlan {
            issue,
            target,
            held,
            already_there,
        })
    }
}

/// Moves an issue into another project, on the version the person looked at. The issue gets a
/// readable ID of the new project and keeps the old one as a redirect. Moving an issue to the
/// project it is in changes nothing and writes nothing.
///
/// # Errors
/// `resource.not_found` for an issue or a project that is not there for the person,
/// `access.forbidden` without *Move issues* in either project, `issue.type_retired` when the
/// project does not offer the issue's type, `field.not_here` when it does not use a field the
/// issue has a value in, `tag.not_here` for a tag the project does not allow,
/// `edit.version_changed` when somebody changed the issue since it was looked at.
pub async fn move_issue<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: MoveIssue,
) -> Result<Authorized<IssueView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let plan = ctx.plan_move(&actor, &input.issue, &input.project).await?;
    let MovePlan {
        issue,
        target,
        held,
        already_there,
    } = plan;
    let place = Place::Project(issue.project);
    if issue.version != input.version {
        return Err(AppError::stale(
            entity(RecordKind::Issue, issue.id),
            place,
            actor.person.id,
        ));
    }
    let record = entity(RecordKind::Issue, issue.id);
    let moved = if already_there {
        issue.clone()
    } else {
        let event = ctx.event(
            by(actor.person.id),
            record.clone(),
            vec![
                FieldChange {
                    field: member::PROJECT.to_owned(),
                    before: Some(json!(issue.project.to_string())),
                    after: Some(json!(target.id.to_string())),
                },
                counted_up(issue.version),
            ],
        )?;
        let writer = MoveWrite {
            id: issue.id,
            issue_type: issue.issue_type,
            from: issue.project,
            from_code: issue.code.clone(),
            to: target.id,
            to_short_name: target.short_name.clone(),
            version: issue.version,
        };
        let (events, mut ids, now, why) = (
            ctx.event_maker(by(actor.person.id)),
            ctx.owned_ids(),
            ctx.now(),
            Why::default(),
        );
        let stopped = why.clone();
        let code = ctx
            .store()
            .record_change(&event, move |transaction| {
                Box::pin(async move {
                    // The codes of the project the issue goes to are given out one change at a
                    // time, and the type is judged again on what is stored now.
                    S::lock_issue_codes(transaction, writer.to).await?;
                    let offered = S::issue_types_in(transaction)
                        .await?
                        .iter()
                        .find(|found| found.id == writer.issue_type)
                        .is_some_and(|found| found.retired_at.is_none());
                    if !offered {
                        return Err(stopped.stop::<S>(ErrorCode::IssueTypeRetired));
                    }
                    let plan = code_plan_in::<S>(transaction, writer.to).await?;
                    let code = new_code_in::<S>(transaction, writer.to, plan, &mut ids).await?;
                    if !S::move_issue(
                        transaction,
                        writer.id,
                        writer.to,
                        &code,
                        writer.version,
                        now,
                    )
                    .await?
                    {
                        return Err(stopped.stop::<S>(ErrorCode::VersionChanged));
                    }
                    S::insert_issue_alias(
                        transaction,
                        &IssueAlias {
                            project: writer.from,
                            code: writer.from_code.clone(),
                            issue: writer.id,
                            retired_at: now,
                        },
                    )
                    .await?;
                    // The new ID is a second event of the issue: the first one could not name a
                    // code that is given out inside the change.
                    let key =
                        |short_name: &str, code: &IssueCode| json!(format!("{short_name}-{code}"));
                    let old_short = S::project_in(transaction, writer.from)
                        .await?
                        .map_or_else(String::new, |found| found.short_name.to_string());
                    let followed = events
                        .make(
                            entity(RecordKind::Issue, writer.id),
                            vec![FieldChange {
                                field: member::KEY.to_owned(),
                                before: Some(key(&old_short, &writer.from_code)),
                                after: Some(key(writer.to_short_name.as_str(), &code)),
                            }],
                        )
                        .map_err(|_| S::aborted())?;
                    S::append_follow_up_event(transaction, &followed).await?;
                    Ok(code)
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
        Issue {
            project: target.id,
            code,
            version: next_version(issue.version),
            updated_at: now,
            ..issue.clone()
        }
    };
    let assigned: Vec<UserId> = ctx
        .store()
        .assignees(moved.id)
        .await
        .map_err(|error| internal::<S>(&error))?
        .into_iter()
        .map(|assignee| assignee.person)
        .collect();
    let mut views = ctx.issue_views(&actor, held.clone(), target).await?;
    let view = ctx.issue_view(&mut views, &moved, &assigned).await?;
    Ok(held.authorize(view))
}

/// What the change that moves an issue writes.
struct MoveWrite {
    id: IssueId,
    issue_type: IssueTypeId,
    from: ProjectId,
    from_code: IssueCode,
    to: ProjectId,
    to_short_name: bb_core::project::ShortName,
    version: i64,
}
