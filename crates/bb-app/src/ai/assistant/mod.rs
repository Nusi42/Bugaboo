//! The assistant on the server (AI-10, AI-05, COMP-08, COMP-10, ADR-0013, M7-Q2, Q5, Q6).
//!
//! A person asks a question about what the panel looks at. The use case [`ask_assistant`] builds the
//! request for the provider (`system.rs`), lets the model read through the tools of the one
//! registry as the person, at most a named number of times, and turns every tool that changes
//! something into a **proposed change** with the value now and after. Nothing is saved: a proposal
//! carries a signed reference (`reference.rs`), and only [`apply_proposal`] runs the tool, through
//! its own use case as the person and through the channel `assistant`. The conversation is the
//! browser's: the server keeps none of it (M7-Q6), only the log of tool calls, which never holds
//! content.
//!
//! This module calls tools only through `ai::tools` (ARCH-AI-TOOLS) and the provider only through
//! [`Ctx::call_provider`](crate::auth::Ctx), so the rules of *AI is off here*, of the agreement and
//! of what is kept from AI are the ones every way in shares.

mod apply;
mod ask;
mod people;
mod reference;
mod status;
mod step;
mod system;
mod words;

use bb_auth::looks_like_access_key;
use bb_core::ErrorCode;
use bb_core::ai::{ChatFrom, LookingAtKind};
use bb_policy::Masked;
use serde::Serialize;
use serde_json::Value;

pub use apply::{Applied, ApplyProposal, apply_proposal};
pub use ask::{AskAssistantQuery, ask_assistant};
pub use status::{AssistantStatus, AssistantStatusQuery, assistant_status};

use super::tools::ProposedChange;
use crate::auth::AppError;
use crate::history::FieldChangeView;

/// The assistant is the panel of a person who is signed in: an access key, also the key of an AI
/// app, is turned away. AI apps work through MCP, with the scopes they were given (ADR-0042).
fn session_only(token: &str) -> Result<(), AppError> {
    if looks_like_access_key(token) {
        return Err(AppError::Refused(ErrorCode::AccessKeyNotAllowed));
    }
    Ok(())
}

/// One thing the panel looks at when the question is asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LookingAt {
    /// What kind of thing it is.
    pub kind: LookingAtKind,
    /// An issue ID, the search of the list, or a short name.
    pub value: String,
}

/// One turn of the conversation so far, which the browser keeps and sends back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatTurn {
    /// Who said it.
    pub from: ChatFrom,
    /// What was said, in Markdown for the assistant.
    pub text: String,
}

/// A change the assistant proposes: the tool and its input with a reference that only this server
/// made for this person, and what the change would be.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Proposal {
    /// The signed reference *Apply* takes back with the tool and the input.
    pub reference: String,
    /// The name of the tool.
    pub tool: String,
    /// Its input, exactly as it was previewed.
    pub input: Value,
    /// The fields it would change, with the value now and the value after.
    pub changes: Vec<ProposedChange>,
    /// The same fields in the words of the history of the issue (status names, people as the
    /// reader may see them), for the fields the reader may see; empty when the change is on no
    /// issue yet.
    pub words: Vec<FieldChangeView>,
}

/// What the assistant answers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AssistantAnswer {
    /// The answer in Markdown, as the model wrote it.
    pub text: String,
    /// The same, written by the one Markdown writer with the option for AI answers: no picture,
    /// every link with its full address.
    pub html: String,
    /// The changes it proposes; nothing is saved until the person applies one.
    pub proposals: Vec<Proposal>,
}

impl Masked for AssistantAnswer {}
