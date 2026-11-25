//! The scenarios of the use cases of e-mailed searches (ADR-0007, M1-F18, QRY-07): the
//! preparation, the call, and how many events the call added. `events.rs` names each use case once
//! and runs them.

use super::scenarios::{Added, measured, text, token};
use super::scenarios_searches::adas_search;
use super::world::World;
use crate::search_mail::{SetSearchMail, StopSearchMail, set_search_mail, stop_search_mail};

/// Every Monday at 8:00, only when something is new or changed.
fn mondays(world: &World, search: bb_core::id::SavedSearchId) -> SetSearchMail {
    SetSearchMail {
        session_token: token(world),
        search,
        days: [true, false, false, false, false, false, false],
        hour: 8,
        only_when_changed: true,
    }
}

pub(crate) async fn set_search_mail_scenario(world: &World) -> Added {
    let made = adas_search(world).await?;
    Box::pin(measured(
        &world.store,
        set_search_mail(&world.ctx(), mondays(world, made.id)),
    ))
    .await
}

pub(crate) async fn stop_search_mail_scenario(world: &World) -> Added {
    let made = adas_search(world).await?;
    set_search_mail(&world.ctx(), mondays(world, made.id))
        .await
        .map_err(text)?;
    Box::pin(measured(
        &world.store,
        stop_search_mail(
            &world.ctx(),
            StopSearchMail {
                session_token: token(world),
                search: made.id,
            },
        ),
    ))
    .await
}
