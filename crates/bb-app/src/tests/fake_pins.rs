//! The pin part of the in-memory store (CFG-05).

use bb_core::id::UserId;
use bb_core::pin::{Pin, PinTarget};

use super::fake::{FakeError, FakeStore, Tx};
use crate::pins::PinStore;

impl PinStore for FakeStore {
    async fn pins_of_in(tx: &mut Tx, person: UserId) -> Result<Vec<Pin>, FakeError> {
        let mut found: Vec<Pin> = tx
            .state
            .pins
            .iter()
            .filter(|pin| pin.person == person)
            .cloned()
            .collect();
        found.sort_by_key(|pin| (pin.place, pin.target));
        Ok(found)
    }

    async fn lock_pins_of(_tx: &mut Tx, _person: UserId) -> Result<(), FakeError> {
        Ok(())
    }

    async fn insert_pin(
        tx: &mut Tx,
        person: UserId,
        target: PinTarget,
        place: i64,
    ) -> Result<(), FakeError> {
        tx.state.pins.push(Pin {
            person,
            kind: target.kind(),
            target: target.record(),
            place,
        });
        Ok(())
    }

    async fn delete_pin(tx: &mut Tx, person: UserId, target: PinTarget) -> Result<bool, FakeError> {
        let before = tx.state.pins.len();
        tx.state
            .pins
            .retain(|pin| !(pin.person == person && pin.is_to(target)));
        Ok(tx.state.pins.len() < before)
    }

    async fn put_pin_places(
        tx: &mut Tx,
        person: UserId,
        places: &[(PinTarget, i64)],
    ) -> Result<(), FakeError> {
        for (target, place) in places {
            for pin in &mut tx.state.pins {
                if pin.person == person && pin.is_to(*target) {
                    pin.place = *place;
                }
            }
        }
        Ok(())
    }

    async fn delete_pins_to(tx: &mut Tx, targets: &[PinTarget]) -> Result<(), FakeError> {
        tx.state
            .pins
            .retain(|pin| !pin.points_to().is_some_and(|each| targets.contains(&each)));
        Ok(())
    }
}
