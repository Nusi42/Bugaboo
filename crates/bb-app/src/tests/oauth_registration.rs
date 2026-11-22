//! Open registration of AI apps (RFC 7591, ADR-0042) cannot fill the server: apps nobody allowed
//! go after a day, the whole server keeps only so many from an hour (a new one takes the place of
//! the first that nobody allowed), and the security log holds one entry an hour instead of one for
//! each app.

use bb_core::access_key::KeyChoice;
use bb_core::id::OAuthClientId;
use bb_core::network::NetworkPrefix;
use bb_core::oauth::{REGISTRATIONS_AN_HOUR, REGISTRATIONS_AN_HOUR_FOR_THE_SERVER};
use bb_core::sign_in::SecurityAction;

use super::oauth::{LOOPBACK, ai_on, allowed, registered, signed_in};
use super::world::{Outcome, World};
use crate::auth::{
    AppError, RegisterApp, RegisterOutcome, RequestInfo, clean_old_grants, register_app,
};

/// Registers an app from the network of `address`.
async fn register_from(world: &World, address: &str) -> Result<RegisterOutcome, AppError> {
    let info = RequestInfo {
        network: Some(NetworkPrefix::from_address(
            address
                .parse()
                .map_err(|_| AppError::Internal("an address".into()))?,
        )),
        ..RequestInfo::default()
    };
    register_app(
        &world.services.ctx(info),
        RegisterApp {
            name: "Claude Code".to_owned(),
            return_addresses: vec![LOOPBACK.to_owned()],
        },
    )
    .await
}

#[tokio::test]
async fn ai_22_an_app_nobody_allowed_is_deleted_after_a_day() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    ai_on(&world).await?;
    registered(&world, LOOPBACK).await?;
    signed_in(&world, KeyChoice::SeeOnly).await?;
    world.clock.advance(25 * 3600);
    let cleaned = clean_old_grants(&world.ctx()).await?;
    // The app nobody allowed goes; the one with a grant stays.
    assert_eq!(cleaned.clients, 1);
    Ok(())
}

/// Registers `count` apps from the network of `address` and gives the IDs back.
async fn flood_from(
    world: &World,
    address: &str,
    count: u32,
) -> Result<Vec<OAuthClientId>, AppError> {
    let mut made = Vec::new();
    for _ in 0..count {
        world.clock.advance(1);
        match register_from(world, address).await? {
            RegisterOutcome::Registered(app) => made.push(app.client_id),
            RegisterOutcome::Refused(problem) => {
                return Err(AppError::Internal(problem.code().to_owned()));
            }
        }
    }
    Ok(made)
}

#[tokio::test]
async fn sec_14_a_flood_of_registrations_cannot_keep_a_real_app_out() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    ai_on(&world).await?;
    // Networks that each register their share fill the hour of the server with apps nobody allows.
    let networks = REGISTRATIONS_AN_HOUR_FOR_THE_SERVER / REGISTRATIONS_AN_HOUR;
    let mut flood = Vec::new();
    for network in 0..networks {
        let address = format!("198.51.{network}.7");
        flood.extend(flood_from(&world, &address, REGISTRATIONS_AN_HOUR).await?);
    }
    // The real app still registers: it takes the place of the first app of the hour nobody allowed.
    let real = registered(&world, LOOPBACK).await?;
    let clients = |world: &World| -> Vec<OAuthClientId> {
        let store = world.store.lock();
        store
            .state
            .oauth
            .clients
            .iter()
            .map(|client| client.id)
            .collect()
    };
    assert!(!clients(&world).contains(&flood[0]));
    assert_eq!(
        clients(&world).len(),
        usize::try_from(REGISTRATIONS_AN_HOUR_FOR_THE_SERVER)?
    );
    // Its person allows it, and a second flood pushes out only apps that nobody allowed.
    allowed(&world, real, "see-and-change", KeyChoice::SeeOnly).await?;
    for network in networks..2 * networks {
        flood_from(
            &world,
            &format!("198.51.{network}.7"),
            REGISTRATIONS_AN_HOUR,
        )
        .await?;
    }
    assert!(clients(&world).contains(&real));
    // A network that used its share still waits, whatever room the server makes.
    let refused = register_from(&world, "198.51.0.7").await;
    assert!(
        matches!(refused, Err(AppError::Limited { .. })),
        "{refused:?}"
    );
    // One entry for the first app of the hour and one for the full hour, not one for each app.
    let log = world.store.lock().log.clone();
    let count = |action: SecurityAction| log.iter().filter(|entry| entry.action == action).count();
    assert_eq!(count(SecurityAction::OAuthClientRegistered), 1);
    assert_eq!(count(SecurityAction::OAuthRegistrationsCapped), 1);
    Ok(())
}
