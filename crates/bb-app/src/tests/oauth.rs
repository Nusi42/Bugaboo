//! How an AI app signs in and what its token may do (AI-22, AI-23, ADR-0042): registration, the
//! consent step, PKCE, the turning over of tokens, taking back, and the scope that only narrows.

use bb_auth::pkce_challenge;
use bb_core::ErrorCode;
use bb_core::access_key::KeyChoice;
use bb_core::ai::{ToolCaller, ToolScope};
use bb_core::id::OAuthClientId;
use bb_core::oauth::OAuthProblem;
use serde_json::json;

use super::ai_tools::call;
use super::scenarios::{a_project, text, token};
use super::scenarios_ai::a_local_model;
use super::world::{Outcome, World};
use crate::ai::tools::{ToolListQuery, call_tool, list_tools};
use crate::ai::{ChangeAiSetup, change_ai_setup};
use crate::auth::{
    AddWebAddress, AppError, AuthorizationQuery, AuthorizationRequest, ConnectedAppsQuery,
    DecideAuthorization, RegisterApp, RegisterOutcome, TakeBack, TokenOutcome, TokenRequest,
    add_web_address, authorization, connected_apps, decide_authorization, exchange_token,
    register_app, sign_out_everywhere, take_back,
};

pub(super) const LOOPBACK: &str = "http://127.0.0.1/callback";
pub(super) const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";

/// AI is on for the organization (a local model needs no agreement).
pub(super) async fn ai_on(world: &World) -> Result<(), String> {
    a_local_model(world).await?;
    change_ai_setup(
        &world.ctx(),
        ChangeAiSetup {
            enabled: Some(true),
            ..super::scenarios_ai::setup(world)
        },
    )
    .await
    .map_err(text)?;
    Ok(())
}

pub(super) async fn registered(world: &World, address: &str) -> Result<OAuthClientId, AppError> {
    match register_app(
        &world.ctx(),
        RegisterApp {
            name: "Claude Code".to_owned(),
            return_addresses: vec![address.to_owned()],
        },
    )
    .await?
    {
        RegisterOutcome::Registered(app) => Ok(app.client_id),
        RegisterOutcome::Refused(problem) => Err(AppError::Internal(problem.code().to_owned())),
    }
}

pub(super) fn request(client: OAuthClientId, scope: &str) -> AuthorizationRequest {
    AuthorizationRequest {
        client_id: client.to_string(),
        response_type: "code".to_owned(),
        return_address: "http://127.0.0.1:51234/callback".to_owned(),
        challenge: pkce_challenge(VERIFIER),
        challenge_method: "S256".to_owned(),
        scope: scope.to_owned(),
        resource: String::new(),
        state: "s1".to_owned(),
    }
}

/// The code an app gets after the person allowed it.
pub(super) async fn allowed(
    world: &World,
    client: OAuthClientId,
    asked: &str,
    given: KeyChoice,
) -> Result<String, AppError> {
    let answer = decide_authorization(
        &world.ctx(),
        DecideAuthorization {
            session_token: token(world),
            request: request(client, asked),
            allow: true,
            scope: given,
            projects: Vec::new(),
        },
    )
    .await?
    .into_value();
    let query = answer
        .redirect_to
        .split_once('?')
        .map_or("", |(_, rest)| rest);
    let pairs = bb_core::oauth::form_pairs(query).ok_or(ErrorCode::InvalidRequest)?;
    assert!(pairs.contains(&("state".to_owned(), "s1".to_owned())));
    Ok(pairs
        .into_iter()
        .find(|(name, _)| name == "code")
        .map(|(_, code)| code)
        .ok_or(ErrorCode::InvalidRequest)?)
}

pub(super) fn swap_code(client: OAuthClientId, code: &str, verifier: &str) -> TokenRequest {
    TokenRequest {
        grant_type: "authorization_code".to_owned(),
        client_id: client.to_string(),
        code: code.to_owned(),
        return_address: "http://127.0.0.1:51234/callback".to_owned(),
        verifier: verifier.to_owned(),
        refresh_token: String::new(),
        resource: String::new(),
    }
}

fn renew(client: OAuthClientId, refresh: &str) -> TokenRequest {
    TokenRequest {
        grant_type: "refresh_token".to_owned(),
        client_id: client.to_string(),
        code: String::new(),
        return_address: String::new(),
        verifier: String::new(),
        refresh_token: refresh.to_owned(),
        resource: String::new(),
    }
}

async fn issued(
    world: &World,
    request: TokenRequest,
) -> Result<crate::auth::TokenAnswer, AppError> {
    match exchange_token(&world.ctx(), request).await? {
        TokenOutcome::Issued(answer) => Ok(answer),
        TokenOutcome::Refused(problem) => Err(AppError::Internal(problem.code().to_owned())),
    }
}

async fn refused(world: &World, request: TokenRequest) -> Result<OAuthProblem, AppError> {
    match exchange_token(&world.ctx(), request).await? {
        TokenOutcome::Refused(problem) => Ok(problem),
        TokenOutcome::Issued(_) => Err(AppError::Internal("issued".to_owned())),
    }
}

/// An app that was allowed, with its tokens.
pub(super) async fn signed_in(
    world: &World,
    given: KeyChoice,
) -> Result<(OAuthClientId, crate::auth::TokenAnswer), AppError> {
    let client = registered(world, LOOPBACK).await?;
    let code = allowed(world, client, "see-and-change", given).await?;
    let tokens = issued(world, swap_code(client, &code, VERIFIER)).await?;
    Ok((client, tokens))
}

pub(super) fn app_call(
    access: &str,
    tool: &str,
    input: serde_json::Value,
) -> crate::ai::tools::ToolCallQuery {
    crate::ai::tools::ToolCallQuery {
        caller: ToolCaller::AiApp,
        app: Some("Claude Code".to_owned()),
        ..call(access, tool, input)
    }
}

#[tokio::test]
async fn ai_22_an_app_on_this_computer_registers_and_the_person_allows_it() -> Outcome {
    let world = World::new().await?;
    ai_on(&world).await?;
    let client = registered(&world, LOOPBACK).await?;
    let seen = authorization(
        &world.ctx(),
        AuthorizationQuery {
            session_token: token(&world),
            request: request(client, "see-and-change"),
        },
    )
    .await?
    .into_value();
    assert_eq!(seen.app_name, "Claude Code");
    assert!(seen.on_this_computer);
    assert_eq!(seen.asked, KeyChoice::SeeAndChange);
    // Cancel sends the app away with `access_denied` and makes no grant.
    let cancelled = decide_authorization(
        &world.ctx(),
        DecideAuthorization {
            session_token: token(&world),
            request: request(client, "see-only"),
            allow: false,
            scope: KeyChoice::SeeOnly,
            projects: Vec::new(),
        },
    )
    .await?
    .into_value();
    assert_eq!(
        cancelled.redirect_to,
        "http://127.0.0.1:51234/callback?error=access_denied&state=s1"
    );
    let listed = connected_apps(
        &world.ctx(),
        ConnectedAppsQuery {
            session_token: token(&world),
        },
    )
    .await?
    .into_value();
    assert!(listed.items.is_empty());
    Ok(())
}

#[tokio::test]
async fn ai_22_apps_are_refused_while_ai_is_off_or_apps_are_switched_off() -> Outcome {
    let world = World::new().await?;
    let off = register_app(
        &world.ctx(),
        RegisterApp {
            name: "Claude Code".to_owned(),
            return_addresses: vec![LOOPBACK.to_owned()],
        },
    )
    .await?;
    assert_eq!(
        off,
        RegisterOutcome::Refused(OAuthProblem::TemporarilyUnavailable)
    );
    Ok(())
}

#[tokio::test]
async fn ai_22_an_app_on_a_web_address_needs_an_administrator_first() -> Outcome {
    let world = World::new().await?;
    ai_on(&world).await?;
    let web = "https://app.example.com/callback";
    let wanted = || RegisterApp {
        name: "Web app".to_owned(),
        return_addresses: vec![web.to_owned()],
    };
    assert_eq!(
        register_app(&world.ctx(), wanted()).await?,
        RegisterOutcome::Refused(OAuthProblem::InvalidRedirectUri)
    );
    // Other addresses are refused whatever an administrator does.
    for bad in ["http://localhost/cb", "http://example.com/cb", "myapp://cb"] {
        let refused = register_app(
            &world.ctx(),
            RegisterApp {
                name: "App".to_owned(),
                return_addresses: vec![bad.to_owned()],
            },
        )
        .await?;
        assert_eq!(
            refused,
            RegisterOutcome::Refused(OAuthProblem::InvalidRedirectUri),
            "{bad}"
        );
    }
    // A person without the right to change the settings cannot allow it.
    let bob = world.person("bob@example.test").await?;
    let denied = add_web_address(
        &world.ctx(),
        AddWebAddress {
            session_token: bob.session_token,
            address: web.to_owned(),
        },
    )
    .await;
    assert_eq!(denied.err(), Some(AppError::Refused(ErrorCode::Forbidden)));
    add_web_address(
        &world.ctx(),
        AddWebAddress {
            session_token: token(&world),
            address: web.to_owned(),
        },
    )
    .await?;
    assert!(matches!(
        register_app(&world.ctx(), wanted()).await?,
        RegisterOutcome::Registered(_)
    ));
    Ok(())
}

#[tokio::test]
async fn ai_22_a_code_without_its_pkce_proof_is_refused() -> Outcome {
    let world = World::new().await?;
    ai_on(&world).await?;
    let client = registered(&world, LOOPBACK).await?;
    let code = allowed(&world, client, "see-only", KeyChoice::SeeOnly).await?;
    let wrong = "wrong-wrong-wrong-wrong-wrong-wrong-wrong-wrong-1";
    assert_eq!(
        refused(&world, swap_code(client, &code, wrong)).await?,
        OAuthProblem::InvalidGrant
    );
    assert_eq!(
        refused(&world, swap_code(client, &code, "")).await?,
        OAuthProblem::InvalidRequest
    );
    // The code was not burned by the wrong tries: the app with the proof still gets its tokens.
    issued(&world, swap_code(client, &code, VERIFIER)).await?;
    // Another return address than the one the code was given for does not work either.
    let second = allowed(&world, client, "see-only", KeyChoice::SeeOnly).await?;
    let mut elsewhere = swap_code(client, &second, VERIFIER);
    "http://127.0.0.1:1/other".clone_into(&mut elsewhere.return_address);
    assert_eq!(
        refused(&world, elsewhere).await?,
        OAuthProblem::InvalidGrant
    );
    Ok(())
}

#[tokio::test]
async fn ai_22_an_address_the_app_did_not_register_is_never_sent_anything() -> Outcome {
    let world = World::new().await?;
    ai_on(&world).await?;
    let client = registered(&world, LOOPBACK).await?;
    let mut elsewhere = request(client, "see-only");
    "https://evil.example.com/steal".clone_into(&mut elsewhere.return_address);
    let asked = authorization(
        &world.ctx(),
        AuthorizationQuery {
            session_token: token(&world),
            request: elsewhere,
        },
    )
    .await;
    assert_eq!(asked.err(), Some(AppError::Refused(ErrorCode::NotFound)));
    let mut plain = request(client, "see-only");
    "plain".clone_into(&mut plain.challenge_method);
    let asked = authorization(
        &world.ctx(),
        AuthorizationQuery {
            session_token: token(&world),
            request: plain,
        },
    )
    .await;
    assert_eq!(
        asked.err(),
        Some(AppError::Refused(ErrorCode::InvalidRequest))
    );
    Ok(())
}

#[tokio::test]
async fn ai_22_a_code_used_twice_ends_the_grant_it_made() -> Outcome {
    let world = World::new().await?;
    ai_on(&world).await?;
    let client = registered(&world, LOOPBACK).await?;
    let code = allowed(&world, client, "see-only", KeyChoice::SeeOnly).await?;
    let tokens = issued(&world, swap_code(client, &code, VERIFIER)).await?;
    assert_eq!(
        refused(&world, swap_code(client, &code, VERIFIER)).await?,
        OAuthProblem::InvalidGrant
    );
    // The grant ended with it: the first token is dead too.
    let listed = list_tools(
        &world.ctx(),
        ToolListQuery {
            session_token: tokens.access_token,
            granted: ToolScope::SeeAndChange,
        },
    )
    .await;
    assert_eq!(
        listed.err(),
        Some(AppError::Refused(ErrorCode::NotSignedIn))
    );
    Ok(())
}

#[tokio::test]
async fn ai_22_a_refresh_token_used_twice_ends_the_grant() -> Outcome {
    let world = World::new().await?;
    ai_on(&world).await?;
    let (client, first) = signed_in(&world, KeyChoice::SeeAndChange).await?;
    let second = issued(&world, renew(client, &first.refresh_token)).await?;
    assert_ne!(second.access_token, first.access_token);
    assert_ne!(second.refresh_token, first.refresh_token);
    // The new tokens work.
    list_tools(
        &world.ctx(),
        ToolListQuery {
            session_token: second.access_token.clone(),
            granted: ToolScope::SeeAndChange,
        },
    )
    .await?;
    // The old renewing token used again: two parties hold it, and the whole grant ends.
    assert_eq!(
        refused(&world, renew(client, &first.refresh_token)).await?,
        OAuthProblem::InvalidGrant
    );
    for dead in [&second.access_token, &first.access_token] {
        let listed = list_tools(
            &world.ctx(),
            ToolListQuery {
                session_token: dead.clone(),
                granted: ToolScope::SeeAndChange,
            },
        )
        .await;
        assert_eq!(
            listed.err(),
            Some(AppError::Refused(ErrorCode::NotSignedIn))
        );
    }
    assert_eq!(
        refused(&world, renew(client, &second.refresh_token)).await?,
        OAuthProblem::InvalidGrant
    );
    Ok(())
}

#[tokio::test]
async fn ai_22_an_access_token_lives_fifteen_minutes_and_a_refresh_token_thirty_days() -> Outcome {
    let world = World::new().await?;
    ai_on(&world).await?;
    let (client, tokens) = signed_in(&world, KeyChoice::SeeOnly).await?;
    assert_eq!(tokens.expires_in, 15 * 60);
    world.clock.advance(15 * 60 + 1);
    let late = list_tools(
        &world.ctx(),
        ToolListQuery {
            session_token: tokens.access_token.clone(),
            granted: ToolScope::SeeOnly,
        },
    )
    .await;
    assert_eq!(late.err(), Some(AppError::Refused(ErrorCode::NotSignedIn)));
    let renewed = issued(&world, renew(client, &tokens.refresh_token)).await?;
    world.clock.advance(31 * 24 * 3600);
    assert_eq!(
        refused(&world, renew(client, &renewed.refresh_token)).await?,
        OAuthProblem::InvalidGrant
    );
    Ok(())
}

#[tokio::test]
async fn ai_22_a_see_only_grant_cannot_call_change_status() -> Outcome {
    let world = World::new().await?;
    ai_on(&world).await?;
    a_project(&world, "WEB").await?;
    let (_, tokens) = signed_in(&world, KeyChoice::SeeOnly).await?;
    // The app asked for more than the person gave: the narrower scope counts.
    let tools = list_tools(
        &world.ctx(),
        ToolListQuery {
            session_token: tokens.access_token.clone(),
            granted: ToolScope::SeeAndChange,
        },
    )
    .await?
    .into_value();
    assert!(
        tools
            .tools
            .iter()
            .all(|tool| tool.effect.scope() == ToolScope::SeeOnly)
    );
    let refused_call = call_tool(
        &world.ctx(),
        app_call(
            &tokens.access_token,
            "change_status",
            json!({"issue": "WEB-1", "status": "done", "version": 0}),
        ),
    )
    .await;
    assert_eq!(
        refused_call.err(),
        Some(AppError::Refused(ErrorCode::Forbidden))
    );
    assert!(
        world
            .store
            .lock()
            .log
            .iter()
            .any(|entry| entry.action == bb_core::sign_in::SecurityAction::AiToolRefused)
    );
    Ok(())
}

#[tokio::test]
async fn ai_22_the_person_may_narrow_what_the_app_asked_and_never_widen_it() -> Outcome {
    let world = World::new().await?;
    ai_on(&world).await?;
    let client = registered(&world, LOOPBACK).await?;
    let wider = decide_authorization(
        &world.ctx(),
        DecideAuthorization {
            session_token: token(&world),
            request: request(client, "see-only"),
            allow: true,
            scope: KeyChoice::SeeAndChange,
            projects: Vec::new(),
        },
    )
    .await;
    assert_eq!(
        wider.err(),
        Some(AppError::Refused(ErrorCode::InvalidRequest))
    );
    let everything = decide_authorization(
        &world.ctx(),
        DecideAuthorization {
            session_token: token(&world),
            request: request(client, "see-and-change"),
            allow: true,
            scope: KeyChoice::Everything,
            projects: Vec::new(),
        },
    )
    .await;
    assert_eq!(
        everything.err(),
        Some(AppError::Refused(ErrorCode::InvalidRequest))
    );
    let everything_asked = authorization(
        &world.ctx(),
        AuthorizationQuery {
            session_token: token(&world),
            request: request(client, "everything"),
        },
    )
    .await;
    assert_eq!(
        everything_asked.err(),
        Some(AppError::Refused(ErrorCode::InvalidRequest))
    );
    allowed(&world, client, "see-and-change", KeyChoice::SeeOnly).await?;
    Ok(())
}

#[tokio::test]
async fn ai_24_connected_apps_lists_an_app_and_take_back_ends_it() -> Outcome {
    let world = World::new().await?;
    ai_on(&world).await?;
    let (_, tokens) = signed_in(&world, KeyChoice::SeeAndChange).await?;
    list_tools(
        &world.ctx(),
        ToolListQuery {
            session_token: tokens.access_token.clone(),
            granted: ToolScope::SeeAndChange,
        },
    )
    .await?;
    let listed = connected_apps(
        &world.ctx(),
        ConnectedAppsQuery {
            session_token: token(&world),
        },
    )
    .await?
    .into_value();
    let [app] = listed.items.as_slice() else {
        return Err("one app".into());
    };
    assert_eq!(app.name, "Claude Code");
    assert!(app.on_this_computer);
    assert_eq!(app.scope, KeyChoice::SeeAndChange);
    assert!(app.last_used_at.is_some());
    // Another person cannot take it back, and cannot tell it exists.
    let bob = world.person("bob@example.test").await?;
    let not_theirs = take_back(
        &world.ctx(),
        TakeBack {
            session_token: bob.session_token,
            id: app.id,
        },
    )
    .await;
    assert_eq!(
        not_theirs.err(),
        Some(AppError::Refused(ErrorCode::NotFound))
    );
    take_back(
        &world.ctx(),
        TakeBack {
            session_token: token(&world),
            id: app.id,
        },
    )
    .await?;
    let dead = list_tools(
        &world.ctx(),
        ToolListQuery {
            session_token: tokens.access_token,
            granted: ToolScope::SeeAndChange,
        },
    )
    .await;
    assert_eq!(dead.err(), Some(AppError::Refused(ErrorCode::NotSignedIn)));
    let none = connected_apps(
        &world.ctx(),
        ConnectedAppsQuery {
            session_token: token(&world),
        },
    )
    .await?
    .into_value();
    assert!(none.items.is_empty());
    Ok(())
}

#[tokio::test]
async fn ai_22_sign_out_everywhere_ends_every_grant() -> Outcome {
    let world = World::new().await?;
    ai_on(&world).await?;
    let (_, tokens) = signed_in(&world, KeyChoice::SeeAndChange).await?;
    let (_, other) = signed_in(&world, KeyChoice::SeeOnly).await?;
    sign_out_everywhere(
        &world.ctx(),
        crate::auth::SignOutEverywhere {
            session_token: token(&world),
        },
    )
    .await?;
    for dead in [tokens.access_token, other.access_token] {
        let listed = list_tools(
            &world.ctx(),
            ToolListQuery {
                session_token: dead,
                granted: ToolScope::SeeAndChange,
            },
        )
        .await;
        assert_eq!(
            listed.err(),
            Some(AppError::Refused(ErrorCode::NotSignedIn))
        );
    }
    let log = world.store.lock().log.clone();
    let ended = log
        .iter()
        .filter(|entry| entry.action == bb_core::sign_in::SecurityAction::OAuthGrantEndedBySignOut)
        .count();
    assert_eq!(ended, 2, "each grant has its own entry");
    Ok(())
}

#[tokio::test]
async fn ai_22_the_token_of_an_app_is_not_a_session() -> Outcome {
    let world = World::new().await?;
    ai_on(&world).await?;
    let (_, tokens) = signed_in(&world, KeyChoice::SeeAndChange).await?;
    // Where only a session goes (the consent step, Connected apps), a token opens nothing.
    let listed = connected_apps(
        &world.ctx(),
        ConnectedAppsQuery {
            session_token: tokens.access_token.clone(),
        },
    )
    .await;
    assert!(listed.is_err());
    let decided = decide_authorization(
        &world.ctx(),
        DecideAuthorization {
            session_token: tokens.access_token,
            request: request(OAuthClientId::from_random([1; 16]), "see-only"),
            allow: true,
            scope: KeyChoice::SeeOnly,
            projects: Vec::new(),
        },
    )
    .await;
    assert!(decided.is_err());
    Ok(())
}

#[tokio::test]
async fn ai_22_switching_apps_off_refuses_the_grants_but_keeps_them() -> Outcome {
    let world = World::new().await?;
    ai_on(&world).await?;
    let (_, tokens) = signed_in(&world, KeyChoice::SeeAndChange).await?;
    let setting = |on| crate::settings::ChangeSetting {
        version: None,
        session_token: token(&world),
        scope: crate::settings::SettingsScope::Organization,
        key: "ai-apps.enabled".to_owned(),
        value: bb_core::settings::SettingValue::Bool(on),
    };
    crate::settings::change_setting(&world.ctx(), setting(false)).await?;
    let refused_now = list_tools(
        &world.ctx(),
        ToolListQuery {
            session_token: tokens.access_token.clone(),
            granted: ToolScope::SeeAndChange,
        },
    )
    .await;
    assert_eq!(
        refused_now.err(),
        Some(AppError::Refused(ErrorCode::NotSignedIn))
    );
    crate::settings::change_setting(&world.ctx(), setting(true)).await?;
    list_tools(
        &world.ctx(),
        ToolListQuery {
            session_token: tokens.access_token,
            granted: ToolScope::SeeAndChange,
        },
    )
    .await?;
    Ok(())
}
