//! The tools of programs (ADR-0029, amendment M9): `ask_for_agents` and `collect_agent_keys`. A
//! program with a person's access key or an app's grant asks for agents and, once the person
//! approved in the browser, collects their keys once. They only call the use cases of requests for
//! agents; neither makes a key without the person's approval (`Tool::is_for_programs`).

use bb_core::ErrorCode;
use bb_core::ai::AuthorKind;
use serde_json::Value;

use super::inputs::{AskForAgentsInput, CollectAgentKeysInput};
use super::read::masked;
use super::registry::{Boxed, Run};
use crate::agents::{
    AgentAsked, AgentRequestView, AskForAgents, CollectAgentKeys, ask_for_agents,
    collect_agent_keys,
};
use crate::ai::context::Part;
use crate::auth::{AppError, AuthStore, Ctx};

/// What the AI reads: one part, written by the person through their program.
fn part(name: &'static str, value: Value) -> Vec<Part> {
    vec![Part {
        name,
        value,
        author: AuthorKind::Member,
        project: None,
    }]
}

fn ask_request(run: &Run<'_>) -> Result<AskForAgents, AppError> {
    let input: AskForAgentsInput = run.input()?;
    Ok(AskForAgents {
        session_token: run.token.to_owned(),
        reason: input.reason,
        agents: input
            .agents
            .into_iter()
            .map(|asked| AgentAsked {
                name: asked.name,
                purpose: asked.purpose,
                key_days: asked.key_days,
            })
            .collect(),
    })
}

fn collect_request(run: &Run<'_>) -> Result<CollectAgentKeys, AppError> {
    let input: CollectAgentKeysInput = run.input()?;
    Ok(CollectAgentKeys {
        session_token: run.token.to_owned(),
        id: input
            .request
            .parse()
            .map_err(|_| AppError::Refused(ErrorCode::NotFound))?,
    })
}

/// What the program reads of a view: its JSON.
fn value_of<T: serde::Serialize>(view: &T) -> Result<Value, AppError> {
    serde_json::to_value(view).map_err(|error| AppError::Internal(error.to_string()))
}

/// `ask_for_agents`, asked about: what the request would hold, with nothing saved.
pub(super) fn preview_ask_for_agents<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let (request, _) = ctx.preview_ask_for_agents(&ask_request(&run)?).await?;
        let view = AgentRequestView::of(&request, ctx.now());
        Ok(part("would_ask", value_of(&view)?))
    })
}

/// `ask_for_agents`, made: the request, which the person approves or declines in the browser.
pub(super) fn apply_ask_for_agents<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let view = ask_for_agents(ctx, ask_request(&run)?).await?;
        Ok(part("request", masked(&view)?))
    })
}

/// `collect_agent_keys`, asked about: where the request stands, and which keys a pickup would
/// make, with nothing made.
pub(super) fn preview_collect_agent_keys<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let collected = ctx
            .preview_collect_agent_keys(&collect_request(&run)?)
            .await?;
        Ok(part("would_collect", value_of(&collected)?))
    })
}

/// `collect_agent_keys`, made: the keys, once, or where the request stands.
pub(super) fn apply_collect_agent_keys<'a, S: AuthStore>(
    ctx: &'a Ctx<'_, S>,
    run: Run<'a>,
) -> Boxed<'a, Result<Vec<Part>, AppError>> {
    Box::pin(async move {
        let collected = collect_agent_keys(ctx, collect_request(&run)?).await?;
        Ok(part("collected", masked(&collected)?))
    })
}
