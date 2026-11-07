//! A question to the assistant (AI-10, M7-Q2): check it, find the place it works at, and run the
//! rounds with the provider, each of which may read through the tools.

use std::collections::HashMap;

use bb_core::ErrorCode;
use bb_core::ai::{
    ANSWER_TOKENS_MAX, ChatFrom, LOOKING_AT_CHARS_MAX, LOOKING_AT_MAX, LookingAtKind,
    QUESTION_CHARS_MAX, ROUNDS_MAX, TURN_CHARS_MAX, TURNS_MAX, ToolEffect, ToolScope,
};
use bb_core::id::ProjectId;
use bb_policy::{Authorized, Place};
use serde_json::Value;

use super::session_only;
use super::step::Tools;
use super::system::SYSTEM;
use super::{AssistantAnswer, ChatTurn, LookingAt};
use crate::access::Actor;
use crate::ai::context::PeopleMode;
use crate::ai::tools::{ToolList, ToolListQuery, list_tools};
use crate::ai::{AiPart, AiRequest, AiRole, AiToolCall, AiToolSpec, AiTurn, Withheld};
use crate::auth::{AppError, AuthStore, Ctx};

/// A question to the assistant.
pub struct AskAssistantQuery {
    /// The secret of the session, or the access key, the question is asked as.
    pub session_token: String,
    /// What the person typed.
    pub text: String,
    /// What the panel looks at; each can be taken away by the person.
    pub looking_at: Vec<LookingAt>,
    /// The turns so far, which the browser keeps (M7-Q6).
    pub turns: Vec<ChatTurn>,
}

fn invalid() -> AppError {
    AppError::Refused(ErrorCode::InvalidRequest)
}

/// The limits of a question, before anything is read or sent.
fn check(input: &AskAssistantQuery) -> Result<(), AppError> {
    let fits = |text: &str, most: usize| !text.trim().is_empty() && text.chars().count() <= most;
    let fine = fits(&input.text, QUESTION_CHARS_MAX)
        && input.turns.len() <= TURNS_MAX
        && input
            .turns
            .iter()
            .all(|turn| fits(&turn.text, TURN_CHARS_MAX))
        && input.looking_at.len() <= LOOKING_AT_MAX
        && input
            .looking_at
            .iter()
            .all(|each| fits(&each.value, LOOKING_AT_CHARS_MAX));
    if fine { Ok(()) } else { Err(invalid()) }
}

/// The sentence that tells the model what the panel looks at.
fn looking_at_sentence(looking_at: &[LookingAt]) -> String {
    if looking_at.is_empty() {
        return String::new();
    }
    let things: Vec<String> = looking_at
        .iter()
        .map(|each| match each.kind {
            LookingAtKind::Issue => format!("the issue {}", each.value),
            LookingAtKind::Project => format!("the project {}", each.value),
            LookingAtKind::Search => format!("a list found by the search \"{}\"", each.value),
        })
        .collect();
    format!("\n\n(The person is looking at {}.)", things.join(", "))
}

/// The turns of the request: the conversation so far, then the question. The browser may send them
/// in any order, so they are put into the order the providers ask for: the first is the person's,
/// and two of one speaker are one turn.
fn conversation(input: &AskAssistantQuery) -> Vec<AiTurn> {
    let mut turns: Vec<AiTurn> = Vec::new();
    let said = input
        .turns
        .iter()
        .map(|turn| (turn.from, turn.text.clone()))
        .chain([(
            ChatFrom::Person,
            format!("{}{}", input.text, looking_at_sentence(&input.looking_at)),
        )]);
    for (from, text) in said {
        let role = match from {
            ChatFrom::Person => AiRole::User,
            ChatFrom::Assistant => AiRole::Assistant,
        };
        match turns.last_mut() {
            Some(last) if last.role == role => last.parts.push(AiPart::Text(text)),
            None if role == AiRole::Assistant => {}
            _ => turns.push(AiTurn {
                role,
                parts: vec![AiPart::Text(text)],
            }),
        }
    }
    turns
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The projects the panel looks at, after each has been found as the person may find it: what
    /// the person cannot read, what does not exist and what is kept from AI answer the same
    /// `resource.not-found`. A search is checked when it runs, through the tool.
    async fn projects_looked_at(
        &self,
        actor: &Actor,
        looking_at: &[LookingAt],
        withheld: &Withheld,
    ) -> Result<Vec<ProjectId>, AppError> {
        let not_found = || AppError::Refused(ErrorCode::NotFound);
        let mut projects = Vec::new();
        for each in looking_at {
            let project = match each.kind {
                LookingAtKind::Issue => self.visible_issue(actor, &each.value).await?.0.project,
                LookingAtKind::Project => {
                    let found = self
                        .find_project(&each.value)
                        .await?
                        .ok_or_else(not_found)?;
                    let seen = self
                        .evaluation_of(actor, Place::Project(found.project.id))
                        .await?
                        .sees_project();
                    if !seen {
                        return Err(not_found());
                    }
                    found.project.id
                }
                LookingAtKind::Search => continue,
            };
            if withheld.hides_project(project) {
                return Err(not_found());
            }
            if !projects.contains(&project) {
                projects.push(project);
            }
        }
        Ok(projects)
    }
}

/// The tools as the model is shown them, and what each does.
fn tools_of(
    list: &Authorized<ToolList>,
) -> Result<(Vec<AiToolSpec>, HashMap<String, ToolEffect>), AppError> {
    let value = list
        .masked_value()
        .map_err(|error| AppError::Internal(error.to_string()))?;
    let mut specs = Vec::new();
    let mut effects = HashMap::new();
    for tool in value
        .get("tools")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let text = |member: &str| tool.get(member).and_then(Value::as_str).unwrap_or_default();
        let name = text("name").to_owned();
        if let Some(effect) = ToolEffect::from_code(text("effect")) {
            effects.insert(name.clone(), effect);
        }
        specs.push(AiToolSpec {
            name,
            description: text("description").to_owned(),
            schema: tool.get("input_schema").cloned().unwrap_or(Value::Null),
        });
    }
    Ok((specs, effects))
}

/// Asks the assistant a question as the person behind the session or key.
///
/// The person's rights, the places AI counts at, what is kept from AI and the agreement of an
/// outside provider are judged before anything is sent; the answer comes back with the changes it
/// proposes and saves nothing.
///
/// # Errors
/// `auth.access-key-not-allowed` for an access key, `request.invalid` for a question that is empty or over a limit, `resource.not-found` for
/// something the panel looks at that the person may not read, `ai.off` while AI does not count at a
/// place, the errors of the provider (`ai.agreement-needed`, `ai.not-set-up`, `ai.unavailable`),
/// and `ai.unavailable` when the model never gives an answer.
pub async fn ask_assistant<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AskAssistantQuery,
) -> Result<Authorized<AssistantAnswer>, AppError> {
    session_only(&input.session_token)?;
    let actor = ctx.actor(&input.session_token).await?;
    check(&input)?;
    // Off for the organization: nothing else is read.
    let mut state = ctx.ai_state_at(None).await?;
    if !state.counts() {
        return Err(ErrorCode::AiOff.into());
    }
    let withheld = ctx.withheld().await?;
    let projects = ctx
        .projects_looked_at(&actor, &input.looking_at, &withheld)
        .await?;
    for (at, project) in projects.iter().enumerate() {
        let here = ctx.ai_state_at(Some(*project)).await?;
        if !here.counts() {
            return Err(ErrorCode::AiOff.into());
        }
        // The first place the panel looks at decides which provider settings count.
        if at == 0 {
            state = here;
        }
    }
    let people = if state.names_sent(state.destination()?) {
        PeopleMode::AsSeen
    } else {
        PeopleMode::RoleOnly
    };
    let listed = list_tools(
        ctx,
        ToolListQuery {
            session_token: input.session_token.clone(),
            granted: ToolScope::SeeAndChange,
        },
    )
    .await?;
    let (specs, effects) = tools_of(&listed)?;
    let mut tools = Tools::new(
        ctx,
        &actor,
        &input.session_token,
        &input.text,
        people,
        effects,
    );
    let mut turns = conversation(&input);
    for round in 1..=ROUNDS_MAX {
        // The last round is given no tools, so the model has to answer with what it has.
        let last = round == ROUNDS_MAX;
        let answer = ctx
            .call_provider(
                &state,
                &AiRequest {
                    system: SYSTEM.to_owned(),
                    turns: turns.clone(),
                    tools: if last { Vec::new() } else { specs.clone() },
                    max_tokens: ANSWER_TOKENS_MAX,
                },
            )
            .await?;
        if answer.tool_calls.is_empty() {
            if answer.text.trim().is_empty() {
                break;
            }
            let held = ctx.evaluation_of(&actor, Place::Organization).await?;
            let html = bb_markdown::render(
                &answer.text,
                &bb_markdown::Options {
                    ai_answer: true,
                    ..bb_markdown::Options::default()
                },
            )
            .html;
            return Ok(held.authorize(AssistantAnswer {
                text: answer.text,
                html,
                proposals: tools.proposals,
            }));
        }
        if last {
            break;
        }
        turns.push(model_turn(&answer.text, &answer.tool_calls));
        let mut results = Vec::new();
        for call in &answer.tool_calls {
            results.push(AiPart::ToolResult {
                call: call.id.clone(),
                text: tools.call(call).await,
            });
        }
        turns.push(AiTurn {
            role: AiRole::User,
            parts: results,
        });
    }
    Err(ErrorCode::AiUnavailable.into())
}

/// The turn of the model that asked for tools.
fn model_turn(text: &str, calls: &[AiToolCall]) -> AiTurn {
    let mut parts = Vec::new();
    if !text.is_empty() {
        parts.push(AiPart::Text(text.to_owned()));
    }
    parts.extend(calls.iter().cloned().map(AiPart::ToolCall));
    AiTurn {
        role: AiRole::Assistant,
        parts,
    }
}
