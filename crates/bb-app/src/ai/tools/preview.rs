//! The previews of the tools that change something (AI-21, ADR-0013): what the use case would
//! record, answered without saving. A preview is the use case's own plan, never a second judgment;
//! the assistant shows it as a proposal and *Apply* runs the use case itself.
//!
//! A preview reads like any other answer: nothing about an issue or a project kept from AI is in
//! it (it is `resource.not-found` for them), no change of a field kept from AI is listed, the
//! values that are there now were written by people and are fenced as theirs, and only the values
//! the AI proposes are fenced as the proposer's.

use bb_core::ErrorCode;
use bb_core::ai::{AuthorKind, ToolCaller};
use bb_core::event::FieldChange;
use bb_core::fields::FieldKey;
use bb_core::id::IssueId;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::registry::{Boxed, Run};
use super::requests;
use crate::ai::context::Part;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::issues::ChangeIssue;

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
pub(super) fn preview_parts(
    run: &Run<'_>,
    changes: Vec<FieldChange>,
    before_by: AuthorKind,
) -> Vec<Part> {
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
        let request = requests::create_issue(&run)?;
        let found = ctx.find_project(&request.project).await?;
        if found
            .as_ref()
            .is_some_and(|found| run.withheld.hides_project(found.project.id))
        {
            return Err(AppError::Refused(ErrorCode::NotFound));
        }
        let changes = ctx.preview_create_issue(run.actor, &request).await?;
        // The issue has no ID before it is saved: the preview says which project it is new in, by
        // the short name, once the use case found the project one the person may create issues in.
        let mut parts: Vec<Part> = found
            .map(|found| Part {
                name: "new_issue",
                value: json!({ "short_name": found.project.short_name.to_string() }),
                author: proposer(run.caller),
                project: Some(found.project.id),
            })
            .into_iter()
            .collect();
        parts.extend(preview_parts(&run, changes, proposer(run.caller)));
        Ok(parts)
    })
}

/// `update_issue`.
pub(super) fn update_issue<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        ctx.preview_change_of_issue(&run, requests::update_issue(&run)?)
            .await
    })
}

/// `change_status`.
pub(super) fn change_status<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        ctx.preview_change_of_issue(&run, requests::change_status(&run)?)
            .await
    })
}

/// `add_comment`.
pub(super) fn add_comment<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let request = requests::add_comment(&run)?;
        let before_by = ctx.preview_issue(&run, &request.issue).await?;
        let changes = ctx.preview_add_comment(run.actor, &request).await?;
        Ok(preview_parts(&run, changes, before_by))
    })
}

/// `log_time`.
pub(super) fn log_time<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let request = requests::log_time(&run)?;
        let before_by = ctx.preview_issue(&run, &request.issue).await?;
        let changes = ctx.preview_add_time(run.actor, &request).await?;
        Ok(preview_parts(&run, changes, before_by))
    })
}

/// `start_timer`.
pub(super) fn start_timer<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let request = requests::start_timer(&run)?;
        let before_by = ctx.preview_issue(&run, &request.issue).await?;
        let changes = ctx.preview_start_timer(run.actor, &request).await?;
        Ok(preview_parts(&run, changes, before_by))
    })
}

/// `stop_timer`: the timer runs on an issue, which must be one the AI may know.
pub(super) fn stop_timer<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let (changes, on): (Vec<FieldChange>, IssueId) = ctx
            .preview_stop_timer(run.actor, &requests::stop_timer(&run)?)
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

/// One field a proposal would change, with the value now and the value after, as the person may
/// read them. It is made from the parts of the preview, which are the one account of a preview.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposedChange {
    /// The field (`title`, `priority`, `status`, `comment`, …).
    pub field: String,
    /// The value now; none when the field is new.
    pub before: Option<Value>,
    /// The value after; none when the change takes the value away.
    pub after: Option<Value>,
}

/// The changes a preview lists, in order: each `would_change` part opens one, and the `before` and
/// `after` parts after it fill it.
pub(super) fn proposed_changes(parts: &[Part]) -> Vec<ProposedChange> {
    let mut changes: Vec<ProposedChange> = Vec::new();
    for part in parts {
        match part.name {
            "would_change" => changes.push(ProposedChange {
                field: part
                    .value
                    .get("field")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                before: None,
                after: None,
            }),
            "before" => {
                if let Some(last) = changes.last_mut() {
                    last.before = Some(part.value.clone());
                }
            }
            "after" => {
                if let Some(last) = changes.last_mut() {
                    last.after = Some(part.value.clone());
                }
            }
            _ => {}
        }
    }
    changes
}
