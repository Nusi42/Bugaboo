//! The password guards the grants of AI apps (SEC-12, ADR-0042): *Allow* asks for it again, and a
//! new password ends every grant, so a stolen session cannot leave an app behind that outlives it.

use bb_core::ErrorCode;
use bb_core::access_key::KeyChoice;
use bb_core::ai::{ToolCaller, ToolScope};
use bb_core::sign_in::SecurityAction;

use super::oauth::{LOOPBACK, ai_on, registered, request, signed_in};
use super::scenarios::token;
use super::world::{OTHER_PASSWORD, Outcome, PASSWORD, World};
use crate::ai::tools::{ToolListQuery, list_tools};
use crate::auth::{
    AppError, ChangePassword, ConnectedAppsQuery, DecideAuthorization, change_password,
    connected_apps, decide_authorization,
};

#[tokio::test]
async fn sec_12_a_stolen_session_cannot_allow_an_app_without_the_password() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    ai_on(&world).await?;
    let client = registered(&world, LOOPBACK).await?;
    let allow = |password: &str| DecideAuthorization {
        session_token: token(&world),
        request: request(client, "see-and-change"),
        allow: true,
        scope: KeyChoice::SeeAndChange,
        projects: Vec::new(),
        password: password.to_owned(),
    };
    // A thief holds the session, not the password: no password and a wrong one make no grant.
    for typed in ["", OTHER_PASSWORD] {
        let refused = decide_authorization(&world.ctx(), allow(typed)).await;
        assert_eq!(
            refused.err(),
            Some(AppError::Refused(ErrorCode::WrongCredentials)),
            "{typed:?}"
        );
    }
    let listed = connected_apps(
        &world.ctx(),
        ConnectedAppsQuery {
            session_token: token(&world),
        },
    )
    .await?
    .into_value();
    assert!(listed.items.is_empty());
    // The person who types it allows the app.
    decide_authorization(&world.ctx(), allow(PASSWORD)).await?;
    // *Cancel* takes nothing away, so it asks for nothing.
    decide_authorization(
        &world.ctx(),
        DecideAuthorization {
            allow: false,
            ..allow("")
        },
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn sec_12_a_new_password_ends_every_grant_of_an_app() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    ai_on(&world).await?;
    let (_, tokens) = signed_in(&world, KeyChoice::SeeAndChange).await?;
    change_password(
        &world.ctx(),
        ChangePassword {
            session_token: token(&world),
            current_password: PASSWORD.to_owned(),
            new_password: OTHER_PASSWORD.to_owned(),
        },
    )
    .await?;
    let listed = list_tools(
        &world.ctx(),
        ToolListQuery {
            session_token: tokens.access_token.to_string(),
            granted: ToolScope::SeeAndChange,
            caller: ToolCaller::AiApp,
        },
    )
    .await;
    assert_eq!(
        listed.err(),
        Some(AppError::Refused(ErrorCode::NotSignedIn))
    );
    let log = world.store.lock().log.clone();
    let ended = log
        .iter()
        .filter(|entry| entry.action == SecurityAction::OAuthGrantTakenBack)
        .count();
    assert_eq!(ended, 1, "the grant that ended is in the security log");
    Ok(())
}

#[tokio::test]
async fn sec_12_a_person_without_a_password_is_told_so_and_not_that_it_is_wrong() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    ai_on(&world).await?;
    let client = registered(&world, LOOPBACK).await?;
    // The person signs in another way (a passkey, company sign-in): there is no password to check.
    for (_, hash) in &mut world.store.lock().state.people {
        *hash = None;
    }
    for typed in ["", PASSWORD] {
        let refused = decide_authorization(
            &world.ctx(),
            DecideAuthorization {
                session_token: token(&world),
                request: request(client, "see-and-change"),
                allow: true,
                scope: KeyChoice::SeeOnly,
                projects: Vec::new(),
                password: typed.to_owned(),
            },
        )
        .await;
        assert_eq!(
            refused.err(),
            Some(AppError::Refused(ErrorCode::PasswordNeeded)),
            "{typed:?}"
        );
    }
    Ok(())
}
