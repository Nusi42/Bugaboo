//! The port of the pins: what the use cases need from storage.

use std::future::Future;

use bb_core::id::UserId;
use bb_core::pin::{Pin, PinTarget};

use crate::ports::ChangeStore;
use crate::store_macros::{queries, writes};

/// What the pin use cases need from storage.
pub trait PinStore: ChangeStore + 'static {
    queries! {
        /// The pins of a person, in their order.
        fn pins_of(person: UserId) -> Vec<Pin> = pins_of_in;
    }

    writes! {
        /// Takes the lock of one person's pins, so that two changes of theirs at the same moment
        /// cannot both pass the limit; does nothing where writes cannot overlap.
        fn lock_pins_of(person: UserId) -> ();
        /// Adds a pin at `place`; the use case decides that it is not pinned yet and where it goes.
        fn insert_pin(person: UserId, target: PinTarget, place: i64) -> ();
        /// Takes a pin away; `false` when the person had not pinned it.
        fn delete_pin(person: UserId, target: PinTarget) -> bool;
        /// Gives each pin the place next to it.
        fn put_pin_places(person: UserId, places: &[(PinTarget, i64)]) -> ();
        /// Deletes the pins of everybody that point to these records (the records are deleted).
        fn delete_pins_to(targets: &[PinTarget]) -> ();
    }
}
