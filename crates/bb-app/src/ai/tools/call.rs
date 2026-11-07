//! Listing and calling tools: the only path from the assistant, an AI app or an agent to a use case
//! (AI-04, AI-05, ADR-0013).
//!
//! A call runs as the actor, so it can never do or read more than the person; the scope of the key
//! or of the app only narrows. What the AI gets back is text: the answer of the use case as the
//! person may read it, without what is kept from AI, with every text people wrote in a fence.

use bb_core::ErrorCode;
use bb_core::ai::{Tool, ToolCaller, ToolEffect, ToolOutcome, ToolScope};
use bb_core::message::Message;
use bb_core::sign_in::SecurityAction;
use bb_policy::{Authorized, Masked, Place};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use super::inputs::{GetIssueInput, GetMyWorkInput, ListProjectsInput, SearchIssuesInput};
use super::log::Called;
use super::registry::{REGISTRY, ToolSpec, tool_named};
use crate::access::Actor;
use crate::ai::context::{AuthorKind, Part, PeopleMode, render};
use crate::auth::{AppError, AuthStore, Ctx};

/// Whether a call runs the tool or only says what it would do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolMode {
    /// Run the tool. A tool that changes something is not run here: it is run by *Apply*, or by
    /// an app with the scope to change (the assistant and MCP packages).
    Run,
    /// Say what a tool that changes something would do, without saving.
    Preview,
}

/// One call of a tool.
pub struct ToolCallQuery {
    /// The secret of the session, or the access key, the call acts for.
    pub session_token: String,
    /// How the tool is called.
    pub caller: ToolCaller,
    /// The name an AI app gave itself, if it is one.
    pub app: Option<String>,
    /// What the app or the assistant was allowed, narrowing the person further.
    pub granted: ToolScope,
    /// How people are named in the answer.
    pub people: PeopleMode,
    /// The name of the tool.
    pub tool: String,
    /// Its input, as JSON.
    pub input: Value,
    /// Run it or preview it.
    pub mode: ToolMode,
}

/// What the AI reads back from a tool.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ToolAnswer {
    /// The text, with the fences of the texts people wrote.
    pub text: String,
}

impl Masked for ToolAnswer {}

/// Ask which tools a caller may use.
pub struct ToolListQuery {
    /// The secret of the session, or the access key.
    pub session_token: String,
    /// What the app or the assistant was allowed.
    pub granted: ToolScope,
}

/// One tool as an AI app is shown it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ToolListing {
    /// The name.
    pub name: &'static str,
    /// What it does, in plain words.
    pub description: String,
    /// Whether it reads or changes.
    pub effect: ToolEffect,
    /// The schema of its input.
    pub input_schema: Value,
}

/// The tools a caller may use.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ToolList {
    /// The tools.
    pub tools: Vec<ToolListing>,
}

impl Masked for ToolList {}

/// What the actor may do with tools: a key that may only look, or an app that may only look, lists
/// and calls no tool that changes anything (AI-04).
fn effective_scope(actor: &Actor, granted: ToolScope) -> ToolScope {
    if actor.may_change_own().is_err() {
        ToolScope::SeeOnly
    } else {
        granted
    }
}

/// The tools the caller may use, in the order of the registry.
///
/// # Errors
/// `auth.not-signed-in` without a person.
pub async fn list_tools<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ToolListQuery,
) -> Result<Authorized<ToolList>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let scope = effective_scope(&actor, input.granted);
    let tools = REGISTRY
        .iter()
        .filter(|spec| scope.covers(spec.scope()))
        .map(|spec| ToolListing {
            name: spec.name(),
            description: Message::from(spec.tool.description()).render(),
            effect: spec.effect,
            input_schema: spec.input_schema(),
        })
        .collect();
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    Ok(held.authorize(ToolList { tools }))
}

fn input_of<T: DeserializeOwned>(input: &Value) -> Result<T, AppError> {
    serde_json::from_value(input.clone()).map_err(|_| AppError::Refused(ErrorCode::InvalidRequest))
}

/// How a call ended, for the log.
fn outcome_of(result: &Result<String, AppError>) -> ToolOutcome {
    match result {
        Ok(_) => ToolOutcome::Done,
        Err(AppError::Refused(_) | AppError::Mistakes(_)) => ToolOutcome::Refused,
        Err(_) => ToolOutcome::Failed,
    }
}

/// Who the AI is told wrote the text of its own proposal.
const fn proposer(caller: ToolCaller) -> AuthorKind {
    match caller {
        ToolCaller::Assistant => AuthorKind::Assistant,
        ToolCaller::AiApp | ToolCaller::Agent => AuthorKind::Agent,
    }
}

/// Calls a tool as the person behind the session or key, and writes one row of the log of tool
/// calls whatever happens (never the content).
///
/// # Errors
/// `resource.not-found` for a tool that does not exist, `access.forbidden` for a tool the scope
/// does not allow (the security log gets a row), `request.invalid` for an input that is not the
/// tool's, and what the use case refuses: a search that names a field kept from AI is answered as a
/// search for a field that does not exist.
pub async fn call_tool<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    request: ToolCallQuery,
) -> Result<Authorized<ToolAnswer>, AppError> {
    let actor = ctx.actor(&request.session_token).await?;
    let Some(spec) = tool_named(&request.tool) else {
        ctx.note(SecurityAction::AiToolRefused, Some(actor.person.id), None)
            .await;
        return Err(AppError::Refused(ErrorCode::NotFound));
    };
    let bytes_in = request.input.to_string().len();
    let allowed = effective_scope(&actor, request.granted).covers(spec.scope());
    let result = if allowed {
        ctx.run_tool(&actor, spec, &request).await
    } else {
        ctx.note(SecurityAction::AiToolRefused, Some(actor.person.id), None)
            .await;
        Err(AppError::Refused(ErrorCode::Forbidden))
    };
    ctx.log_tool_call(Called {
        person: actor.person.id,
        caller: request.caller,
        app: request.app.as_deref(),
        tool: spec.tool,
        effect: spec.effect,
        bytes_in,
        bytes_out: result.as_ref().map_or(0, String::len),
        outcome: outcome_of(&result),
    })
    .await;
    let text = result?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    Ok(held.authorize(ToolAnswer { text }))
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Runs the tool for an actor whose scope allows it and writes what the AI reads.
    async fn run_tool(
        &self,
        actor: &Actor,
        spec: &ToolSpec,
        request: &ToolCallQuery,
    ) -> Result<String, AppError> {
        let token = request.session_token.as_str();
        let parts = match (spec.effect, request.mode) {
            (ToolEffect::Changes, ToolMode::Preview) => {
                let changes = self
                    .tool_preview(actor, token, spec.tool, &request.input)
                    .await?;
                let value = json!({
                    "would_change": changes.iter().map(|change| json!({
                        "field": change.field,
                        "before": change.before,
                        "after": change.after,
                    })).collect::<Vec<_>>(),
                });
                vec![Part {
                    name: "preview",
                    value,
                    author: proposer(request.caller),
                    project: None,
                }]
            }
            // A tool that changes something is run by *Apply* and by apps with the scope to change.
            (ToolEffect::Changes, ToolMode::Run) => {
                return Err(AppError::Refused(ErrorCode::Forbidden));
            }
            (ToolEffect::Reads, _) => {
                let withheld = self.withheld().await?;
                match spec.tool {
                    Tool::SearchIssues => {
                        let input: SearchIssuesInput = input_of(&request.input)?;
                        self.tool_search_issues(actor, token, &input, &withheld)
                            .await?
                    }
                    Tool::GetMyWork => {
                        let input: GetMyWorkInput = input_of(&request.input)?;
                        self.tool_get_my_work(actor, token, &input, &withheld)
                            .await?
                    }
                    Tool::GetIssue => {
                        let input: GetIssueInput = input_of(&request.input)?;
                        self.tool_get_issue(token, &input, &withheld).await?
                    }
                    Tool::ListProjects => {
                        let input: ListProjectsInput = input_of(&request.input)?;
                        self.tool_list_projects(token, &input, &withheld).await?
                    }
                    _ => return Err(AppError::Refused(ErrorCode::InvalidRequest)),
                }
            }
        };
        let people = self
            .people_as(actor.person.id, request.people, &parts)
            .await?;
        Ok(render(&parts, &people))
    }
}
