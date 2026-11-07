//! The AI provider and its setup (AI-01, AI-02, PRIV-08, COMP-08), run against the in-memory store
//! and the fake provider: off at the start, a project that follows the organization, the agreement
//! step, what goes where, the password asked again, and a failure that tells nothing.

use bb_core::ErrorCode;
use bb_core::ai::{Destination, ProviderKind};

use super::scenarios::{a_project, text, token};
use super::scenarios_ai::{LOCAL_MODEL, OUTSIDE_MODEL, a_local_model, a_question, setup};
use super::world::{OTHER_PASSWORD, Outcome, PASSWORD, World};
use crate::ai::{
    AiAnswer, ChangeAiSetup, ChangeProjectAi, ConfirmAiAgreement, ProjectAiQuery, SetAiKey,
    TryAiProvider, WhatGoesWhereQuery, ai_setup, change_ai_setup, change_project_ai,
    confirm_ai_agreement, project_ai, set_ai_key, try_ai_provider, what_goes_where,
};
use crate::auth::AppError;

fn refused(error: &AppError, code: ErrorCode) -> bool {
    *error == AppError::Refused(code)
}

/// The organization uses Anthropic with a model and a key, and AI is still off.
async fn anthropic_without_agreement(world: &World) -> Outcome {
    change_ai_setup(
        &world.ctx(),
        ChangeAiSetup {
            model: Some("claude-test".to_owned()),
            ..setup(world)
        },
    )
    .await?;
    set_ai_key(
        &world.ctx(),
        SetAiKey {
            session_token: token(world),
            key: Some("a made-up key".to_owned()),
            password: PASSWORD.to_owned(),
        },
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn ai_02_ai_is_off_at_the_start_and_no_call_leaves_the_server() -> Outcome {
    let world = World::new().await?;
    let start = ai_setup(
        &world.ctx(),
        crate::ai::AiSetupQuery {
            session_token: token(&world),
        },
    )
    .await?
    .into_value();
    assert!(!start.enabled);
    assert!(!start.key_set);
    assert_eq!(start.provider, ProviderKind::Anthropic);

    // Every way in answers that AI is off, also with a provider and a key entered.
    let error = world.ctx().ask_ai(None, &a_question()).await.unwrap_err();
    assert!(refused(&error, ErrorCode::AiOff), "{error:?}");
    anthropic_without_agreement(&world).await?;
    let error = world.ctx().ask_ai(None, &a_question()).await.unwrap_err();
    assert!(refused(&error, ErrorCode::AiOff), "{error:?}");
    // Trying the provider needs the agreement of an outside provider first.
    let error = try_ai_provider(
        &world.ctx(),
        TryAiProvider {
            session_token: token(&world),
        },
    )
    .await
    .unwrap_err();
    assert!(refused(&error, ErrorCode::AiAgreementNeeded), "{error:?}");
    assert!(world.ai.calls().is_empty(), "nothing left the server");
    Ok(())
}

#[tokio::test]
async fn ai_02_a_project_follows_the_organization_until_it_has_its_own_switch() -> Outcome {
    let world = World::new().await?;
    a_local_model(&world).await?;
    a_project(&world, "ACME").await?;
    let project = a_project(&world, "BETA").await?;
    let acme = world
        .ctx()
        .ai_state_at(Some(a_project_id(&world, "ACME").await?))
        .await?;
    assert!(!acme.counts(), "the organization has AI off");

    change_ai_setup(
        &world.ctx(),
        ChangeAiSetup {
            enabled: Some(true),
            ..setup(&world)
        },
    )
    .await?;
    let acme_id = a_project_id(&world, "ACME").await?;
    world.ctx().ask_ai(Some(acme_id), &a_question()).await?;
    assert_eq!(
        world.ai.calls().len(),
        1,
        "the project follows the organization"
    );

    let off = change_project_ai(
        &world.ctx(),
        ChangeProjectAi {
            session_token: token(&world),
            project: "ACME".to_owned(),
            enabled: Some(false),
        },
    )
    .await?
    .into_value();
    assert_eq!(off.own, Some(false));
    assert!(!off.works_here);
    let error = world
        .ctx()
        .ask_ai(Some(acme_id), &a_question())
        .await
        .unwrap_err();
    assert!(refused(&error, ErrorCode::AiOff), "{error:?}");
    // The other project keeps following.
    world.ctx().ask_ai(Some(project), &a_question()).await?;

    // Going back to the organization's switch.
    let back = change_project_ai(
        &world.ctx(),
        ChangeProjectAi {
            session_token: token(&world),
            project: "ACME".to_owned(),
            enabled: None,
        },
    )
    .await?
    .into_value();
    assert_eq!(back.own, None);
    assert!(back.works_here);

    // A project cannot switch on what the organization has off.
    change_ai_setup(
        &world.ctx(),
        ChangeAiSetup {
            enabled: Some(false),
            ..setup(&world)
        },
    )
    .await?;
    change_project_ai(
        &world.ctx(),
        ChangeProjectAi {
            session_token: token(&world),
            project: "ACME".to_owned(),
            enabled: Some(true),
        },
    )
    .await?;
    let view = project_ai(
        &world.ctx(),
        ProjectAiQuery {
            session_token: token(&world),
            project: "ACME".to_owned(),
        },
    )
    .await?
    .into_value();
    assert_eq!(view.own, Some(true));
    assert!(!view.works_here);
    Ok(())
}

async fn a_project_id(world: &World, short_name: &str) -> Result<bb_core::id::ProjectId, String> {
    Ok(world
        .ctx()
        .find_project(short_name)
        .await
        .map_err(text)?
        .ok_or("no such project")?
        .project
        .id)
}

#[tokio::test]
async fn priv_08_an_outside_provider_needs_the_agreement_before_it_is_switched_on() -> Outcome {
    let world = World::new().await?;
    anthropic_without_agreement(&world).await?;
    let switch_on = || ChangeAiSetup {
        enabled: Some(true),
        ..setup(&world)
    };
    let error = change_ai_setup(&world.ctx(), switch_on())
        .await
        .unwrap_err();
    assert!(refused(&error, ErrorCode::AiAgreementNeeded), "{error:?}");
    assert!(!ai_enabled(&world).await?);

    // The page that is out of date confirms nothing.
    let error = confirm_ai_agreement(
        &world.ctx(),
        ConfirmAiAgreement {
            session_token: token(&world),
            destination: Destination::OtherProvider,
        },
    )
    .await
    .unwrap_err();
    assert!(refused(&error, ErrorCode::InvalidRequest), "{error:?}");

    let confirmed = confirm_ai_agreement(
        &world.ctx(),
        ConfirmAiAgreement {
            session_token: token(&world),
            destination: Destination::Anthropic,
        },
    )
    .await?
    .into_value();
    let agreement = confirmed.agreement.ok_or("no agreement kept")?;
    assert_eq!(agreement.destination, Destination::Anthropic);
    assert!(!confirmed.agreement_needed);
    change_ai_setup(&world.ctx(), switch_on()).await?;
    assert!(ai_enabled(&world).await?);
    world.ctx().ask_ai(None, &a_question()).await?;
    let calls = world.ai.calls();
    assert_eq!(calls.len(), 1);
    assert!(calls[0].had_key);
    assert_eq!(calls[0].kind, ProviderKind::Anthropic);

    // A new provider asks again: the agreement of the old one does not carry over.
    change_ai_setup(
        &world.ctx(),
        ChangeAiSetup {
            enabled: Some(false),
            ..setup(&world)
        },
    )
    .await?;
    change_ai_setup(
        &world.ctx(),
        ChangeAiSetup {
            provider: Some(ProviderKind::OpenAiCompatible),
            address: Some(OUTSIDE_MODEL.to_owned()),
            model: Some("other-test".to_owned()),
            password: Some(PASSWORD.to_owned()),
            ..setup(&world)
        },
    )
    .await?;
    let error = change_ai_setup(&world.ctx(), switch_on())
        .await
        .unwrap_err();
    assert!(refused(&error, ErrorCode::AiAgreementNeeded), "{error:?}");
    Ok(())
}

async fn ai_enabled(world: &World) -> Result<bool, String> {
    Ok(ai_setup(
        &world.ctx(),
        crate::ai::AiSetupQuery {
            session_token: token(world),
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .enabled)
}

#[tokio::test]
async fn priv_08_a_local_model_needs_no_agreement() -> Outcome {
    let world = World::new().await?;
    a_local_model(&world).await?;
    let setup_now = change_ai_setup(
        &world.ctx(),
        ChangeAiSetup {
            enabled: Some(true),
            ..setup(&world)
        },
    )
    .await?
    .into_value();
    assert!(setup_now.enabled);
    assert_eq!(setup_now.destination, Some(Destination::LocalModel));
    assert!(!setup_now.agreement_needed);
    world.ctx().ask_ai(None, &a_question()).await?;
    let calls = world.ai.calls();
    assert_eq!(calls[0].address.as_deref(), Some(LOCAL_MODEL));
    assert!(!calls[0].had_key, "a local model asks for no key");
    Ok(())
}

#[tokio::test]
async fn priv_08_what_goes_where_names_the_provider_its_country_and_the_data() -> Outcome {
    let world = World::new().await?;
    a_local_model(&world).await?;
    a_project(&world, "ACME").await?;
    change_project_ai(
        &world.ctx(),
        ChangeProjectAi {
            session_token: token(&world),
            project: "ACME".to_owned(),
            enabled: Some(false),
        },
    )
    .await?;
    let asked = || WhatGoesWhereQuery {
        session_token: token(&world),
    };
    let local = what_goes_where(&world.ctx(), asked()).await?.into_value();
    assert_eq!(local.destination, Destination::LocalModel);
    assert_eq!(local.model, "local-test");
    assert!(
        local.names_sent,
        "a local model gets the names a person sees"
    );
    assert!(!local.agreement_needed);
    assert_eq!(local.switched_off.len(), 1);
    assert_eq!(
        local.switched_off[0].id,
        a_project_id(&world, "ACME").await?
    );
    assert_eq!(
        Destination::LocalModel.row().country,
        bb_core::message::AI_COUNTRY_LOCAL_MODEL
    );

    change_ai_setup(
        &world.ctx(),
        ChangeAiSetup {
            provider: Some(ProviderKind::Anthropic),
            address: Some(String::new()),
            model: Some("claude-test".to_owned()),
            password: Some(PASSWORD.to_owned()),
            ..setup(&world)
        },
    )
    .await?;
    let outside = what_goes_where(&world.ctx(), asked()).await?.into_value();
    assert_eq!(outside.destination, Destination::Anthropic);
    assert!(!outside.names_sent, "people go as their role at the start");
    assert!(outside.agreement_needed);
    assert_eq!(
        Destination::Anthropic.row().country,
        bb_core::message::AI_COUNTRY_ANTHROPIC
    );
    assert!(Destination::Anthropic.row().information.is_some());

    change_ai_setup(
        &world.ctx(),
        ChangeAiSetup {
            send_names: Some(true),
            ..setup(&world)
        },
    )
    .await?;
    let named = what_goes_where(&world.ctx(), asked()).await?.into_value();
    assert!(named.names_sent);
    Ok(())
}

#[tokio::test]
async fn ai_01_the_key_is_never_read_back_and_is_stored_as_a_secret() -> Outcome {
    let world = World::new().await?;
    let error = set_ai_key(
        &world.ctx(),
        SetAiKey {
            session_token: token(&world),
            key: Some("the-key-123".to_owned()),
            password: OTHER_PASSWORD.to_owned(),
        },
    )
    .await
    .unwrap_err();
    assert!(refused(&error, ErrorCode::WrongCredentials), "{error:?}");
    assert!(world.store.lock().state.ai_key.is_none());

    set_ai_key(
        &world.ctx(),
        SetAiKey {
            session_token: token(&world),
            key: Some("the-key-123".to_owned()),
            password: PASSWORD.to_owned(),
        },
    )
    .await?;
    let shown = ai_setup(
        &world.ctx(),
        crate::ai::AiSetupQuery {
            session_token: token(&world),
        },
    )
    .await?
    .into_value();
    assert!(shown.key_set);
    assert!(
        !format!("{shown:?}").contains("the-key-123"),
        "the setup has no place for the key"
    );
    // No event, setting or security-log line holds the key.
    let events = format!("{:?}", world.store.lock().events);
    assert!(!events.contains("the-key-123"));

    // A new provider does not get the key of the old one.
    a_local_model(&world).await?;
    assert!(world.store.lock().state.ai_key.is_none());
    Ok(())
}

#[tokio::test]
async fn ai_01_an_error_of_the_provider_never_reaches_the_person() -> Outcome {
    let world = World::new().await?;
    a_local_model(&world).await?;
    change_ai_setup(
        &world.ctx(),
        ChangeAiSetup {
            enabled: Some(true),
            ..setup(&world)
        },
    )
    .await?;
    world.ai.will_fail();
    let error = world.ctx().ask_ai(None, &a_question()).await.unwrap_err();
    assert_eq!(error, AppError::Refused(ErrorCode::AiUnavailable));
    assert_eq!(
        bb_core::message::Message::from(ErrorCode::AiUnavailable.message()).render(),
        "The AI provider did not answer."
    );

    world.ai.will_fail();
    let error = try_ai_provider(
        &world.ctx(),
        TryAiProvider {
            session_token: token(&world),
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error, AppError::Refused(ErrorCode::AiUnavailable));

    // A model that answers is not an error, and a second try within seconds waits.
    world.ai.will_answer(AiAnswer {
        text: "OK".to_owned(),
        ..AiAnswer::default()
    });
    let waited = try_ai_provider(
        &world.ctx(),
        TryAiProvider {
            session_token: token(&world),
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(waited, AppError::Limited { .. }), "{waited:?}");
    Ok(())
}

#[tokio::test]
async fn ai_01_changing_the_address_asks_for_the_password_again() -> Outcome {
    let world = World::new().await?;
    a_local_model(&world).await?;
    let change = |password: Option<&str>| ChangeAiSetup {
        address: Some("http://other.lan/v1".to_owned()),
        password: password.map(str::to_owned),
        ..setup(&world)
    };
    for password in [None, Some(OTHER_PASSWORD)] {
        let error = change_ai_setup(&world.ctx(), change(password))
            .await
            .unwrap_err();
        assert!(refused(&error, ErrorCode::WrongCredentials), "{error:?}");
    }
    let changed = change_ai_setup(&world.ctx(), change(Some(PASSWORD)))
        .await?
        .into_value();
    assert_eq!(changed.address, "http://other.lan/v1");

    // The model and the names need no password; the provider does.
    change_ai_setup(
        &world.ctx(),
        ChangeAiSetup {
            model: Some("another-model".to_owned()),
            ..setup(&world)
        },
    )
    .await?;
    let error = change_ai_setup(
        &world.ctx(),
        ChangeAiSetup {
            provider: Some(ProviderKind::Anthropic),
            ..setup(&world)
        },
    )
    .await
    .unwrap_err();
    assert!(refused(&error, ErrorCode::WrongCredentials), "{error:?}");
    Ok(())
}

#[tokio::test]
async fn ai_01_an_address_on_the_internet_must_be_https() -> Outcome {
    let world = World::new().await?;
    for (address, code) in [
        (
            "http://models.example.test/v1",
            ErrorCode::AiAddressNotEncrypted,
        ),
        ("models.example.test", ErrorCode::AiAddressInvalid),
    ] {
        let error = change_ai_setup(
            &world.ctx(),
            ChangeAiSetup {
                provider: Some(ProviderKind::OpenAiCompatible),
                address: Some(address.to_owned()),
                password: Some(PASSWORD.to_owned()),
                ..setup(&world)
            },
        )
        .await
        .unwrap_err();
        assert!(refused(&error, code), "{address}: {error:?}");
    }
    Ok(())
}
