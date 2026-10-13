//! A change to many issues that goes on in the background (ISS-09, ADR-0017): reading how far it
//! is, and doing a step of it.
//!
//! A step changes up to [`BULK_STEP`] issues, each by the use case of a single change, as the
//! person who asked (the job has no session; the run keeps the person). The progress is saved after
//! every issue, so a worker that stops loses nothing: the next one goes on at the first issue that
//! is not counted. When issues are left, the step queues the next one.

use bb_core::ErrorCode;
use bb_core::bulk::{BULK_STEP, BulkRun, BulkSkip, BulkStatus};
use bb_core::id::BulkRunId;
use bb_policy::{Authorized, Place};
use jiff::Timestamp;
use serde_json::Value;

use super::types::{
    BulkAction, BulkRunQuery, BulkRunView, BulkSkipped, BulkTarget, RUN_SKIPPED_MAX,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::jobs::{kind, kind_name};

/// The member of the payload of a step that names the run.
pub(super) const RUN_KEY: &str = "run";

impl<S: AuthStore> Ctx<'_, S> {
    /// How far a run is, as the person who asked for it reads it: the issues it left alone as
    /// that person sees them.
    pub(super) async fn run_view(
        &self,
        actor: &Actor,
        run: &BulkRun,
    ) -> Result<BulkRunView, AppError> {
        let mut maker = self.line_maker().await?;
        let mut skipped = Vec::new();
        for left in run.skipped.iter().take(RUN_SKIPPED_MAX) {
            let issue = match self.issues_named(&[left.issue]).await?.first() {
                Some(found) => self.line_of(actor, &mut maker, found).await?,
                None => None,
            };
            skipped.push(BulkSkipped {
                issue,
                reason: left.reason,
            });
        }
        let count = |number: usize| u32::try_from(number).unwrap_or(u32::MAX);
        Ok(BulkRunView {
            id: run.id,
            status: run.status,
            total: count(run.items.len()),
            done: u32::try_from(run.done).unwrap_or(u32::MAX),
            changed: u32::try_from(run.changed).unwrap_or(u32::MAX),
            skipped_count: count(run.skipped.len()),
            skipped,
            created_at: run.created_at,
            finished_at: run.finished_at,
        })
    }

    /// Does the next step of a run, as the job of the queue says (`payload` names the run).
    ///
    /// # Errors
    /// `request.invalid` for a payload that names no run, and the error of the store.
    pub(crate) async fn run_bulk_step(&self, payload: &Value) -> Result<(), AppError> {
        let id: BulkRunId = payload
            .get(RUN_KEY)
            .and_then(Value::as_str)
            .and_then(|text| text.parse().ok())
            .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
        let Some(mut run) = self
            .store()
            .bulk_run(id)
            .await
            .map_err(|error| internal::<S>(&error))?
        else {
            return Ok(());
        };
        if matches!(run.status, BulkStatus::Done | BulkStatus::Failed) {
            return Ok(());
        }
        let Some(person) = self
            .store()
            .person(run.person)
            .await
            .map_err(|error| internal::<S>(&error))?
        else {
            // The person who asked is gone: nobody to change things as.
            return self.end_run(&mut run, BulkStatus::Failed).await;
        };
        let actor = Actor { person, key: None };
        let acting = self.services.ctx_acting_as(actor.clone());
        let action: BulkAction = serde_json::from_value(run.action.clone())
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let selection = acting.selection(&actor, "", &action).await;
        let selection = match selection {
            Ok(selection) => selection,
            Err(AppError::Refused(_)) => {
                // What the action names is gone since it was asked for (the project, the label).
                return self.end_run(&mut run, BulkStatus::Failed).await;
            }
            Err(other) => return Err(other),
        };
        let mut maker = acting.line_maker().await?;
        let end = run
            .items
            .len()
            .min(usize::try_from(run.done).unwrap_or(usize::MAX) + BULK_STEP);
        run.status = BulkStatus::Running;
        while usize::try_from(run.done).unwrap_or(usize::MAX) < end {
            let at = run.done;
            let item = run.items[usize::try_from(at).unwrap_or(usize::MAX)];
            let target = BulkTarget {
                issue: item.issue.to_string(),
                version: item.version,
            };
            match acting.apply_item(&selection, &target, &mut maker).await {
                Ok(outcome) => match outcome.skipped {
                    None => run.changed += 1,
                    Some(reason) => run.skipped.push(BulkSkip {
                        issue: item.issue,
                        reason,
                    }),
                },
                Err(error) => {
                    // The store broke: the run stops where it is, and says so.
                    tracing::error!(cause = %error, "a change to many issues stopped");
                    return self.end_run(&mut run, BulkStatus::Failed).await;
                }
            }
            run.done += 1;
            if usize::try_from(run.done).unwrap_or(usize::MAX) >= run.items.len() {
                run.status = BulkStatus::Done;
                run.finished_at = Some(self.now());
            }
            if !self
                .store()
                .save_bulk_progress(&run, at)
                .await
                .map_err(|error| internal::<S>(&error))?
            {
                // Another worker took the same step and counted this issue first.
                return Ok(());
            }
        }
        if run.status != BulkStatus::Done {
            self.queue_next_step(id).await?;
        }
        Ok(())
    }

    /// Ends a run that cannot go on, with what it did so far.
    async fn end_run(&self, run: &mut BulkRun, status: BulkStatus) -> Result<(), AppError> {
        let at = run.done;
        run.status = status;
        run.finished_at = Some(self.now());
        self.store()
            .save_bulk_progress(run, at)
            .await
            .map_err(|error| internal::<S>(&error))?;
        Ok(())
    }

    /// Queues the next step of a run.
    async fn queue_next_step(&self, run: BulkRunId) -> Result<(), AppError> {
        let job = bb_core::id::JobId::generate(&mut self.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let now: Timestamp = self.now();
        self.store()
            .queue_job(
                job,
                &kind_name(kind::BULK_STEP)?,
                &serde_json::json!({ RUN_KEY: run.to_string() }),
                now,
                now,
            )
            .await
            .map_err(|error| internal::<S>(&error))
    }
}

/// How far a change to many issues that goes on in the background is. Only the person who asked
/// for it reads it.
///
/// # Errors
/// `resource.not_found` for a run that is not there or that somebody else asked for.
pub async fn bulk_run<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: BulkRunQuery,
) -> Result<Authorized<BulkRunView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    let run = ctx
        .store()
        .bulk_run(input.run)
        .await
        .map_err(|error| internal::<S>(&error))?
        .filter(|run| run.person == actor.person.id)
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    Ok(held.authorize(ctx.run_view(&actor, &run).await?))
}
