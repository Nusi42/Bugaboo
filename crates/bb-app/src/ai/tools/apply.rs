//! *Apply*: the use case of a tool that changes something, run as the person (AI-05, ADR-0013).
//!
//! Each function first runs the preview of its tool again, as the person, now: it is the one check
//! that the issue or the project is still one AI may know, that the version is the one that was
//! read and that the change is still possible, and it gives back what was saved. Then it runs the
//! use case with the request that `requests.rs` makes from the same input, so what is saved is
//! what was shown. There is no second way to change anything.

use super::registry::{Boxed, Run};
use super::{preview, requests};
use crate::ai::context::Part;
use crate::auth::{AppError, AuthStore, Ctx};
use crate::comments::add_comment as add_comment_as_person;
use crate::issues::{change_issue, create_issue as create_issue_as_person};
use crate::time::{
    add_time, start_timer as start_timer_as_person, stop_timer as stop_timer_as_person,
};

/// `create_issue`.
pub(super) fn create_issue<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let shown = preview::create_issue(ctx, run).await?;
        create_issue_as_person(ctx, requests::create_issue(&run)?).await?;
        Ok(shown)
    })
}

/// `update_issue`.
pub(super) fn update_issue<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let shown = preview::update_issue(ctx, run).await?;
        change_issue(ctx, requests::update_issue(&run)?).await?;
        Ok(shown)
    })
}

/// `change_status`.
pub(super) fn change_status<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let shown = preview::change_status(ctx, run).await?;
        change_issue(ctx, requests::change_status(&run)?).await?;
        Ok(shown)
    })
}

/// `add_comment`.
pub(super) fn add_comment<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let shown = preview::add_comment(ctx, run).await?;
        add_comment_as_person(ctx, requests::add_comment(&run)?).await?;
        Ok(shown)
    })
}

/// `log_time`.
pub(super) fn log_time<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let shown = preview::log_time(ctx, run).await?;
        add_time(ctx, requests::log_time(&run)?).await?;
        Ok(shown)
    })
}

/// `start_timer`.
pub(super) fn start_timer<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let shown = preview::start_timer(ctx, run).await?;
        start_timer_as_person(ctx, requests::start_timer(&run)?).await?;
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
        stop_timer_as_person(ctx, requests::stop_timer(&run)?).await?;
        Ok(shown)
    })
}
