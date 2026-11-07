//! The scenarios of `events.rs` for the commands of the sign-in of AI apps: each appends its event.

use bb_core::access_key::KeyChoice;

use super::oauth::{LOOPBACK, VERIFIER, ai_on, allowed, registered, request, signed_in, swap_code};
use super::scenarios::{Added, measured, text, token};
use super::world::World;
use crate::auth::{
    AddWebAddress, ConnectedAppsQuery, DecideAuthorization, RegisterApp, TakeBack, add_web_address,
    connected_apps, decide_authorization, exchange_token, register_app, take_back,
};

pub(crate) async fn register_app_scenario(world: &World) -> Added {
    ai_on(world).await?;
    measured(
        &world.store,
        register_app(
            &world.ctx(),
            RegisterApp {
                name: "Claude Code".to_owned(),
                return_addresses: vec![LOOPBACK.to_owned()],
            },
        ),
    )
    .await
}

pub(crate) async fn decide_authorization_scenario(world: &World) -> Added {
    ai_on(world).await?;
    let client = registered(world, LOOPBACK).await.map_err(text)?;
    measured(
        &world.store,
        decide_authorization(
            &world.ctx(),
            DecideAuthorization {
                session_token: token(world),
                request: request(client, "see-only"),
                allow: true,
                scope: KeyChoice::SeeOnly,
                projects: Vec::new(),
            },
        ),
    )
    .await
}

/// A swap of a code writes tokens and no event: a renewal is not logged one by one (ADR-0042).
pub(crate) async fn exchange_token_scenario(world: &World) -> Added {
    ai_on(world).await?;
    let client = registered(world, LOOPBACK).await.map_err(text)?;
    let code = allowed(world, client, "see-only", KeyChoice::SeeOnly)
        .await
        .map_err(text)?;
    measured(
        &world.store,
        exchange_token(&world.ctx(), swap_code(client, &code, VERIFIER)),
    )
    .await
}

pub(crate) async fn take_back_scenario(world: &World) -> Added {
    ai_on(world).await?;
    let (_, _) = signed_in(world, KeyChoice::SeeOnly).await.map_err(text)?;
    let listed = connected_apps(
        &world.ctx(),
        ConnectedAppsQuery {
            session_token: token(world),
        },
    )
    .await
    .map_err(text)?
    .into_value();
    let id = listed.items.first().ok_or("no app")?.id;
    measured(
        &world.store,
        take_back(
            &world.ctx(),
            TakeBack {
                session_token: token(world),
                id,
            },
        ),
    )
    .await
}

pub(crate) async fn add_web_address_scenario(world: &World) -> Added {
    measured(
        &world.store,
        add_web_address(
            &world.ctx(),
            AddWebAddress {
                session_token: token(world),
                address: "https://app.example.com/callback".to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn remove_web_address_scenario(world: &World) -> Added {
    add_web_address_scenario(world).await?;
    measured(
        &world.store,
        crate::auth::remove_web_address(
            &world.ctx(),
            crate::auth::RemoveWebAddress {
                session_token: token(world),
                address: "https://app.example.com/callback".to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn clean_old_grants_scenario(world: &World) -> Added {
    measured(&world.store, crate::auth::clean_old_grants(&world.ctx())).await
}
