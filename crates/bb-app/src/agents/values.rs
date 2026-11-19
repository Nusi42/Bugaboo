//! The values of an agent that its person changes on the version they saw (ADR-0029, decision 9
//! and amendment M9): its limits, and the switches *May hand over to my other agents* and *May undo
//! its own runs*, off when the agent is made.

use bb_core::ErrorCode;
use bb_core::agent::AgentState;
use bb_core::event::RecordKind;
use bb_core::id::{AgentId, UserId};
use bb_policy::{Authorized, Place};
use serde_json::json;

use super::manage::{agent, counted, stale_agent};
use super::types::{AgentQuery, AgentView, SetAgentLimits, SetAgentSwitches};
use crate::auth::{AppError, AuthStore, Ctx, Stops, change_failed, entity};

/// The end of the change of a value: it is written only on the version the person saw.
fn written<S: AuthStore>(at_version: bool) -> Result<(), S::Error> {
    if at_version {
        Ok(())
    } else {
        Err(S::aborted())
    }
}

/// What a change of a value that failed answers: the agent changed meanwhile, or the failure.
fn refused<S: AuthStore>(error: &S::Error, id: AgentId, person: UserId) -> AppError {
    change_failed::<S>(error, Stops::both(ErrorCode::VersionChanged)).about(
        &entity(RecordKind::Agent, id),
        Place::Organization,
        person,
    )
}

/// Gives the agent other limits, each at least one and at most what the organization allows. Only
/// the person responsible may.
///
/// # Errors
/// `resource.not-found`, `request.invalid` for limits that do not fit, `edit.version-changed` when the
/// agent changed meanwhile.
pub async fn set_agent_limits<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SetAgentLimits,
) -> Result<Authorized<AgentView>, AppError> {
    let record = ctx.active(&input.session_token).await?;
    let person = record.person.id;
    let found = ctx.agent_to_handle(person, false, input.id).await?;
    if found.state == AgentState::Stopped || !input.limits.fit_within(ctx.agent_maximums().await?) {
        return Err(ErrorCode::InvalidRequest.into());
    }
    if found.version != input.version {
        return Err(stale_agent(person, found.id));
    }
    let before = found.limits();
    let now = input.limits;
    let change = |field: &str, was: i64, to: i64| bb_core::event::FieldChange {
        field: field.to_owned(),
        before: Some(json!(was)),
        after: Some(json!(to)),
    };
    let event = counted(
        ctx,
        person,
        found.id,
        vec![
            change(
                "changes_a_minute",
                before.changes_a_minute,
                now.changes_a_minute,
            ),
            change("issues_at_once", before.issues_at_once, now.issues_at_once),
            change(
                "attempts_on_one_issue",
                before.attempts_on_one_issue,
                now.attempts_on_one_issue,
            ),
        ],
        found.version,
    )?;
    let (id, version) = (found.id, found.version);
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                written::<S>(S::set_agent_limits(transaction, id, now, version).await?)
            })
        })
        .await
        .map_err(|error| refused::<S>(&error, id, person))?;
    agent(
        ctx,
        AgentQuery {
            session_token: input.session_token,
            id,
        },
    )
    .await
}

/// Switches *May hand over to my other agents* and *May undo its own runs* of the agent on or off
/// (ADR-0029, amendment M9). Only the person responsible may: the switches widen what the agent does
/// for them, so an administrator does not.
///
/// # Errors
/// `resource.not-found`, `request.invalid` for a stopped agent, `edit.version-changed` when the
/// agent changed meanwhile.
pub async fn set_agent_switches<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SetAgentSwitches,
) -> Result<Authorized<AgentView>, AppError> {
    let record = ctx.active(&input.session_token).await?;
    let person = record.person.id;
    let found = ctx.agent_to_handle(person, false, input.id).await?;
    if found.state == AgentState::Stopped {
        return Err(ErrorCode::InvalidRequest.into());
    }
    if found.version != input.version {
        return Err(stale_agent(person, found.id));
    }
    let (before, now) = (found.switches(), input.switches);
    let change = |field: &str, was: bool, to: bool| bb_core::event::FieldChange {
        field: field.to_owned(),
        before: Some(json!(was)),
        after: Some(json!(to)),
    };
    let event = counted(
        ctx,
        person,
        found.id,
        vec![
            change("may_hand_over", before.may_hand_over, now.may_hand_over),
            change("may_undo", before.may_undo, now.may_undo),
        ],
        found.version,
    )?;
    let (id, version) = (found.id, found.version);
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                written::<S>(S::set_agent_switches(transaction, id, now, version).await?)
            })
        })
        .await
        .map_err(|error| refused::<S>(&error, id, person))?;
    agent(
        ctx,
        AgentQuery {
            session_token: input.session_token,
            id,
        },
    )
    .await
}
