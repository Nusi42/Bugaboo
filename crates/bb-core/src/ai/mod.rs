//! The AI provider an administrator chose (AI-01, AI-02, PRIV-08, COMP-08, ADR-0013): the table of
//! providers, the rules for the address of a model, and the limits of one call.
//!
//! The settings themselves (`ai.*`) are in the registry; the key of the provider is a stored secret
//! (SEC-16) and the agreement is a stored record ([`Agreement`]). Nothing here does input or output.

mod address;
mod assistant;
mod tools;

pub use address::{ADDRESS_MAX, AddressProblem, ModelAddress, ModelEndpoint};
pub use assistant::{
    LOOKING_AT_CHARS_MAX, LOOKING_AT_MAX, PROPOSAL_INPUT_BYTES_MAX, PROPOSAL_LIFETIME_SECONDS,
    PROPOSALS_MAX, QUESTION_CHARS_MAX, ROUNDS_MAX, TOOL_CALLS_MAX, TURN_CHARS_MAX, TURNS_MAX,
};
pub use tools::{
    AuthorKind, NeverSentMark, NeverSentSubject, TOOL_CALL_KEEP_DAYS, Tool, ToolCall, ToolCaller,
    ToolEffect, ToolOutcome, ToolScope,
};

use jiff::Timestamp;

use crate::id::UserId;
use crate::message::{self, MessageKey};

words! {
    /// The kind of provider an administrator chooses in *Set up the assistant*.
    pub enum ProviderKind {
        /// Anthropic, through its Messages API at a fixed address.
        Anthropic => "anthropic",
        /// Any service that speaks the chat shape of `OpenAI`, at an address the administrator
        /// enters: an outside provider or a model on the organization's own network.
        OpenAiCompatible => "openai-compatible",
    }
}

words_in_api!(
    ProviderKind,
    "The kind of AI provider: anthropic, or openai-compatible (any service with the chat shape of OpenAI, also a model on the organization's own network)."
);

words! {
    /// Where the data of a call goes. Only this decides whether an agreement is needed.
    pub enum Destination {
        /// To Anthropic.
        Anthropic => "anthropic",
        /// To a model on this computer or the organization's own network: nothing leaves it.
        LocalModel => "local-model",
        /// To another provider on the internet that speaks the chat shape of `OpenAI`.
        OtherProvider => "other-provider",
    }
}

words_in_api!(
    Destination,
    "Where the data of a call goes: to anthropic, to a local model on the organization's own network, or to another provider on the internet."
);

words! {
    /// Where the agreement for the provider stands.
    pub enum AgreementStatus {
        /// The provider needs none: a model on the organization's own network.
        NotNeeded => "not-needed",
        /// The provider needs one, and none is confirmed for it.
        Needed => "needed",
        /// An administrator confirmed it for this provider.
        Confirmed => "confirmed",
    }
}

words_in_api!(
    AgreementStatus,
    "Where the agreement for the provider stands: not needed (a model on the organization's own network), needed (and not confirmed for this provider), or confirmed."
);

impl AgreementStatus {
    /// Where the agreement stands for a destination, given the one that was confirmed.
    #[must_use]
    pub fn of(destination: Destination, agreement: Option<&Agreement>) -> Self {
        if !destination.row().needs_agreement {
            Self::NotNeeded
        } else if agreement.is_some_and(|agreement| agreement.destination == destination) {
            Self::Confirmed
        } else {
            Self::Needed
        }
    }

    /// Whether the provider may be called: it needs no agreement or has one.
    #[must_use]
    pub const fn holds(self) -> bool {
        !matches!(self, Self::Needed)
    }
}

/// What the table knows about one destination.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProviderRow {
    /// The destination the row is about.
    pub destination: Destination,
    /// The name people read.
    pub name: MessageKey,
    /// The country the data goes to, in words.
    pub country: MessageKey,
    /// Where the provider says what it does with the data (a web address; none for a local model).
    pub information: Option<&'static str>,
    /// Whether an administrator must confirm a data processing agreement before it is switched on.
    pub needs_agreement: bool,
}

/// The providers: written once, read by the settings screen, *What goes where* and the agreement
/// step (PRIV-08).
pub const PROVIDERS: [ProviderRow; 3] = [
    ProviderRow {
        destination: Destination::Anthropic,
        name: message::AI_PROVIDER_ANTHROPIC,
        country: message::AI_COUNTRY_ANTHROPIC,
        information: Some("https://www.anthropic.com/legal/privacy"),
        needs_agreement: true,
    },
    ProviderRow {
        destination: Destination::LocalModel,
        name: message::AI_PROVIDER_LOCAL_MODEL,
        country: message::AI_COUNTRY_LOCAL_MODEL,
        information: None,
        needs_agreement: false,
    },
    ProviderRow {
        destination: Destination::OtherProvider,
        name: message::AI_PROVIDER_OTHER,
        country: message::AI_COUNTRY_OTHER,
        information: None,
        needs_agreement: true,
    },
];

impl Destination {
    /// The row of the table of providers.
    #[must_use]
    pub fn row(self) -> &'static ProviderRow {
        PROVIDERS
            .iter()
            .find(|row| row.destination == self)
            .unwrap_or(&PROVIDERS[0])
    }

    /// Whether the data leaves the organization's own network.
    #[must_use]
    pub fn is_outside(self) -> bool {
        self != Self::LocalModel
    }
}

/// The destination the settings lead to: Anthropic, or the address of an OpenAI-compatible model
/// (a model on the local network is private, any other address is an outside provider).
#[must_use]
pub fn destination_of(kind: ProviderKind, address: Option<&ModelAddress>) -> Destination {
    match (kind, address) {
        (ProviderKind::Anthropic, _) => Destination::Anthropic,
        (ProviderKind::OpenAiCompatible, Some(address)) if address.is_local() => {
            Destination::LocalModel
        }
        (ProviderKind::OpenAiCompatible, _) => Destination::OtherProvider,
    }
}

/// The confirmation that a data processing agreement with the provider is in place: who confirmed
/// it, when, and for which destination. One record; a change of the provider deletes it.
#[derive(Clone, Debug, PartialEq, Eq, crate::personal::Entity)]
pub struct Agreement {
    /// The destination the administrator confirmed it for.
    #[not_personal]
    pub destination: Destination,
    /// The administrator who confirmed it.
    #[personal(category = "activity", basis = "contract")]
    pub confirmed_by: UserId,
    /// When it was confirmed.
    #[personal(category = "activity", basis = "contract")]
    pub confirmed_at: Timestamp,
}

/// The name the key of the provider is stored under (SEC-16).
pub const KEY_SECRET: &str = "ai.key";

/// The longest key of a provider, in bytes: a limit of storage, not of security.
pub const KEY_MAX: usize = 1024;

/// The longest name of a model, in characters.
pub const MODEL_MAX: usize = 100;

/// How long one call to the provider may take in all, in seconds. A provider that does not answer
/// in this time has not answered.
pub const CALL_TIMEOUT_SECONDS: u64 = 60;

/// How long the connection to the provider may take to open, in seconds.
pub const CONNECT_TIMEOUT_SECONDS: u64 = 10;

/// The most tokens of output one call asks the provider for.
pub const ANSWER_TOKENS_MAX: u32 = 4096;

/// The most bytes of an answer the server reads from the provider; more is cut off and counts as
/// no answer.
pub const ANSWER_BYTES_MAX: usize = 1024 * 1024;

/// How many seconds a person waits between two tries of the provider, so the button cannot be used
/// to find open ports on the network.
pub const TRY_EVERY_SECONDS: i64 = 10;

/// The most projects *What goes where* names as having switched AI off; it says when it cut the
/// list off.
pub const SWITCHED_OFF_MAX: usize = 100;

/// The text the try of the provider sends: it holds no data of the organization.
pub const TRY_TEXT: &str = "Reply with the word OK.";

/// Whether AI counts at a place: the organization has it on and the place has it on (AI-02). The
/// one answer every way in asks (the assistant, MCP, agents); `place_on` is the value that counts
/// at the place (the nearest project that sets it, else the organization's).
#[must_use]
pub const fn counts_here(organization_on: bool, place_on: bool) -> bool {
    organization_on && place_on
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_counts_only_where_the_organization_and_the_place_have_it_on() {
        assert!(counts_here(true, true));
        assert!(!counts_here(true, false));
        assert!(!counts_here(false, true));
        assert!(!counts_here(false, false));
    }

    #[test]
    fn a_model_on_the_local_network_is_the_private_choice() {
        let local = ModelAddress::parse("http://192.168.1.20:11434/v1").unwrap();
        let outside = ModelAddress::parse("https://models.example.test/v1").unwrap();
        assert_eq!(
            destination_of(ProviderKind::OpenAiCompatible, Some(&local)),
            Destination::LocalModel
        );
        assert_eq!(
            destination_of(ProviderKind::OpenAiCompatible, Some(&outside)),
            Destination::OtherProvider
        );
        assert_eq!(
            destination_of(ProviderKind::Anthropic, None),
            Destination::Anthropic
        );
        assert!(!Destination::LocalModel.row().needs_agreement);
        assert!(Destination::Anthropic.row().needs_agreement);
        assert!(Destination::OtherProvider.row().needs_agreement);
    }
}
