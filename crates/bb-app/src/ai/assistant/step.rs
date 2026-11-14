//! One call of a tool inside a question: through the registry, as a preview, and never more than
//! the named number of times. A tool that reads answers; a tool that changes something becomes a
//! proposal, or is turned away with a sentence the model reads.

use std::collections::HashMap;

use bb_core::ai::{
    LookUps, PROPOSAL_INPUT_BYTES_MAX, PROPOSALS_MAX, ToolCaller, ToolEffect, ToolScope,
};
use bb_core::message::{self, Message};
use bb_policy::{Authorized, Place};
use serde_json::Value;

use super::system::{
    CHANGED, COULD_NOT_RUN, LIMIT_REACHED, NO_PERSON, NOT_VALID, NOTHING, PROPOSED, TOO_MANY,
};
use super::{Proposal, reference};
use crate::access::Actor;
use crate::ai::AiToolCall;
use crate::ai::context::PeopleMode;
use crate::ai::tools::{ToolAnswer, ToolCallQuery, ToolMode, call_tool};
use crate::auth::{AppError, AuthStore, Ctx};

/// The tools of one question: who asks, how people are named, what the tools do, and what came of
/// the calls so far.
pub(super) struct Tools<'a, S: AuthStore> {
    ctx: &'a Ctx<'a, S>,
    actor: &'a Actor,
    token: &'a str,
    question: &'a str,
    people: PeopleMode,
    effects: HashMap<String, ToolEffect>,
    look_ups: LookUps,
    used: usize,
    pub(super) proposals: Vec<Proposal>,
}

impl<'a, S: AuthStore> Tools<'a, S> {
    pub(super) fn new(
        ctx: &'a Ctx<'a, S>,
        actor: &'a Actor,
        token: &'a str,
        question: &'a str,
        people: PeopleMode,
        effects: HashMap<String, ToolEffect>,
        look_ups: LookUps,
    ) -> Self {
        Self {
            ctx,
            actor,
            token,
            question,
            people,
            effects,
            look_ups,
            used: 0,
            proposals: Vec::new(),
        }
    }

    /// What the model reads of this call.
    pub(super) async fn call(&mut self, call: &AiToolCall) -> String {
        if self.look_ups.calls().is_some_and(|most| self.used >= most) {
            return LIMIT_REACHED.to_owned();
        }
        self.used += 1;
        // Only previews: a tool that changes something is never run from here, so nothing the
        // model asks for can be saved without *Apply*.
        let answered = call_tool(
            self.ctx,
            ToolCallQuery {
                session_token: self.token.to_owned(),
                caller: ToolCaller::Assistant,
                app: None,
                granted: ToolScope::SeeAndChange,
                people: self.people,
                tool: call.name.clone(),
                input: call.arguments.clone(),
                mode: ToolMode::Preview,
            },
        )
        .await;
        match answered {
            Ok(answer) => match self.effects.get(&call.name) {
                Some(ToolEffect::Changes) => self.proposed(call, &answer).await,
                _ => read_text(&answer),
            },
            Err(error) => sentence_of(&error),
        }
    }

    /// Where a call works: the project of the issue it names, else the organization.
    async fn place_of(&self, input: &Value) -> Place {
        if let Some(address) = input.get("issue").and_then(Value::as_str)
            && let Ok((issue, _)) = self.ctx.visible_issue(self.actor, address).await
        {
            return Place::Project(issue.project);
        }
        Place::Organization
    }

    async fn proposed(&mut self, call: &AiToolCall, answer: &Authorized<ToolAnswer>) -> String {
        // *Apply* takes no input over this size back, so a proposal of one could never be applied.
        if call.arguments.to_string().len() > PROPOSAL_INPUT_BYTES_MAX {
            return NOT_VALID.to_owned();
        }
        let Some(shown) = split(answer) else {
            return COULD_NOT_RUN.to_owned();
        };
        if shown.changes.is_empty() {
            return NOTHING.to_owned();
        }
        let place = self.place_of(&call.arguments).await;
        match self
            .ctx
            .assignment_was_asked_for(
                self.actor,
                self.question,
                place,
                &call.arguments,
                &shown.changes,
            )
            .await
        {
            Ok(true) => {}
            Ok(false) => return NO_PERSON.to_owned(),
            Err(error) => return sentence_of(&error),
        }
        let already = self
            .proposals
            .iter()
            .any(|each| each.tool == call.name && each.input == call.arguments);
        if !already {
            if self.proposals.len() >= PROPOSALS_MAX {
                return TOO_MANY.to_owned();
            }
            let Some(reference) = reference::create(
                &self.ctx.services.links,
                self.actor.person.id,
                self.ctx.now(),
                &call.name,
                &call.arguments,
            ) else {
                return COULD_NOT_RUN.to_owned();
            };
            let words = self
                .ctx
                .words_of(self.actor, &call.arguments, &shown.changes)
                .await;
            self.proposals.push(Proposal {
                reference,
                tool: call.name.clone(),
                input: call.arguments.clone(),
                changes: shown.changes,
                words,
            });
        }
        format!("{}\n{PROPOSED}", shown.text)
    }
}

/// The text and the changes of an answer. An answer leaves the use cases only as its masked JSON,
/// so that is what is read back.
fn split(answer: &Authorized<ToolAnswer>) -> Option<ToolAnswer> {
    serde_json::from_value(answer.masked_value().ok()?).ok()
}

fn read_text(answer: &Authorized<ToolAnswer>) -> String {
    split(answer).map_or_else(|| COULD_NOT_RUN.to_owned(), |answer| answer.text)
}

/// A sentence for the model about a call that did not work. It names a class of problem and never
/// what the store said.
fn sentence_of(error: &AppError) -> String {
    match error {
        AppError::Refused(code) => code.text().render(),
        AppError::Mistakes(_) => Message::from(message::AI_SEARCH_MISTAKE).render(),
        AppError::RefusedAtField { .. } => NOT_VALID.to_owned(),
        AppError::Stale(_) | AppError::Conflict(_) => CHANGED.to_owned(),
        _ => COULD_NOT_RUN.to_owned(),
    }
}
