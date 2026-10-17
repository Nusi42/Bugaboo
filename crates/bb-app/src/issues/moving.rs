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
use bb_core::id::{IdSource, IssueId, IssueTypeId, ProjectId, UserId};
use bb_core::issue::{Issue, IssueAlias, StatusGroup};
use bb_core::project::{Project, ShortName};
use bb_core::readable_id::{IssueCode, ReadableId};
use bb_core::version::{counted_up, next as next_version};
use bb_core::workflow::StatusKey;
use bb_policy::{Authorized, Evaluation, Permission, Place};
use jiff::Timestamp;
use serde_json::json;

use super::ids::{code_plan_in, new_code_in};
use super::store::MovedStatus;
use super::types::{IssueView, MoveIssue};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, EventMaker, by, entity, internal};
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
    /// The status and status group the issue starts again in, when the workflow that counts in
    /// the project it goes to lacks the status it has (ADR-0031 amendment 3).
    restart: Option<(StatusKey, StatusGroup)>,
}

impl MovePlan {
    /// The project the issue goes to.
    pub(crate) const fn target_id(&self) -> ProjectId {
        self.target.id
    }

    /// The change of the status the move makes, before and after, when it makes one.
    pub(crate) fn status_change(&self) -> Option<FieldChange> {
        self.restart.as_ref().map(|(status, _)| FieldChange {
            field: member::STATUS.to_owned(),
            before: Some(json!(self.issue.status.as_str())),
            after: Some(json!(status.as_str())),
        })
    }
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

    /// The project issues are moved into, as the person asks for it, and what they hold there: it
    /// is not there for a person who does not see it. The one check of the target that moving an
    /// issue and a change to many issues (the whole selection) make.
    ///
    /// # Errors
    /// `resource.not_found` for a project that is not there for the person.
    pub(crate) async fn move_target(
        &self,
        actor: &Actor,
        project: &str,
    ) -> Result<(Project, Arc<Evaluation>), AppError> {
        let target = self
            .find_project(project)
            .await?
            .ok_or(AppError::Refused(ErrorCode::NotFound))?
            .project;
        let held = self.evaluation_of(actor, Place::Project(target.id)).await?;
        if !held.sees_project() {
            return Err(ErrorCode::NotFound.into());
        }
        Ok((target, held))
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
        let (target, held) = self.move_target(actor, project).await?;
        if !held.allows(Permission::MoveIssues) {
            return Err(ErrorCode::Forbidden.into());
        }
        let already_there = target.id == issue.project;
        let mut restart = None;
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
            let status = self
                .status_after_change(&chain, issue.issue_type, &issue.status)
                .await?;
            if status != issue.status {
                let workflow = self
                    .workflow_in_force(&chain, Some(issue.issue_type))
                    .await?;
                let group = workflow
                    .status(&status)
                    .map(|each| each.group)
                    .ok_or(AppError::Refused(ErrorCode::WorkflowNoSuchChange))?;
                restart = Some((status, group));
            }
        }
        Ok(MovePlan {
            issue,
            target,
            held,
            already_there,
            restart,
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
        restart,
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
        let mut changes = vec![FieldChange {
            field: member::PROJECT.to_owned(),
            before: Some(json!(issue.project.to_string())),
            after: Some(json!(target.id.to_string())),
        }];
        changes.extend(restart.as_ref().map(|(status, _)| FieldChange {
            field: member::STATUS.to_owned(),
            before: Some(json!(issue.status.as_str())),
            after: Some(json!(status.as_str())),
        }));
        changes.push(counted_up(issue.version));
        let event = ctx.event(by(actor.person.id), record.clone(), changes)?;
        let now = ctx.now();
        let (status, closed_at) = match &restart {
            Some((status, group)) => (status.clone(), group.closed_after(issue.closed_at, now)),
            None => (issue.status.clone(), issue.closed_at),
        };
        let writer = MoveWrite {
            id: issue.id,
            issue_type: issue.issue_type,
            from: issue.project,
            from_code: issue.code.clone(),
            to: target.id,
            to_short_name: target.short_name.clone(),
            status: status.clone(),
            closed_at,
            version: issue.version,
        };
        let (events, mut ids, why) = (
            ctx.event_maker(by(actor.person.id)),
            ctx.owned_ids(),
            Why::default(),
        );
        let stopped = why.clone();
        let code = ctx
            .store()
            .record_change(&event, move |transaction| {
                Box::pin(async move {
                    writer
                        .run::<S>(transaction, &events, &mut ids, now, &stopped)
                        .await
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
            status,
            closed_at,
            version: next_version(issue.version),
            updated_at: now,
            ..issue.clone()
        }
    };
    let assigned: Vec<UserId> = ctx
        .store()
        .assigned_people(moved.id)
        .await
        .map_err(|error| internal::<S>(&error))?
        .into_iter()
        .map(|assigned_person| assigned_person.person)
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
    to_short_name: ShortName,
    status: StatusKey,
    closed_at: Option<Timestamp>,
    version: i64,
}

impl MoveWrite {
    /// Writes the move inside the change: the codes of the project it goes to are given out one
    /// change at a time, the type is judged again on what is stored now, the issue gets a code of
    /// the new project, the old code is kept as a redirect, and the new readable ID is a second
    /// event of the issue (the first one could not name a code that is given out in the change).
    async fn run<S: AuthStore>(
        self,
        transaction: &mut S::Transaction,
        events: &EventMaker,
        ids: &mut impl IdSource,
        now: Timestamp,
        stopped: &Why,
    ) -> Result<IssueCode, S::Error> {
        S::lock_issue_codes(transaction, self.to).await?;
        let offered = S::issue_types_in(transaction)
            .await?
            .iter()
            .find(|found| found.id == self.issue_type)
            .is_some_and(|found| found.retired_at.is_none());
        if !offered {
            return Err(stopped.stop::<S>(ErrorCode::IssueTypeRetired));
        }
        let plan = code_plan_in::<S>(transaction, self.to).await?;
        let code = new_code_in::<S>(transaction, self.to, plan, ids).await?;
        if !S::move_issue(
            transaction,
            self.id,
            self.to,
            &code,
            &MovedStatus {
                status: self.status.clone(),
                closed_at: self.closed_at,
            },
            self.version,
            now,
        )
        .await?
        {
            return Err(stopped.stop::<S>(ErrorCode::VersionChanged));
        }
        S::insert_issue_alias(
            transaction,
            &IssueAlias {
                project: self.from,
                code: self.from_code.clone(),
                issue: self.id,
                retired_at: now,
            },
        )
        .await?;
        let old_short = S::project_in(transaction, self.from)
            .await?
            .ok_or_else(S::aborted)?
            .short_name;
        let key = |short_name: &ShortName, code: &IssueCode| {
            json!(ReadableId::new(short_name.clone(), code.clone()).to_string())
        };
        let followed = events
            .create(
                entity(RecordKind::Issue, self.id),
                vec![FieldChange {
                    field: member::KEY.to_owned(),
                    before: Some(key(&old_short, &self.from_code)),
                    after: Some(key(&self.to_short_name, &code)),
                }],
            )
            .map_err(|_| S::aborted())?;
        S::append_follow_up_event(transaction, &followed).await?;
        Ok(code)
    }
}
