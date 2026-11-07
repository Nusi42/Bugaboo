//! The run of a session of an AI app (AI-38, ADR-0042 decision 5, ADR-0029 decision 7): a session
//! acting for a person is one run, opened when it starts and closed when it ends or has been idle
//! for [`bb_core::oauth::SESSION_IDLE_MINUTES`], through the port of runs. Every change the session
//! makes carries the run (`ToolCallQuery::run`), so its changes are listed together and undone in
//! one step.
//!
//! An agent that signs in opens no run of its own: its calls belong to the run of its attempt.

use bb_core::event::RecordKind;
use bb_core::id::RunId;
use bb_core::run::{RunActor, RunKind};
use serde_json::json;

use super::ctx::Ctx;
use super::error::{AppError, Stops, change_failed, internal};
use super::events::{by, entity, set_to};
use super::oauth_types::{AppSession, EndAppRuns, StartAppSession};
use super::store::AuthStore;
use super::types::Done;
use crate::access::CredentialSource;
use crate::runs::{RunStore, RunsOf};

/// Opens the run of a session of an AI app, for the person and the app behind the token (or the
/// access key). An agent gets no run: `run` is empty.
///
/// # Errors
/// `auth.not-signed-in` for a token or key that does not work.
pub async fn start_app_session<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: StartAppSession,
) -> Result<AppSession, AppError> {
    let actor = ctx.actor(&input.token).await?;
    if actor.agent().is_some() {
        return Ok(AppSession { run: None });
    }
    let app = match actor.key.as_ref().map(|key| &key.source) {
        Some(CredentialSource::App { name, .. }) => Some(name.clone()),
        _ => None,
    };
    let run =
        RunId::generate(&mut ctx.ids()).map_err(|error| AppError::Internal(error.to_string()))?;
    let who = RunActor {
        person: actor.person.id,
        app: app.clone(),
        agent: None,
    };
    RunsOf::new(ctx, &actor, &input.token)
        .open_run(run, RunKind::AppSession, &who, ctx.now())
        .await?;
    // The fact that a session of an app began is its own record: who answers for it and which app.
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::ChangeRun, run),
        vec![set_to("app", json!(app))],
    )?;
    ctx.store()
        .record_change(&event, |_transaction| Box::pin(async { Ok(()) }))
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    Ok(AppSession { run: Some(run) })
}

/// Closes the runs of sessions that ended or were idle too long. The server calls it with the runs
/// it opened; a run that is not a session of an app, or is over already, is left alone.
///
/// # Errors
/// An internal error when the store fails.
pub async fn end_app_runs<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: EndAppRuns,
) -> Result<Done, AppError> {
    for run in input.runs {
        let Some(found) = ctx
            .store()
            .many_changes_run(run)
            .await
            .map_err(|error| internal::<S>(&error))?
            .filter(|found| found.kind == RunKind::AppSession && !found.status.is_over())
        else {
            continue;
        };
        let event = ctx.event(
            by(found.person),
            entity(RecordKind::ChangeRun, run),
            vec![set_to("ended", json!(input.how.code()))],
        )?;
        ctx.store()
            .record_change(&event, |_transaction| Box::pin(async { Ok(()) }))
            .await
            .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
        ctx.end_many_changes_run(
            run,
            input.how.status(),
            bb_core::event::Actor::Person(found.person),
        )
        .await?;
    }
    Ok(Done)
}
