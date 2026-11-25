//! The scenarios of the pin use cases (ADR-0007, M1-F18, CFG-05): the preparation, the call, and
//! how many events the call added. `events.rs` names each use case once and runs them; `pins.rs`
//! uses the same preparation for the rules.

use bb_core::id::SavedSearchId;
use bb_core::pin::PinKind;

use super::scenarios::{Added, measured, text, token};
use super::scenarios_searches::adas_search;
use super::world::World;
use crate::auth::AppError;
use crate::pins::{NamedPin, OrderPins, PinRequest, order_pins, pin_to_sidebar, unpin};

/// The request that names a saved search.
pub(crate) fn request(world: &World, search: SavedSearchId) -> PinRequest {
    PinRequest {
        session_token: token(world),
        kind: PinKind::SavedSearch,
        target: search.to_string(),
    }
}

/// Pins a saved search of Ada's as Ada; its ID.
pub(crate) async fn pinned_search(world: &World) -> Result<SavedSearchId, AppError> {
    let made = adas_search(world).await.map_err(AppError::Internal)?;
    pin_to_sidebar(&world.ctx(), request(world, made.id)).await?;
    Ok(made.id)
}

pub(crate) async fn pin_to_sidebar_scenario(world: &World) -> Added {
    let made = adas_search(world).await?;
    Box::pin(measured(
        &world.store,
        pin_to_sidebar(&world.ctx(), request(world, made.id)),
    ))
    .await
}

pub(crate) async fn unpin_scenario(world: &World) -> Added {
    let made = pinned_search(world).await.map_err(text)?;
    Box::pin(measured(
        &world.store,
        unpin(&world.ctx(), request(world, made)),
    ))
    .await
}

pub(crate) async fn order_pins_scenario(world: &World) -> Added {
    let made = pinned_search(world).await.map_err(text)?;
    Box::pin(measured(
        &world.store,
        order_pins(
            &world.ctx(),
            OrderPins {
                session_token: token(world),
                pins: vec![NamedPin {
                    kind: PinKind::SavedSearch,
                    target: made.to_string(),
                }],
            },
        ),
    ))
    .await
}
