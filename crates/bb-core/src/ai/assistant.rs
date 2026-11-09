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

/// The most tools one question may run until an administrator sets another most (the setting
/// `assistant.look-ups-most`). A tool that asks for the same again counts, a call over the limit
/// gets a sentence for an answer and runs nothing.
pub const TOOL_CALLS_AT_START: usize = 8;

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

/// How many tools the assistant may run for one question: a number, or no limit the administrator
/// set. The organization sets the most; a person may choose a lower one for themselves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LookUps {
    /// At most this many tools.
    Most(usize),
    /// No limit: nothing stops the look-ups of a question.
    NoLimit,
}

/// The stored word of *No limit*.
pub const LOOK_UPS_NO_LIMIT: &str = "no-limit";

impl Default for LookUps {
    fn default() -> Self {
        Self::Most(TOOL_CALLS_AT_START)
    }
}

impl LookUps {
    /// The look-ups a stored word means: a whole number or `no-limit`; `None` for anything else
    /// (the word of a person who follows the organization is no look-up).
    #[must_use]
    pub fn of_code(code: &str) -> Option<Self> {
        if code == LOOK_UPS_NO_LIMIT {
            return Some(Self::NoLimit);
        }
        code.parse().ok().filter(|most| *most > 0).map(Self::Most)
    }

    /// The tools one question may run, or `None` for no limit.
    #[must_use]
    pub const fn calls(self) -> Option<usize> {
        match self {
            Self::Most(most) => Some(most),
            Self::NoLimit => None,
        }
    }

    /// The most calls to the provider for one question, or `None` for no limit: every round but the
    /// last runs at least one tool, and the last is given none.
    #[must_use]
    pub const fn rounds(self) -> Option<usize> {
        match self {
            Self::Most(most) => Some(most + 1),
            Self::NoLimit => None,
        }
    }

    /// What counts for a person: the lower of the organization's and their own choice, so nobody
    /// goes above the most of the organization. A person who chose nothing gets the organization's.
    #[must_use]
    pub const fn lowered_by(self, person: Option<Self>) -> Self {
        match (self, person) {
            (organization, None) | (organization @ Self::Most(_), Some(Self::NoLimit)) => {
                organization
            }
            (Self::NoLimit, Some(own)) => own,
            (Self::Most(most), Some(Self::Most(own))) => {
                Self::Most(if own < most { own } else { most })
            }
        }
    }
}

#[cfg(test)]
mod look_up_tests {
    use super::*;

    #[test]
    fn m7_wp15_look_ups_are_read_from_their_words() {
        assert_eq!(LookUps::of_code("8"), Some(LookUps::Most(8)));
        assert_eq!(LookUps::of_code("no-limit"), Some(LookUps::NoLimit));
        for bad in ["", "0", "-1", "many", "as-organization", "8 "] {
            assert_eq!(LookUps::of_code(bad), None, "{bad:?}");
        }
        assert_eq!(LookUps::default(), LookUps::Most(TOOL_CALLS_AT_START));
    }

    #[test]
    fn m7_wp15_no_limit_has_no_number_at_all() {
        assert_eq!(LookUps::NoLimit.calls(), None);
        assert_eq!(LookUps::NoLimit.rounds(), None);
        assert_eq!(LookUps::Most(1_000).calls(), Some(1_000));
        assert_eq!(LookUps::Most(3).rounds(), Some(4));
    }

    #[test]
    fn m7_wp15_a_person_never_goes_above_the_organization() {
        let most = |n| LookUps::Most(n);
        assert_eq!(most(8).lowered_by(None), most(8));
        assert_eq!(most(8).lowered_by(Some(most(3))), most(3));
        assert_eq!(most(4).lowered_by(Some(most(20))), most(4));
        assert_eq!(most(4).lowered_by(Some(LookUps::NoLimit)), most(4));
        assert_eq!(LookUps::NoLimit.lowered_by(Some(most(5))), most(5));
        assert_eq!(LookUps::NoLimit.lowered_by(None), LookUps::NoLimit);
    }
}
