//! Pins (CFG-05, ADR-0050, decision 12): the rules, run against the in-memory store; the same rules
//! run on both databases through the API in `bb-server`.

use bb_core::ErrorCode;
use bb_core::pin::PINS_MAX;

use super::scenarios::{text, token};
use super::scenarios_pins::{pinned_search, request};
use super::scenarios_searches::adas_search;
use super::world::World;
use crate::auth::AppError;
use crate::pins::{PinsQuery, my_pins, pin_to_sidebar, unpin};

#[tokio::test]
async fn cfg_05_a_pin_is_listed_in_order_by_its_name_and_only_once() -> Result<(), String> {
    let world = World::new().await?;
    let first = pinned_search(&world).await.map_err(text)?;
    let second = pinned_search(&world).await.map_err(text)?;
    // Pinning what is pinned writes no second event.
    let before = world.store.event_count();
    pin_to_sidebar(&world.ctx(), request(&world, first))
        .await
        .map_err(text)?;
    assert_eq!(world.store.event_count(), before);
    let shown = my_pins(
        &world.ctx(),
        PinsQuery {
            session_token: token(&world),
        },
    )
    .await
    .map_err(text)?;
    let ids: Vec<String> = shown
        .into_value()
        .items
        .iter()
        .map(|pin| pin.id.to_string())
        .collect();
    assert_eq!(ids, [first.to_string(), second.to_string()]);
    // Taking away what is not pinned writes none either.
    unpin(&world.ctx(), request(&world, first))
        .await
        .map_err(text)?;
    let before = world.store.event_count();
    unpin(&world.ctx(), request(&world, first))
        .await
        .map_err(text)?;
    assert_eq!(world.store.event_count(), before);
    Ok(())
}

#[tokio::test]
async fn cfg_05_at_most_30_pins_counted_in_the_change() -> Result<(), String> {
    let world = World::new().await?;
    for _ in 0..PINS_MAX {
        pinned_search(&world).await.map_err(text)?;
    }
    let over = adas_search(&world).await?;
    let refused = pin_to_sidebar(&world.ctx(), request(&world, over.id)).await;
    assert!(
        matches!(refused, Err(AppError::Refused(ErrorCode::LimitReached))),
        "{refused:?}"
    );
    Ok(())
}
