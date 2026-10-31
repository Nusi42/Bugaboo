//! A change to many issues that goes on in the background (ISS-09, ADR-0017): reading how far it
//! is, and doing a step of it.
//!
//! A step changes up to [`MANY_CHANGES_STEP`] issues, each by the use case of a single change, as the
//! person who asked (the job has no session; the run keeps the person). The progress is saved after
//! every issue, so a worker that stops loses nothing: the next one goes on at the first issue that
//! is not counted. The change and the saving of the progress are two writes, so the event of every
//! change carries the item it was made for (`ManyChangeRun::item_reason`): a worker that stopped between
//! the two finds that event when it goes on and counts the issue as changed, instead of judging it
//! again and reporting that it changed since the preview. A run that was stopped (by the person, by
//! somebody who manages people, or by a sign-out everywhere or a new password) is looked at before
//! each issue and changes nothing more. When issues are left, the step queues the next one.

use bb_core::ErrorCode;
use bb_core::event::{Actor as EventActor, RecordKind};
use bb_core::id::ManyChangeRunId;
use bb_core::many_changes::{
    MANY_CHANGES_STEP, ManyChangeRun, ManyChangeSkip, ManyChangeStatus, RunsToEnd,
};
use bb_policy::{Authorized, Permission, Place};
use jiff::Timestamp;
use serde_json::Value;

use super::types::{
    ManyChangeAction, ManyChangeRunQuery, ManyChangeRunStatus, ManyChangeRunView,
    ManyChangeSkipped, ManyChangeTarget, RUN_SKIPPED_MAX,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, by, entity, internal, set_to};
use crate::jobs::JobKind;
use crate::member;

/// The member of the payload of a step that names the run.
const RUN_KEY: &str = "run";

/// What a step of a run is queued with: the run it is a step of.
pub(crate) fn step_payload(run: ManyChangeRunId) -> Value {
    serde_json::json!({ RUN_KEY: run.to_string() })
}

/// The run that the payload of a step names.
fn run_named(payload: &Value) -> Result<ManyChangeRunId, AppError> {
    payload
        .get(RUN_KEY)
        .and_then(Value::as_str)
        .and_then(|text| text.parse().ok())
        .ok_or(AppError::Refused(ErrorCode::InvalidRequest))
}

impl<S: AuthStore> Ctx<'_, S> {
    /// How far a run is, as the person who asked for it reads it: the issues it left alone as
    /// that person sees them.
    pub(super) async fn run_view(
        &self,
        actor: &Actor,
        run: &ManyChangeRun,
    ) -> Result<ManyChangeRunView, AppError> {
        let mut maker = self.line_maker().await?;
        let mut skipped = Vec::new();
        for left in run.skipped.iter().take(RUN_SKIPPED_MAX) {
            let issue = match self.issues_named(&[left.issue]).await?.first() {
                Some(found) => self.line_of(actor, &mut maker, found).await?,
                None => None,
            };
            skipped.push(ManyChangeSkipped {
                issue,
                reason: left.reason,
            });
        }
        let count = |number: usize| u32::try_from(number).unwrap_or(u32::MAX);
        Ok(ManyChangeRunView {
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
    pub(crate) async fn run_many_changes_step(&self, payload: &Value) -> Result<(), AppError> {
        let id = run_named(payload)?;
        let Some(mut run) = self
            .store()
            .many_changes_run(id)
            .await
            .map_err(|error| internal::<S>(&error))?
        else {
            return Ok(());
        };
        if run.status.is_over() {
            return Ok(());
        }
        let Some(person) = self
            .store()
            .person(run.person)
            .await
            .map_err(|error| internal::<S>(&error))?
        else {
            // The person who asked is gone: nobody to change things as.
            return self
                .end_many_changes_run(id, ManyChangeStatus::Failed, EventActor::System)
                .await;
        };
        let actor = Actor { person, key: None };
        let acting = self.services.ctx_acting_as(actor.clone());
        let action: ManyChangeAction = serde_json::from_value(run.action.clone())
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let selection = acting.selection(&actor, "", &action).await;
        let selection = match selection {
            Ok(selection) => selection,
            Err(AppError::Refused(_)) => {
                // What the action names is gone since it was asked for (the project, the label).
                return self
                    .end_many_changes_run(id, ManyChangeStatus::Failed, EventActor::System)
                    .await;
            }
            Err(other) => return Err(other),
        };
        let mut maker = acting.line_maker().await?;
        let end = run
            .items
            .len()
            .min(usize::try_from(run.done).unwrap_or(usize::MAX) + MANY_CHANGES_STEP);
        run.status = ManyChangeStatus::Running;
        let first = run.done;
        while usize::try_from(run.done).unwrap_or(usize::MAX) < end {
            // A run that was stopped meanwhile changes nothing more, not even the issue at hand.
            if self.run_is_over(id).await? {
                return Ok(());
            }
            let at = run.done;
            let item = run.items[usize::try_from(at).unwrap_or(usize::MAX)];
            let target = ManyChangeTarget {
                issue: item.issue.to_string(),
                version: item.version,
            };
            // Only the issue the step begins at can have been changed by a worker that stopped
            // before it counted the change: its event says so, and it counts as changed.
            let written = at == first && self.item_was_written(&run, at).await?;
            let left_alone = if written {
                None
            } else {
                acting.set_reason(Some(run.item_reason(at)));
                let applied = acting.apply_item(&selection, &target, &mut maker).await;
                acting.set_reason(None);
                match applied {
                    Ok(outcome) => outcome.skipped,
                    // The database was busy: the step fails and the queue tries it again, from the
                    // issue at hand (the progress up to it is saved).
                    Err(busy) if busy.is_busy() => return Err(busy),
                    Err(error) => {
                        // The store broke: the run stops where it is, and says so.
                        tracing::error!(cause = %error, "a change to many issues stopped");
                        return self
                            .end_many_changes_run(id, ManyChangeStatus::Failed, EventActor::System)
                            .await;
                    }
                }
            };
            match left_alone {
                None => run.changed += 1,
                Some(reason) => run.skipped.push(ManyChangeSkip {
                    issue: item.issue,
                    reason,
                }),
            }
            run.done += 1;
            if usize::try_from(run.done).unwrap_or(usize::MAX) >= run.items.len() {
                run.status = ManyChangeStatus::Done;
                run.finished_at = Some(self.now());
            }
            if !self
                .store()
                .save_many_changes_progress(&run, at)
                .await
                .map_err(|error| internal::<S>(&error))?
            {
                // Another worker took the same step and counted this issue first.
                return Ok(());
            }
        }
        self.after_step(id, run.status).await
    }

    /// What follows a step: the next step is queued, or the run is announced as done.
    async fn after_step(
        &self,
        id: ManyChangeRunId,
        status: ManyChangeStatus,
    ) -> Result<(), AppError> {
        if status == ManyChangeStatus::Done {
            self.announce_run_done(id).await
        } else {
            self.queue_next_step(id).await
        }
    }

    /// Writes the event that a run is done, so the person who asked for it is told (M3-F125): the
    /// status was counted up by the step that changed the last issue, which writes no event of the
    /// run itself.
    async fn announce_run_done(&self, id: ManyChangeRunId) -> Result<(), AppError> {
        let event = self.event(
            EventActor::System,
            entity(RecordKind::ChangeRun, id),
            vec![set_to(
                member::run::STATUS,
                serde_json::json!(ManyChangeStatus::Done.code()),
            )],
        )?;
        self.store()
            .record_change(&event, |_| Box::pin(async { Ok(()) }))
            .await
            .map_err(|error| internal::<S>(&error))
    }

    /// Whether the run is over (done, failed or stopped) now, as the store has it.
    async fn run_is_over(&self, id: ManyChangeRunId) -> Result<bool, AppError> {
        self.store()
            .many_changes_run_is_over(id)
            .await
            .map_err(|error| internal::<S>(&error))
    }

    /// Whether a worker that stopped wrote the change of the issue at `at` before it counted it.
    async fn item_was_written(&self, run: &ManyChangeRun, at: i64) -> Result<bool, AppError> {
        self.store()
            .event_with_reason(&run.item_reason(at), run.created_at)
            .await
            .map_err(|error| internal::<S>(&error))
    }

    /// Ends the run of a step that gave up (its job is on the dead-letter list) as failed, with
    /// what it did so far (saved after each issue), so the person who asked reads an end and not a run that never ends.
    ///
    /// # Errors
    /// The error of the store.
    pub(crate) async fn fail_many_changes_run(&self, payload: &Value) -> Result<(), AppError> {
        let id = run_named(payload)?;
        if self.run_is_over(id).await? {
            return Ok(());
        }
        self.end_many_changes_run(id, ManyChangeStatus::Failed, EventActor::System)
            .await
    }

    /// Ends a run that is not over, as failed or stopped, with an event of the run. The one place
    /// that ends one run: the steps that cannot go on, the dead-letter list and the person who
    /// stops it all come here, `who` being the actor of the event.
    pub(crate) async fn end_many_changes_run(
        &self,
        id: ManyChangeRunId,
        status: ManyChangeStatus,
        who: EventActor,
    ) -> Result<(), AppError> {
        let now = self.now();
        let event = self.event(
            who,
            entity(RecordKind::ChangeRun, id),
            vec![set_to(
                member::run::STATUS,
                serde_json::json!(status.code()),
            )],
        )?;
        self.store()
            .record_change(&event, move |transaction| {
                Box::pin(async move {
                    S::end_many_changes_runs(transaction, &RunsToEnd::One(id), status, now)
                        .await
                        .map(|_| ())
                })
            })
            .await
            .map_err(|error| internal::<S>(&error))
    }

    /// Queues the next step of a run.
    async fn queue_next_step(&self, run: ManyChangeRunId) -> Result<(), AppError> {
        let job = bb_core::id::JobId::generate(&mut self.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let now: Timestamp = self.now();
        self.store()
            .queue_job(
                job,
                &JobKind::ManyChangesStep.name()?,
                &step_payload(run),
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
/// `resource.not-found` for a run that is not there or that somebody else asked for.
pub async fn many_changes_run<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ManyChangeRunQuery,
) -> Result<Authorized<ManyChangeRunView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    let run = ctx
        .store()
        .many_changes_run(input.run)
        .await
        .map_err(|error| internal::<S>(&error))?
        .filter(|run| run.person == actor.person.id)
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    Ok(held.authorize(ctx.run_view(&actor, &run).await?))
}

/// Stops a change to many issues that goes on in the background: what was changed so far stays,
/// the rest is not touched. The person who asked for the run may stop it, and so may whoever holds
/// *Manage people* for the organization (acting on the things of another person). The answer is the
/// status and nothing more: the counts are of the issues the person who asked named (PRIV-13).
/// Stopping a run that is over changes nothing.
///
/// # Errors
/// `resource.not-found` for a run that is not there or that somebody else asked for and the person
/// does not manage people.
pub async fn stop_many_changes_run<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ManyChangeRunQuery,
) -> Result<Authorized<ManyChangeRunStatus>, AppError> {
    // A key cannot start a run that goes on in the background (M3-F121), so it has none to stop.
    let actor = ctx.session_actor(&input.session_token).await?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    let run = ctx
        .store()
        .many_changes_run(input.run)
        .await
        .map_err(|error| internal::<S>(&error))?
        .filter(|run| run.person == actor.person.id || held.allows(Permission::ManagePeople))
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    if matches!(
        run.status,
        ManyChangeStatus::Queued | ManyChangeStatus::Running
    ) {
        ctx.end_many_changes_run(run.id, ManyChangeStatus::Stopped, by(actor.person.id))
            .await?;
    }
    let status = ctx
        .store()
        .many_changes_run(run.id)
        .await
        .map_err(|error| internal::<S>(&error))?
        .map_or(run.status, |run| run.status);
    Ok(held.authorize(ManyChangeRunStatus { id: run.id, status }))
}
