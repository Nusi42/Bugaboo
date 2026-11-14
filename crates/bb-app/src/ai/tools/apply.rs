//! *Apply*: the use case of a tool that changes something, run as the person (AI-05, ADR-0013).
//!
//! Each function first runs the preview of its tool again, as the person, now: it is the one check
//! that the issue or the project is still one AI may know, that the version is the one that was
//! read and that the change is still possible, and it gives back what was saved. Then it runs the
//! use case with the request that `requests.rs` makes from the same input, so what is saved is
//! what was shown. There is no second way to change anything.

use bb_policy::{Authorized, Masked};
use serde_json::{Value, json};

use super::call::{Saved, ToolUndo, UNDO_PART, UndoKind};
use super::registry::{Boxed, Run};
use super::{preview, read, requests};
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

/// The issue a call made, as the AI may name it from now on: its internal ID, its readable ID and
/// its version. The search finds a new issue only after the next round of the jobs (ADR-0016), so
/// this is how the caller knows it was made and does not make it again.
fn made_issue_part(made: &Authorized<IssueView>) -> Part {
    let value = made.masked_value().unwrap_or(Value::Null);
    Part {
        name: "issue",
        value: json!({
            "id": value.get("id"),
            "key": value.get("key"),
            "version": value.get("version"),
        }),
        author: AuthorKind::Assistant,
        project: read::project_of(&value),
    }
}

/// What a write of an issue saved, as the write gave it back: the readable ID and the version, not
/// read again, so it is the version of this change and never one a later change by somebody else
/// made.
fn issue_saved(written: &Authorized<IssueView>, new: bool) -> Part {
    saved_issue(&written.masked_value().unwrap_or(Value::Null), new).part()
}

/// [`issue_saved`] from the masked value of the write: without a readable ID the actor may name,
/// the version stays, because *Undo* of fields is made on it.
fn saved_issue(value: &Value, new: bool) -> Saved {
    let key = value.get("key").and_then(Value::as_str).map(str::to_owned);
    match (key, version_in(value)) {
        (Some(key), Some(version)) if new => Saved::NewIssue { key, version },
        (Some(key), Some(version)) => Saved::Issue { key, version },
        (None, Some(version)) => Saved::Version { version },
        (_, None) => Saved::Done,
    }
}

/// What stopping the timer saved, from the masked value of the stop: the entry, named by its
/// readable ID when the actor may name it; nothing booked when the timer ran for less than a
/// minute, which is told by there being no entry.
fn saved_stop(value: &Value) -> Saved {
    match value.get("entry").filter(|entry| !entry.is_null()) {
        None => Saved::TimerStoppedEmpty,
        Some(entry) => Saved::on(
            entry
                .get("readable_id")
                .and_then(Value::as_str)
                .map(str::to_owned),
        ),
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// What was saved on the issue at `address`, by the readable ID the actor may name it by.
    async fn saved_on(&self, actor: &Actor, address: &str) -> Result<Part, AppError> {
        let (issue, _) = self.visible_issue(actor, address).await?;
        let key = self.readable_id_for(actor, issue.id).await?;
        Ok(Saved::on(key).part())
    }

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
        shown.push(made_issue_part(&made));
        shown.push(issue_saved(&made, true));
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
        shown.push(issue_saved(&changed, false));
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
        shown.push(issue_saved(&changed, false));
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
        let request = requests::add_comment(&run)?;
        let on = ctx.saved_on(run.actor, &request.issue).await?;
        let made = add_comment_as_person(ctx, request).await?;
        shown.push(undo_part(UndoKind::Comment, &made));
        shown.push(on);
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
        let request = requests::log_time(&run)?;
        let on = ctx.saved_on(run.actor, &request.issue).await?;
        let made = add_time(ctx, request).await?;
        shown.push(undo_part(UndoKind::TimeEntry, &made));
        shown.push(on);
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
        let request = requests::start_timer(&run)?;
        let on = ctx.saved_on(run.actor, &request.issue).await?;
        let made = start_timer_as_person(ctx, request).await?;
        shown.push(undo_part(UndoKind::Timer, &made));
        shown.push(on);
        Ok(shown)
    })
}

/// `stop_timer`.
pub(super) fn stop_timer<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let mut shown = preview::stop_timer(ctx, run).await?;
        // Time that was worked and recorded is never erased by *Undo*: stopping the timer has no way back.
        let stopped = stop_timer_as_person(ctx, requests::stop_timer(&run)?).await?;
        let stopped = stopped
            .masked_value()
            .map_err(|error| AppError::Internal(error.to_string()))?;
        shown.push(saved_stop(&stopped).part());
        Ok(shown)
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn ai_21_a_saved_change_keeps_its_version_without_a_readable_id() {
        assert_eq!(
            saved_issue(&json!({"key": "WEB-1", "version": 5}), false),
            Saved::Issue {
                key: "WEB-1".to_owned(),
                version: 5
            }
        );
        let unnamed = saved_issue(&json!({"version": 5}), false);
        assert_eq!(unnamed, Saved::Version { version: 5 });
        assert_eq!(unnamed.version_of_change(), Some(5));
        assert_eq!(saved_issue(&json!({}), false), Saved::Done);
    }

    #[test]
    fn ai_21_a_stopped_timer_says_booked_by_the_entry_not_by_its_readable_id() {
        assert_eq!(saved_stop(&json!({"timer": {}})), Saved::TimerStoppedEmpty);
        assert_eq!(
            saved_stop(&json!({"timer": {}, "entry": null})),
            Saved::TimerStoppedEmpty
        );
        // An entry the actor may not name by its readable ID was still booked.
        assert_eq!(
            saved_stop(&json!({"timer": {}, "entry": {"minutes": 5}})),
            Saved::Done
        );
        assert_eq!(
            saved_stop(&json!({"entry": {"readable_id": "WEB-1"}})),
            Saved::On {
                key: "WEB-1".to_owned()
            }
        );
    }

    #[test]
    fn ai_21_the_line_of_what_was_saved_never_names_the_address_of_the_caller() {
        // Without a readable ID the actor may name, the line says only that it was saved.
        assert_eq!(Saved::on(None), Saved::Done);
        assert_eq!(
            Saved::on(Some("WEB-1".to_owned())),
            Saved::On {
                key: "WEB-1".to_owned()
            }
        );
    }
}
