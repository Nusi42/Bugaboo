//! The settings of AI (AI-01, AI-02, PRIV-08, ADR-0013): the switch, the provider, the model, the
//! address of an OpenAI-compatible model and whether names of people go to an outside provider.
//! They are part of the registry like every other setting ([`super::registry`] lists them), but
//! they are changed only through their own routes (`changed_only_on_its_own_screen`), because
//! switching AI on needs the agreement, and changing the provider or the address needs the
//! password again. The key of the provider is a stored secret and the agreement a stored record;
//! neither is a setting.

use super::{Bool, ChangedBy, Choice, Format, Level, Setting, SettingDefinition, Text, settings};
use crate::ai::ProviderKind;
use crate::message;

/// The choices of *Provider*. Written out like the modes of the issue IDs (the registry is read as
/// text by the tools that make the constants of the browser) and held to the words of
/// `ProviderKind` by the assertions below.
const PROVIDER_CHOICES: &[Choice] = &[
    Choice::new("anthropic", message::AI_PROVIDER_ANTHROPIC),
    Choice::new("openai-compatible", message::AI_PROVIDER_OPENAI_COMPATIBLE),
];

const _: () = {
    assert!(crate::words::same_text(
        ProviderKind::Anthropic.code(),
        "anthropic"
    ));
    assert!(crate::words::same_text(
        ProviderKind::OpenAiCompatible.code(),
        "openai-compatible"
    ));
};

settings! {
    /// Whether AI is on: for the organization (off at the start), and for a project that does not
    /// follow the organization. AI counts at a place only when both have it on.
    AI_ENABLED: Bool = {
        key: "ai.enabled",
        name: message::SETTING_AI_ENABLED_NAME,
        default: false,
        levels: [Organization, Project],
        changed_by: ProjectLeads,
        help: message::SETTING_AI_ENABLED_HELP,
        lockable: false,
    }

    /// Which kind of provider answers.
    AI_PROVIDER: Text = {
        key: "ai.provider",
        name: message::SETTING_AI_PROVIDER_NAME,
        default: "anthropic",
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_AI_PROVIDER_HELP,
        lockable: false,
        format: Format::Choices(PROVIDER_CHOICES),
    }

    /// The model the provider is asked to use, as the provider names it.
    AI_MODEL: Text = {
        key: "ai.model",
        name: message::SETTING_AI_MODEL_NAME,
        default: "",
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_AI_MODEL_HELP,
        lockable: false,
    }

    /// The address of an OpenAI-compatible model. Anthropic has a fixed address.
    AI_ADDRESS: Text = {
        key: "ai.address",
        name: message::SETTING_AI_ADDRESS_NAME,
        default: "",
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_AI_ADDRESS_HELP,
        lockable: false,
    }

    /// Whether names of people go to an outside provider (M7-Q5). Off: people go as their role
    /// label. A model on the local network always gets the names a person sees.
    AI_SEND_NAMES: Bool = {
        key: "ai.send-names",
        name: message::SETTING_AI_SEND_NAMES_NAME,
        default: false,
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_AI_SEND_NAMES_HELP,
        lockable: false,
    }
}

/// Whether the key is one of the settings of AI.
#[must_use]
pub fn is_ai_setting(key: &str) -> bool {
    ALL.iter().any(|definition| definition.key() == key)
}
