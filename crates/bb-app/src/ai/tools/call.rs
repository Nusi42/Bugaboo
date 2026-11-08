//! Listing and calling tools: the only path from the assistant, an AI app or an agent to a use case
//! (AI-04, AI-05, ADR-0013).
//!
//! A call runs as the actor, so it can never do or read more than the person; the scope of the key
//! or of the app only narrows. The tool is found in the registry and run through the function the
//! registry names; nothing here knows one tool from another. What the AI gets back is text: the
//! answer of the use case as the person may read it, without what is kept from AI, with every text
//! people wrote in a fence.

use bb_core::ErrorCode;
use bb_core::ai::{ToolCaller, ToolEffect, ToolOutcome, ToolScope};
use bb_core::event::RecordKind;
use bb_core::message::Message;
use bb_core::sign_in::SecurityAction;
use bb_policy::{Authorized, Masked, Place};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::log::Called;
use super::preview::{ProposedChange, proposed_changes};
use super::registry::{Run, ToolSpec, registry};
use crate::access::Actor;
use crate::ai::context::{PeopleMode, render};
use crate::auth::{AppError, AuthStore, Ctx};

/// Whether a call runs the tool or only says what it would do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolMode {
    /// Run the tool. A tool that changes something is not run here: it is run by *Apply*, or by
    /// an app with the scope to change (the assistant and MCP packages).
    Run,
    /// Say what a tool that changes something would do, without saving. A tool that reads answers
    /// as it does when it runs.
    Preview,
    /// Save what the preview of a tool that changes something showed, with the use case of the
    /// tool, as the person, through the channel of the caller. Only a tool that changes something
    /// is applied.
    Apply,
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolAnswer {
    /// The text, with the fences of the texts people wrote.
    pub text: String,
    /// For a preview or an *Apply*: what the tool changes, as the person may read it. Empty for a
    /// tool that reads. The AI reads `text`; this is for the person, and the assistant
    /// reads it back from the masked answer.
    pub changes: Vec<ProposedChange>,
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
    if actor.key_may_change() {
        granted
    } else {
        ToolScope::SeeOnly
    }
}

/// Whether the tool is for this actor at all: a tool of agents only for an agent, and a tool about a
/// person's own time (M7-Q7) never for an agent.
fn offered_to(actor: &Actor, tool: bb_core::ai::Tool) -> bool {
    if actor.agent().is_some() {
        !tool.is_for_people()
    } else {
        !tool.is_for_agents()
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
    let tools = registry::<S>()
        .iter()
        .filter(|spec| scope.covers(spec.scope()) && offered_to(&actor, spec.tool))
        .map(|spec| ToolListing {
            name: spec.name(),
            description: Message::from(spec.tool.description()).render(),
            effect: spec.effect(),
            input_schema: spec.input_schema(),
        })
        .collect();
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    Ok(held.authorize(ToolList { tools }))
}

/// How a call ended, for the log.
fn outcome_of<T>(result: &Result<T, AppError>) -> ToolOutcome {
    match result {
        Ok(_) => ToolOutcome::Done,
        Err(AppError::Refused(_) | AppError::Mistakes(_)) => ToolOutcome::Refused,
        Err(_) => ToolOutcome::Failed,
    }
}

/// Calls a tool as the person behind the session or key, and writes one row of the log of tool
/// calls whatever happens (never the content), also for a name that is no tool.
///
/// # Errors
/// `resource.not-found` for a tool that does not exist, `access.forbidden` for a tool the scope
/// does not allow (the security log gets a row), `request.invalid` for an input that is not the
/// tool's, and what the use case refuses: a search that names a field kept from AI is answered as a
/// search for a field that does not exist, and an issue or a project kept from AI as one that is
/// not there.
pub async fn call_tool<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    request: ToolCallQuery,
) -> Result<Authorized<ToolAnswer>, AppError> {
    let actor = ctx.actor(&request.session_token).await?;
    if request.caller.acts_by_itself() {
        // The call was counted per grant and per person at the entry of MCP (`mcp_identity`).
        ctx.enter_mcp();
    }
    let bytes_in = request.input.to_string().len();
    let tools = registry::<S>();
    let found = tools.iter().find(|spec| spec.name() == request.tool);
    let result = match found {
        None => {
            ctx.note(SecurityAction::AiToolRefused, Some(actor.person.id), None)
                .await;
            Err(AppError::Refused(ErrorCode::NotFound))
        }
        Some(spec)
            if !effective_scope(&actor, request.granted).covers(spec.scope())
                || !offered_to(&actor, spec.tool) =>
        {
            ctx.note(SecurityAction::AiToolRefused, Some(actor.person.id), None)
                .await;
            Err(AppError::Refused(ErrorCode::Forbidden))
        }
        Some(spec) => ctx.run_tool(&actor, spec, &request).await,
    };
    let outcome = if found.is_none() {
        ToolOutcome::Unknown
    } else {
        outcome_of(&result)
    };
    ctx.log_tool_call(Called {
        person: actor.person.id,
        caller: request.caller,
        app: request.app.as_deref(),
        tool: found.map(|spec| spec.tool),
        effect: found.map(ToolSpec::effect),
        bytes_in,
        bytes_out: result.as_ref().map_or(0, |(text, _)| text.len()),
        outcome,
    })
    .await;
    let (text, changes) = result?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    Ok(held.authorize(ToolAnswer { text, changes }))
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The answer for a stale version, as the actor may read it: judged with the actor (an agent
    /// with its narrowing, an app with its grant), not with the person behind it, so the issue must be
    /// one the actor reaches and the fields are those it may see; and without a field that is kept
    /// from AI.
    async fn conflict_for_ai(
        &self,
        actor: &Actor,
        stale: &crate::editing::Stale,
        withheld: &crate::ai::Withheld,
    ) -> AppError {
        let is_issue = stale.record.kind == bb_core::event::EntityKind::from(RecordKind::Issue);
        let issue = match bb_core::id::IssueId::from_bytes(stale.record.id.to_bytes()) {
            Ok(id) if is_issue => self.seen_issue_by_id(actor, id).await,
            _ => Ok(None),
        };
        let (issue, held) = match issue {
            Ok(Some(found)) => found,
            Ok(None) => return AppError::Refused(ErrorCode::NotFound),
            Err(error) => return error,
        };
        let hidden = |field: &str| {
            bb_core::fields::FieldKey::from_code(field).is_some_and(|key| {
                withheld.hides_field(key)
                    || held.field_level_for(Some(issue.issue_type), key)
                        < bb_core::access::FieldLevel::CanSee
            })
        };
        match self.conflict_of(stale).await {
            AppError::Conflict(mut conflict) => {
                conflict.fields.retain(|field| !hidden(field));
                conflict.current.retain(|field, _| !hidden(field));
                // Who changed it was judged for the person; an agent is told what changed, not by
                // whom.
                conflict.changed_by = None;
                AppError::Conflict(conflict)
            }
            other => other,
        }
    }
}

/// A word that is new for every answer: both marker lines of a fence carry it, so a text cannot
/// write a closing line before it knows the answer.
fn nonce() -> Result<String, AppError> {
    let mut bytes = [0_u8; 4];
    getrandom::fill(&mut bytes).map_err(|error| AppError::Internal(error.to_string()))?;
    let [a, b, c, d] = bytes;
    Ok(format!("{a:02x}{b:02x}{c:02x}{d:02x}"))
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Runs the tool for an actor whose scope allows it, through the function the registry names,
    /// and writes what the AI reads.
    async fn run_tool(
        &self,
        actor: &Actor,
        spec: &ToolSpec<S>,
        request: &ToolCallQuery,
    ) -> Result<(String, Vec<ProposedChange>), AppError> {
        // An AI app that was given the scope to change runs a tool that changes something at once, as
        // its own act: that is *Apply* for it. The assistant only proposes, and *Apply* is the
        // person's.
        let mode = if request.caller.acts_by_itself()
            && request.mode == ToolMode::Run
            && spec.effect() == ToolEffect::Changes
        {
            ToolMode::Apply
        } else {
            request.mode
        };
        let refused = match mode {
            // A tool that changes something is run by *Apply* and by apps with the scope to change.
            ToolMode::Run => spec.effect() == ToolEffect::Changes,
            ToolMode::Preview => false,
            ToolMode::Apply => spec.effect() != ToolEffect::Changes,
        };
        if refused {
            return Err(AppError::Refused(ErrorCode::Forbidden));
        }
        // What is kept from AI is read first, for a preview as for a read.
        let withheld = self.withheld().await?;
        let run = Run {
            actor,
            token: request.session_token.as_str(),
            input: &request.input,
            withheld: &withheld,
            caller: request.caller,
        };
        let outcome = if mode == ToolMode::Apply {
            // What an agent changes counts against its limits, before anything is written.
            if actor.agent().is_some() {
                self.expect_change();
            }
            let applied = spec
                .apply(self, run)
                .ok_or(AppError::Refused(ErrorCode::Forbidden))?;
            // What the use case records says it was made through the assistant or an AI app.
            self.act_through(request.caller.channel());
            let result = applied.await;
            self.settle_agent_change(result.is_ok());
            result
        } else {
            spec.run(self, run).await
        };
        let parts = match outcome {
            Ok(parts) => parts,
            // A change made from an old version tells what changed, as the actor may read it and
            // without what is kept from AI.
            Err(AppError::Stale(stale)) => {
                return Err(self.conflict_for_ai(actor, &stale, &withheld).await);
            }
            Err(error) => return Err(error),
        };
        let changes = if spec.effect() == ToolEffect::Changes {
            proposed_changes(&parts)
        } else {
            Vec::new()
        };
        let people = self
            .people_as(actor.person.id, request.people, &parts)
            .await?;
        Ok((render(&parts, &people, &nonce()?), changes))
    }
}
