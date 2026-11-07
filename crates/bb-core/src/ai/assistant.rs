//! The limits of a question to the assistant and of the proposals it makes (AI-10, AI-05, M7-Q6).
//! Written once: the use case, the route and the screen all read these.

use jiff::Timestamp;

use crate::personal::Entity;

/// The longest question a person types, in characters.
pub const QUESTION_CHARS_MAX: usize = 4_000;

/// The most turns of the conversation so far that the browser sends back. The server stores none of
/// them (M7-Q6).
pub const TURNS_MAX: usize = 20;

/// The longest turn of the conversation so far, in characters.
pub const TURN_CHARS_MAX: usize = 8_000;

/// The most things the panel looks at that one question names (an issue, a search, a project).
pub const LOOKING_AT_MAX: usize = 5;

/// The longest name of what the panel looks at (an issue ID, a short name, a search), in characters.
pub const LOOKING_AT_CHARS_MAX: usize = 500;

/// The most tools one question may run. A tool that asks for the same again counts, a call over
/// the limit gets a sentence for an answer and runs nothing.
pub const TOOL_CALLS_MAX: usize = 8;

/// The most times one question calls the provider: every round but the last runs at least one tool.
pub const ROUNDS_MAX: usize = TOOL_CALLS_MAX + 1;

/// The most proposals one answer carries.
pub const PROPOSALS_MAX: usize = 10;

/// How long a proposal can be applied, in seconds. A person who is away longer asks again.
pub const PROPOSAL_LIFETIME_SECONDS: i64 = 10 * 60;

/// The most bytes of the input of a proposal that *Apply* takes back.
pub const PROPOSAL_INPUT_BYTES_MAX: usize = 64 * 1024;

words! {
    /// Who said a turn of the conversation the browser keeps.
    pub enum ChatFrom {
        /// The person who types into the panel.
        Person => "person",
        /// The assistant.
        Assistant => "assistant",
    }
}

words_in_api!(
    ChatFrom,
    "Who said a turn of the conversation: the person, or the assistant."
);

words! {
    /// What the panel looks at when a question is asked.
    pub enum LookingAtKind {
        /// An issue, by its readable ID.
        Issue => "issue",
        /// The search of a list, in the search language.
        Search => "search",
        /// A project, by its short name.
        Project => "project",
    }
}

words_in_api!(
    LookingAtKind,
    "What the panel looks at: an issue, the search of a list, or a project."
);

/// A reference of a proposal that was applied. It works once, so its random word is kept until the
/// reference would have run out anyway.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct UsedReference {
    /// The random word of the reference; it tells nothing about the change or the person.
    #[not_personal]
    pub nonce: ReferenceWord,
    /// When the reference runs out and the word can go.
    #[not_personal]
    pub expires_at: Timestamp,
}

/// The random word of a reference of the assistant. It tells nothing about the change or the person.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferenceWord(pub String);

impl crate::personal::NotPersonal for ReferenceWord {}
