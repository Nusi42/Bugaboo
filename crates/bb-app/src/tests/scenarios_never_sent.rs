//! The scenario of the use case of *Never sent to AI* for the table of `events.rs`.

use bb_core::fields::Field;

use super::scenarios::{Added, measured, token};
use super::world::World;
use crate::ai::{NeverSentTarget, SetNeverSent, set_never_sent};

pub(crate) async fn set_never_sent_scenario(world: &World) -> Added {
    measured(
        &world.store,
        set_never_sent(
            &world.ctx(),
            SetNeverSent {
                session_token: token(world),
                target: NeverSentTarget::Field(Field::Description),
                marked: true,
                version: 0,
            },
        ),
    )
    .await
}
