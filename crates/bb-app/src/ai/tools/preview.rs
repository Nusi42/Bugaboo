//! The previews of the tools that change something (AI-21, ADR-0013): what the use case would
//! record, answered without saving. A preview is the use case's own plan, never a second judgment;
//! the assistant shows it as a proposal and *Apply* runs the use case itself.
//!
//! A preview reads like any other answer: nothing about an issue or a project kept from AI is in
//! it (it is `resource.not-found` for them), no change of a field kept from AI is listed, the
//! values that are there now were written by people and are fenced as theirs, and only the values
//! the AI proposes are fenced as the proposer's.

use std::collections::BTreeMap;

use bb_core::ErrorCode;
use bb_core::ai::{AuthorKind, ToolCaller};
use bb_core::event::FieldChange;
use bb_core::fields::FieldKey;
use bb_core::id::{IssueId, IssueTypeId, UserId};
use bb_core::issue::Priority;
use bb_core::time::Minutes;
use bb_core::workflow::StatusKey;
use serde_json::json;

use super::inputs::{
    AddCommentInput, ChangeStatusInput, CreateIssueInput, LogTimeInput, StartTimerInput,
    StopTimerInput, UpdateIssueInput,
};
use super::registry::{Boxed, Run};
use crate::ai::context::Part;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::comments::AddComment;
use crate::issues::{ChangeIssue, CreateIssue};
use crate::time::{AddTime, EntryDetails, StartTimer, StopTimer};

fn invalid<T, E>(parsed: Result<T, E>) -> Result<T, AppError> {
    parsed.map_err(|_| AppError::Refused(ErrorCode::InvalidRequest))
}

fn priority(text: Option<&str>) -> Result<Option<Priority>, AppError> {
    text.map(|text| Priority::from_code(text).ok_or(AppError::Refused(ErrorCode::InvalidRequest)))
        .transpose()
}

fn people(ids: Option<&[String]>) -> Result<Option<Vec<UserId>>, AppError> {
    ids.map(|ids| {
        ids.iter()
            .map(|id| invalid(id.parse::<UserId>()))
            .collect::<Result<Vec<_>, _>>()
    })
    .transpose()
}

/// Who the AI is told wrote the values it proposes.
const fn proposer(caller: ToolCaller) -> AuthorKind {
    match caller {
        ToolCaller::Assistant => AuthorKind::Assistant,
        ToolCaller::AiApp | ToolCaller::Agent => AuthorKind::Agent,
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The kind of author of what the issue holds now, after the issue is found to be one the AI
    /// may know: `resource.not-found` for an issue the person cannot read and for one in a project
    /// kept from AI, the same answer.
    async fn preview_issue(&self, run: &Run<'_>, address: &str) -> Result<AuthorKind, AppError> {
        let (issue, _) = self.visible_issue(run.actor, address).await?;
        self.author_of_issue(run, &issue).await
    }

    /// The preview of a change of an issue, for `update_issue` and `change_status`.
    async fn preview_change_of_issue(
        &self,
        run: &Run<'_>,
        input: ChangeIssue,
    ) -> Result<Vec<Part>, AppError> {
        let before_by = self.preview_issue(run, &input.issue).await?;
        let changes = self.preview_change_issue(run.actor, &input).await?;
        Ok(preview_parts(run, changes, before_by))
    }

    async fn author_of_issue(
        &self,
        run: &Run<'_>,
        issue: &bb_core::issue::Issue,
    ) -> Result<AuthorKind, AppError> {
        if run.withheld.hides_project(issue.project) {
            return Err(AppError::Refused(ErrorCode::NotFound));
        }
        self.author_kind(issue.reported_by, issue.project).await
    }
}

/// The parts of a preview: for each change the field, the value now and the value after. A change
/// of a field kept from AI is left out.
fn preview_parts(run: &Run<'_>, changes: Vec<FieldChange>, before_by: AuthorKind) -> Vec<Part> {
    let mut parts = Vec::new();
    for change in changes {
        let kept = change
            .field
            .parse::<FieldKey>()
            .is_ok_and(|key| run.withheld.hides_field(key));
        if kept {
            continue;
        }
        parts.push(Part {
            name: "would_change",
            value: json!({ "field": change.field }),
            author: proposer(run.caller),
            project: None,
        });
        for (name, value, author) in [
            ("before", change.before, before_by),
            ("after", change.after, proposer(run.caller)),
        ] {
            if let Some(value) = value {
                parts.push(Part {
                    name,
                    value,
                    author,
                    project: None,
                });
            }
        }
    }
    parts
}

/// `create_issue`: a project kept from AI is not there.
pub(super) fn create_issue<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let input: CreateIssueInput = run.input()?;
        let in_kept_project = ctx
            .find_project(&input.project)
            .await?
            .is_some_and(|found| run.withheld.hides_project(found.project.id));
        if in_kept_project {
            return Err(AppError::Refused(ErrorCode::NotFound));
        }
        let changes = ctx
            .preview_create_issue(
                run.actor,
                &CreateIssue {
                    session_token: run.token.to_owned(),
                    project: input.project,
                    issue_type: invalid(input.issue_type.parse::<IssueTypeId>())?,
                    title: input.title,
                    description: input.description,
                    status: None,
                    priority: priority(input.priority.as_deref())?,
                    assigned_to: None,
                    estimated_time: None,
                    fields: BTreeMap::new(),
                },
            )
            .await?;
        Ok(preview_parts(&run, changes, proposer(run.caller)))
    })
}

/// `update_issue`.
pub(super) fn update_issue<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let input: UpdateIssueInput = run.input()?;
        let change = ChangeIssue {
            session_token: run.token.to_owned(),
            issue: input.issue,
            version: input.version,
            issue_type: None,
            title: input.title,
            status: None,
            priority: priority(input.priority.as_deref())?,
            estimated_time: None,
            time_left: None,
            assigned_to: people(input.assigned_to.as_deref())?,
            fields: BTreeMap::new(),
        };
        ctx.preview_change_of_issue(&run, change).await
    })
}

/// `change_status`.
pub(super) fn change_status<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let input: ChangeStatusInput = run.input()?;
        let change = ChangeIssue {
            session_token: run.token.to_owned(),
            issue: input.issue,
            version: input.version,
            issue_type: None,
            title: None,
            status: Some(invalid(StatusKey::new(&input.status))?),
            priority: None,
            estimated_time: None,
            time_left: None,
            assigned_to: None,
            fields: BTreeMap::new(),
        };
        ctx.preview_change_of_issue(&run, change).await
    })
}

/// `add_comment`.
pub(super) fn add_comment<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let input: AddCommentInput = run.input()?;
        let before_by = ctx.preview_issue(&run, &input.issue).await?;
        let changes = ctx
            .preview_add_comment(
                run.actor,
                &AddComment {
                    session_token: run.token.to_owned(),
                    issue: input.issue,
                    text: input.text,
                },
            )
            .await?;
        Ok(preview_parts(&run, changes, before_by))
    })
}

/// `log_time`.
pub(super) fn log_time<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let input: LogTimeInput = run.input()?;
        let before_by = ctx.preview_issue(&run, &input.issue).await?;
        let changes = ctx
            .preview_add_time(
                run.actor,
                &AddTime {
                    session_token: run.token.to_owned(),
                    issue: input.issue,
                    date: invalid(input.date.parse())?,
                    minutes: Minutes::new(input.minutes),
                    details: EntryDetails {
                        note: input.note,
                        ..EntryDetails::default()
                    },
                },
            )
            .await?;
        Ok(preview_parts(&run, changes, before_by))
    })
}

/// `start_timer`.
pub(super) fn start_timer<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let input: StartTimerInput = run.input()?;
        let before_by = ctx.preview_issue(&run, &input.issue).await?;
        let changes = ctx
            .preview_start_timer(
                run.actor,
                &StartTimer {
                    session_token: run.token.to_owned(),
                    issue: input.issue,
                },
            )
            .await?;
        Ok(preview_parts(&run, changes, before_by))
    })
}

/// `stop_timer`: the timer runs on an issue, which must be one the AI may know.
pub(super) fn stop_timer<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let input: StopTimerInput = run.input()?;
        let (changes, on): (Vec<FieldChange>, IssueId) = ctx
            .preview_stop_timer(
                run.actor,
                &StopTimer {
                    session_token: run.token.to_owned(),
                    version: input.version,
                    details: EntryDetails::default(),
                },
            )
            .await?;
        let issue = ctx
            .store()
            .issue(on)
            .await
            .map_err(|error| internal::<S>(&error))?
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        let before_by = ctx.author_of_issue(&run, &issue).await?;
        Ok(preview_parts(&run, changes, before_by))
    })
}
