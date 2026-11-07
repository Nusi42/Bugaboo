//! The AI provider (AI-01, AI-02, PRIV-08, COMP-08, ADR-0013): the port every way in to AI calls
//! (the assistant, MCP, agents), and the setup of the provider.
//!
//! The port takes a request and gives back the text and the tool calls the provider asks for. It
//! knows nothing of issues or people: the context builder makes the request. The adapters
//! (`bb-ai`) speak the Messages API of Anthropic and the chat shape of `OpenAI`; the tests use the
//! fake of `bb-ai`. A failure names a class of problem and never what the provider sent back, which
//! can repeat the text of the request (DATA-LOGS).

mod call;
pub mod context;
#[cfg(any(test, feature = "testing"))]
mod fake;
mod never_sent;
mod setup;
mod setup_view;
mod store;
pub mod tools;

use std::fmt;

use bb_core::ai::ModelEndpoint;
use bb_core::ai::ProviderKind;
use serde_json::Value;
use zeroize::Zeroizing;

use crate::ports::BoxFuture;

pub use never_sent::{
    NEVER_SENT_PROJECTS_MAX, NeverSentField, NeverSentList, NeverSentProject, NeverSentQuery,
    NeverSentStore, NeverSentTarget, SetNeverSent, Withheld, never_sent_list, set_never_sent,
};
pub use setup::{
    ChangeAiSetup, ChangeProjectAi, ConfirmAiAgreement, SetAiKey, TryAiProvider, change_ai_setup,
    change_project_ai, confirm_ai_agreement, set_ai_key, try_ai_provider,
};
pub use setup_view::{
    AgreementView, AiSetup, AiSetupQuery, ProjectAi, ProjectAiQuery, WhatGoesWhere,
    WhatGoesWhereQuery, ai_setup, project_ai, what_goes_where,
};
pub use store::AiStore;

#[cfg(any(test, feature = "testing"))]
pub use fake::{FakeAiProvider, RecordedCall};

/// Who speaks in a turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiRole {
    /// The person (or the program on their behalf).
    User,
    /// The model.
    Assistant,
}

/// One piece of a turn.
#[derive(Clone, Debug, PartialEq)]
pub enum AiPart {
    /// Text.
    Text(String),
    /// A tool the model asked for, in the turn of the model.
    ToolCall(AiToolCall),
    /// What a tool answered, in a turn of the user, for the call with this ID.
    ToolResult {
        /// The ID of the call this answers.
        call: String,
        /// The text of the answer.
        text: String,
    },
}

/// One turn of the conversation.
#[derive(Clone, Debug, PartialEq)]
pub struct AiTurn {
    /// Who speaks.
    pub role: AiRole,
    /// What is said, in order.
    pub parts: Vec<AiPart>,
}

/// A tool the model may ask for: its name, what it does and the shape of its arguments.
#[derive(Clone, Debug, PartialEq)]
pub struct AiToolSpec {
    /// The name the model uses.
    pub name: String,
    /// What the tool does, for the model.
    pub description: String,
    /// The JSON schema of the arguments.
    pub schema: Value,
}

/// A tool the model asked for.
#[derive(Clone, Debug, PartialEq)]
pub struct AiToolCall {
    /// The ID the provider gave the call.
    pub id: String,
    /// The name of the tool.
    pub name: String,
    /// The arguments.
    pub arguments: Value,
}

/// One call to the provider.
#[derive(Clone, Debug, PartialEq)]
pub struct AiRequest {
    /// The system text.
    pub system: String,
    /// The conversation so far.
    pub turns: Vec<AiTurn>,
    /// The tools the model may ask for.
    pub tools: Vec<AiToolSpec>,
    /// The most tokens of output (never above `bb_core::ai::ANSWER_TOKENS_MAX`).
    pub max_tokens: u32,
}

/// What the provider answered.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AiAnswer {
    /// The text of the answer; may be empty when the model only asks for tools.
    pub text: String,
    /// The tools it asks for, in order.
    pub tool_calls: Vec<AiToolCall>,
    /// Tokens the provider counted for the request.
    pub tokens_in: u32,
    /// Tokens the provider counted for the answer.
    pub tokens_out: u32,
}

/// Where and how to call: the settings and the key as they are for one call. The server reads them
/// again for every call, so a change counts from the next one.
#[derive(Clone)]
pub struct AiTarget {
    /// The kind of provider.
    pub kind: ProviderKind,
    /// The address of an OpenAI-compatible model; none for Anthropic, which has a fixed address.
    pub endpoint: Option<ModelEndpoint>,
    /// The model.
    pub model: String,
    /// The key; empty for a model that asks for none. Wiped from memory when dropped.
    pub key: Zeroizing<String>,
}

impl fmt::Debug for AiTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AiTarget")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

/// Why a call did not give an answer. One class: nothing of what the provider said is passed on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiFailure {
    /// The provider could not be reached, took too long, refused, or answered with something that
    /// is not an answer.
    Unavailable,
}

/// Calls an AI provider.
pub trait AiProvider: Send + Sync {
    /// Sends `request` to the provider `target` names and waits for the answer, once. The adapter
    /// keeps to the time limit and the size limit of `bb_core::ai`.
    ///
    /// # Errors
    /// [`AiFailure::Unavailable`] when there is no usable answer.
    fn complete<'a>(
        &'a self,
        target: &'a AiTarget,
        request: &'a AiRequest,
    ) -> BoxFuture<'a, Result<AiAnswer, AiFailure>>;
}

/// The provider of a server that has none wired in: every call is turned away.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoAiProvider;

impl AiProvider for NoAiProvider {
    fn complete<'a>(
        &'a self,
        _target: &'a AiTarget,
        _request: &'a AiRequest,
    ) -> BoxFuture<'a, Result<AiAnswer, AiFailure>> {
        Box::pin(async { Err(AiFailure::Unavailable) })
    }
}
