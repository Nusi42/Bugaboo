//! The limits around the sign-in of AI apps (AI-22, ADR-0042): how often a network may register
//! apps, what happens to the days without use while AI apps are off, a grant that ran out, and who
//! may come in at the entry of MCP at all.

use bb_core::ErrorCode;
use bb_core::access_key::KeyChoice;
use bb_core::oauth::{ENDED_GRANT_KEEP_DAYS, REGISTRATIONS_AN_HOUR};

use super::oauth::{LOOPBACK, ai_on, issued, renew, signed_in};
use super::scenarios::token;
use super::world::{Outcome, PASSWORD, World};
use crate::auth::{
    AppError, McpIdentityQuery, OAuthStore as _, RegisterApp, RegisterOutcome, SignIn,
    clean_old_grants, mcp_identity, register_app, sign_in,
};
use crate::settings::{ChangeSetting, SettingsScope, change_setting};

async fn apps_on(world: &World, on: bool) -> Result<(), AppError> {
    change_setting(
        &world.ctx(),
        ChangeSetting {
            version: None,
            session_token: token(world),
            scope: SettingsScope::Organization,
            key: "ai-apps.enabled".to_owned(),
            value: bb_core::settings::SettingValue::Bool(on),
        },
    )
    .await
    .map(|_| ())
}

fn identity(token: &str) -> McpIdentityQuery {
    McpIdentityQuery {
        token: token.to_owned(),
    }
}

#[tokio::test]
async fn ai_22_a_network_registers_only_so_many_apps_in_an_hour() -> Outcome {
    let world = World::new().await?;
    ai_on(&world).await?;
    let register = async || {
        register_app(
            &world.ctx(),
            RegisterApp {
                name: "Claude Code".to_owned(),
                return_addresses: vec![LOOPBACK.to_owned()],
            },
        )
        .await
    };
    for _ in 0..REGISTRATIONS_AN_HOUR {
        assert!(matches!(register().await?, RegisterOutcome::Registered(_)));
    }
    assert!(matches!(
        register().await.err(),
        Some(AppError::Limited { .. })
    ));
    Ok(())
}

#[tokio::test]
async fn ai_22_while_apps_are_off_nobody_comes_in_at_mcp_and_it_is_no_guess() -> Outcome {
    let world = World::new().await?;
    ai_on(&world).await?;
    let (_, tokens) = signed_in(&world, KeyChoice::SeeAndChange).await?;
    mcp_identity(&world.ctx(), identity(&tokens.access_token)).await?;
    apps_on(&world, false).await?;
    // The token of an app, an access key and the secret of a session: all turned away, and so many
    // tries do not make the network wait, because switching apps off is not a guess.
    let session = token(&world);
    for _ in 0..40 {
        for secret in [tokens.access_token.to_string(), session.clone()] {
            let refused = mcp_identity(&world.ctx(), identity(&secret)).await;
            assert_eq!(
                refused.err(),
                Some(AppError::Refused(ErrorCode::NotSignedIn))
            );
        }
    }
    apps_on(&world, true).await?;
    mcp_identity(&world.ctx(), identity(&tokens.access_token)).await?;
    Ok(())
}

#[tokio::test]
async fn ai_22_the_secret_of_a_session_does_not_open_mcp() -> Outcome {
    let world = World::new().await?;
    ai_on(&world).await?;
    let refused = mcp_identity(&world.ctx(), identity(&token(&world))).await;
    assert_eq!(
        refused.err(),
        Some(AppError::Refused(ErrorCode::NotSignedIn))
    );
    Ok(())
}

#[tokio::test]
async fn ai_22_the_days_without_use_do_not_count_while_apps_are_off() -> Outcome {
    let world = World::new().await?;
    ai_on(&world).await?;
    let (client, tokens) = signed_in(&world, KeyChoice::SeeAndChange).await?;
    apps_on(&world, false).await?;
    // Ten days off, a clean-up each day: nothing is deleted and the renewing token is moved on.
    for _ in 0..10 {
        world.clock.advance(24 * 3600);
        let cleaned = clean_old_grants(&world.ctx()).await?;
        assert_eq!(cleaned.grants + cleaned.clients, 0);
    }
    // The administrator comes back on the tenth day, with a new session, and switches apps on.
    let back = sign_in(
        &world.ctx(),
        SignIn {
            email: "ada@example.test".to_owned(),
            password: PASSWORD.to_owned(),
            previous_session: None,
        },
    )
    .await?;
    change_setting(
        &world.ctx(),
        ChangeSetting {
            version: None,
            session_token: back.session_token,
            scope: SettingsScope::Organization,
            key: "ai-apps.enabled".to_owned(),
            value: bb_core::settings::SettingValue::Bool(true),
        },
    )
    .await?;
    // Thirty-five days after it was made the token would be dead; the ten days off were not counted.
    world.clock.advance(25 * 24 * 3600);
    assert!(
        issued(&world, renew(client, &tokens.refresh_token))
            .await
            .is_ok()
    );
    Ok(())
}

#[tokio::test]
async fn ai_22_a_grant_past_its_end_is_not_listed_nor_counted_and_the_clean_up_deletes_it()
-> Outcome {
    let world = World::new().await?;
    ai_on(&world).await?;
    signed_in(&world, KeyChoice::SeeAndChange).await?;
    let person = world.admin.person.id;
    let at = |world: &World| crate::context::Clock::now(&*world.clock);
    assert_eq!(
        world
            .store
            .grant_count_of(person, at(&world))
            .await
            .map_err(|error| format!("{error:?}"))?,
        1
    );
    // The grant lasts as long as the longest time an access key may (365 days by default).
    world.clock.advance(366 * 24 * 3600);
    assert_eq!(
        world
            .store
            .grant_count_of(person, at(&world))
            .await
            .map_err(|error| format!("{error:?}"))?,
        0
    );
    assert!(
        world
            .store
            .grants_of(person, at(&world), 10)
            .await
            .map_err(|error| format!("{error:?}"))?
            .is_empty()
    );
    // It is kept for a while, then the clean-up deletes it.
    world.clock.advance((ENDED_GRANT_KEEP_DAYS + 1) * 24 * 3600);
    let cleaned = clean_old_grants(&world.ctx()).await?;
    assert_eq!(cleaned.grants, 1);
    Ok(())
}

#[tokio::test]
async fn ai_22_the_pause_adds_only_the_time_that_passed_however_often_it_runs() -> Outcome {
    use bb_auth::{OAuthKind, OAuthSecret};
    let world = World::new().await?;
    ai_on(&world).await?;
    let (_, tokens) = signed_in(&world, KeyChoice::SeeAndChange).await?;
    let hash = OAuthSecret::parse(OAuthKind::Refresh, &tokens.refresh_token)
        .ok_or("no refresh token")?
        .hash();
    let ends = async |world: &World| -> Result<i64, String> {
        Ok(world
            .store
            .oauth_token(&hash)
            .await
            .map_err(|error| format!("{error:?}"))?
            .ok_or("no token")?
            .expires_at
            .as_second())
    };
    let at_first = ends(&world).await?;
    apps_on(&world, false).await?;
    // Ten runs in a row, at the start of the server for instance: no time passed, nothing is added.
    for _ in 0..10 {
        clean_old_grants(&world.ctx()).await?;
    }
    assert_eq!(ends(&world).await?, at_first);
    // Half a day later, then runs again: exactly the half day.
    world.clock.advance(12 * 3600);
    for _ in 0..3 {
        clean_old_grants(&world.ctx()).await?;
    }
    assert_eq!(ends(&world).await?, at_first + 12 * 3600);
    Ok(())
}

#[tokio::test]
async fn ai_22_apps_switched_off_is_no_failed_try_at_registration_or_the_token_endpoint() -> Outcome
{
    let world = World::new().await?;
    ai_on(&world).await?;
    apps_on(&world, false).await?;
    // Far more tries than a network may fail in a minute: every one is answered the same way, and
    // none is answered with a wait.
    for _ in 0..60 {
        let registered = register_app(
            &world.ctx(),
            RegisterApp {
                name: "Claude Code".to_owned(),
                return_addresses: vec![LOOPBACK.to_owned()],
            },
        )
        .await?;
        assert_eq!(
            registered,
            RegisterOutcome::Refused(bb_core::oauth::OAuthProblem::TemporarilyUnavailable)
        );
        let swapped = crate::auth::exchange_token(
            &world.ctx(),
            super::oauth::swap_code(
                bb_core::id::OAuthClientId::from_random([1; 16]),
                "bbc_nothing",
                "v",
            ),
        )
        .await?;
        assert!(matches!(
            swapped,
            crate::auth::TokenOutcome::Refused(
                bb_core::oauth::OAuthProblem::TemporarilyUnavailable
            )
        ));
    }
    apps_on(&world, true).await?;
    // And once apps are on again the same network registers at once.
    let back = register_app(
        &world.ctx(),
        RegisterApp {
            name: "Claude Code".to_owned(),
            return_addresses: vec![LOOPBACK.to_owned()],
        },
    )
    .await?;
    assert!(matches!(back, RegisterOutcome::Registered(_)));
    Ok(())
}

#[tokio::test]
async fn ai_22_an_access_key_and_an_agent_key_are_turned_away_at_mcp_while_apps_or_ai_are_off()
-> Outcome {
    use super::agents::{an_agent, key_of};
    let world = World::new().await?;
    ai_on(&world).await?;
    let session = token(&world);
    let key = crate::auth::create_access_key(
        &world.ctx(),
        crate::auth::CreateAccessKey {
            session_token: session.clone(),
            password: PASSWORD.to_owned(),
            name: "For an app".to_owned(),
            choice: KeyChoice::SeeAndChange,
            permissions: Vec::new(),
            projects: Vec::new(),
            expires_at: None,
            agent: None,
            renews: None,
        },
    )
    .await
    .map_err(|e| format!("person key {e:?}"))?
    .into_value()
    .secret;
    let project = super::scenarios::a_project(&world, "WEB")
        .await
        .map_err(|e| format!("project {e:?}"))?;
    let agent = an_agent(&world, &session, "Fixer")
        .await
        .map_err(|e| format!("agent {e:?}"))?;
    super::agents::give_work(&world, agent, project);
    let agent_key = key_of(&world, &session, agent)
        .await
        .map_err(|e| format!("agent key {e:?}"))?;
    for secret in [&key, &agent_key] {
        mcp_identity(&world.ctx(), identity(secret))
            .await
            .map_err(|e| format!("first {e:?}"))?;
    }
    let refused = Some(AppError::Refused(ErrorCode::NotSignedIn));
    // AI apps switched off.
    apps_on(&world, false).await?;
    for secret in [&key, &agent_key] {
        assert_eq!(
            mcp_identity(&world.ctx(), identity(secret)).await.err(),
            refused
        );
    }
    apps_on(&world, true).await?;
    // AI itself switched off for the organization.
    crate::ai::change_ai_setup(
        &world.ctx(),
        crate::ai::ChangeAiSetup {
            enabled: Some(false),
            ..super::scenarios_ai::setup(&world)
        },
    )
    .await?;
    for secret in [&key, &agent_key] {
        assert_eq!(
            mcp_identity(&world.ctx(), identity(secret)).await.err(),
            refused
        );
    }
    Ok(())
}
