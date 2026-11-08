//! *Apply*: the use case of a tool that changes something, run as the person (AI-05, ADR-0013).
//!
//! Each function first runs the preview of its tool again, as the person, now: it is the one check
//! that the issue or the project is still one AI may know, that the version is the one that was
//! read and that the change is still possible, and it gives back what was saved. Then it runs the
//! use case with the request that `requests.rs` makes from the same input, so what is saved is
//! what was shown. There is no second way to change anything.

use bb_policy::{Authorized, Masked};
use serde_json::{Value, json};

use super::call::{ToolUndo, UNDO_PART, UndoKind, VERSION_PART};
use super::registry::{Boxed, Run};
use super::{preview, requests};
use crate::access::Actor;
use crate::ai::context::{AuthorKind, Part};
use crate::auth::{AppError, AuthStore, Ctx};
use crate::comments::add_comment as add_comment_as_person;
use crate::issues::{IssueView, change_issue, create_issue as create_issue_as_person};
use crate::time::{
    add_time, start_timer as start_timer_as_person, stop_timer as stop_timer_as_person,
};

/// What undoes an *Apply*: the part that tells where the thing it made is. It is not text for the
/// AI; the call takes it out of the answer.
fn undo_part<T: Masked + serde::Serialize>(kind: UndoKind, made: &Authorized<T>) -> Part {
    let value = made.masked_value().unwrap_or(Value::Null);
    let id = value.get("id").and_then(Value::as_str);
    let undo = ToolUndo {
        kind,
        id: id.unwrap_or_default().to_owned(),
        version: version_in(&value).unwrap_or_default(),
    };
    Part {
        name: UNDO_PART,
        value: json!(undo),
        author: AuthorKind::Assistant,
        project: None,
    }
}

/// The version of the thing a write gave back.
fn version_in(value: &Value) -> Option<i64> {
    value.get("version").and_then(Value::as_i64)
}

/// The version an issue is at after a change, as the write gave it back: not read again, so it is
/// the version of this change and never one a later change by somebody else made.
fn version_part(changed: &Authorized<IssueView>) -> Part {
    let version = changed
        .masked_value()
        .ok()
        .and_then(|value| version_in(&value));
    Part {
        name: VERSION_PART,
        value: json!(version),
        author: AuthorKind::Assistant,
        project: None,
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Whether the actor may delete the issue they just made.
    async fn may_delete_made(&self, actor: &Actor, made: &Authorized<IssueView>) -> bool {
        let key = made
            .masked_value()
            .ok()
            .and_then(|value| value.get("key").and_then(Value::as_str).map(str::to_owned));
        let Some(key) = key else { return false };
        match self.visible_issue(actor, &key).await {
            Ok((issue, held)) => held.may_delete_issue(issue.reported_by),
            Err(_) => false,
        }
    }
}

/// `create_issue`.
pub(super) fn create_issue<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let mut shown = preview::create_issue(ctx, run).await?;
        let made = create_issue_as_person(ctx, requests::create_issue(&run)?).await?;
        // *Undo* of a new issue sends it to the trash: only for a person who may delete it.
        if ctx.may_delete_made(run.actor, &made).await {
            shown.push(undo_part(UndoKind::Issue, &made));
        }
        Ok(shown)
    })
}

/// `update_issue`.
pub(super) fn update_issue<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let mut shown = preview::update_issue(ctx, run).await?;
        let changed = change_issue(ctx, requests::update_issue(&run)?).await?;
        shown.push(version_part(&changed));
        Ok(shown)
    })
}

/// `change_status`.
pub(super) fn change_status<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let mut shown = preview::change_status(ctx, run).await?;
        let changed = change_issue(ctx, requests::change_status(&run)?).await?;
        shown.push(version_part(&changed));
        Ok(shown)
    })
}

/// `add_comment`.
pub(super) fn add_comment<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let mut shown = preview::add_comment(ctx, run).await?;
        let made = add_comment_as_person(ctx, requests::add_comment(&run)?).await?;
        shown.push(undo_part(UndoKind::Comment, &made));
        Ok(shown)
    })
}

/// `log_time`.
pub(super) fn log_time<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let mut shown = preview::log_time(ctx, run).await?;
        let made = add_time(ctx, requests::log_time(&run)?).await?;
        shown.push(undo_part(UndoKind::TimeEntry, &made));
        Ok(shown)
    })
}

/// `start_timer`.
pub(super) fn start_timer<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let mut shown = preview::start_timer(ctx, run).await?;
        let made = start_timer_as_person(ctx, requests::start_timer(&run)?).await?;
        shown.push(undo_part(UndoKind::Timer, &made));
        Ok(shown)
    })
}

/// `stop_timer`.
pub(super) fn stop_timer<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let shown = preview::stop_timer(ctx, run).await?;
        // Time that was worked and recorded is never erased by *Undo*: stopping the timer has no way back.
        stop_timer_as_person(ctx, requests::stop_timer(&run)?).await?;
        Ok(shown)
    })
}
