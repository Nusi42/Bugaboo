//! The scenarios of the AI use cases (ADR-0007, M1-F18): each runs the call under test once and says
//! how many events it appended.

use bb_core::ai::{Destination, ProviderKind};

use super::scenarios::{Added, a_project, measured, text, token};
use super::world::{PASSWORD, World};
use crate::ai::{
    AiPart, AiRequest, AiRole, AiTurn, ChangeAiSetup, ChangeProjectAi, ConfirmAiAgreement,
    SetAiKey, TryAiProvider, change_ai_setup, change_project_ai, confirm_ai_agreement, set_ai_key,
    try_ai_provider,
};

/// The address of a model on the local network, as the tests enter it.
pub(crate) const LOCAL_MODEL: &str = "http://models.lan/v1";

/// The address of a provider on the internet, as the tests enter it.
pub(crate) const OUTSIDE_MODEL: &str = "https://models.example.test/v1";

/// A change of the settings that leaves everything as it is, to be filled in by the test.
pub(crate) fn setup(world: &World) -> ChangeAiSetup {
    ChangeAiSetup {
        session_token: token(world),
        enabled: None,
        provider: None,
        model: None,
        address: None,
        send_names: None,
        password: None,
    }
}

/// The model of the organization is a local one, with a name.
pub(crate) async fn a_local_model(world: &World) -> Result<(), String> {
    change_ai_setup(
        &world.ctx(),
        ChangeAiSetup {
            provider: Some(ProviderKind::OpenAiCompatible),
            model: Some("local-test".to_owned()),
            address: Some(LOCAL_MODEL.to_owned()),
            password: Some(PASSWORD.to_owned()),
            ..setup(world)
        },
    )
    .await
    .map(|_| ())
    .map_err(text)
}

/// A short question for the provider.
pub(crate) fn a_question() -> AiRequest {
    AiRequest {
        system: String::new(),
        turns: vec![AiTurn {
            role: AiRole::User,
            parts: vec![AiPart::Text("Hello".to_owned())],
        }],
        tools: Vec::new(),
        max_tokens: 16,
    }
}

pub(crate) async fn change_ai_setup_scenario(world: &World) -> Added {
    measured(
        &world.store,
        change_ai_setup(
            &world.ctx(),
            ChangeAiSetup {
                provider: Some(ProviderKind::OpenAiCompatible),
                model: Some("local-test".to_owned()),
                address: Some(LOCAL_MODEL.to_owned()),
                password: Some(PASSWORD.to_owned()),
                ..setup(world)
            },
        ),
    )
    .await
}

pub(crate) async fn set_ai_key_scenario(world: &World) -> Added {
    measured(
        &world.store,
        set_ai_key(
            &world.ctx(),
            SetAiKey {
                session_token: token(world),
                key: Some("a made-up key".to_owned()),
                password: PASSWORD.to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn confirm_ai_agreement_scenario(world: &World) -> Added {
    measured(
        &world.store,
        confirm_ai_agreement(
            &world.ctx(),
            ConfirmAiAgreement {
                session_token: token(world),
                destination: Destination::Anthropic,
            },
        ),
    )
    .await
}

pub(crate) async fn change_project_ai_scenario(world: &World) -> Added {
    a_project(world, "ACME").await?;
    measured(
        &world.store,
        change_project_ai(
            &world.ctx(),
            ChangeProjectAi {
                session_token: token(world),
                project: "ACME".to_owned(),
                enabled: Some(false),
            },
        ),
    )
    .await
}

pub(crate) async fn try_ai_provider_scenario(world: &World) -> Added {
    a_local_model(world).await?;
    measured(
        &world.store,
        try_ai_provider(
            &world.ctx(),
            TryAiProvider {
                session_token: token(world),
            },
        ),
    )
    .await
}
