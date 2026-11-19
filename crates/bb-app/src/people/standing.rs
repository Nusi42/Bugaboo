//! The port of the standing of a person (PRIV-04, ADR-0044, decisions 5 and 6): the record of a
//! person who is switched off or removed, with the tombstone of a removed person and its
//! legal-hold field. An active person has no record.
//!
//! Like the other ports, reads take `&self` and **writes** are associated functions that run inside
//! `record_change`, so a change and its domain event are committed together (ARCH-EVENTS).

use std::future::Future;

use bb_core::id::UserId;
use bb_core::person::PersonStanding;

use crate::ports::ChangeStore;
use crate::store_macros::{queries, reads, writes};

/// What the use cases of *Switch off* and *Remove a person* need from storage.
pub trait StandingStore: ChangeStore + 'static {
    queries! {
        /// The standing of `person`; none while they are active.
        fn standing(person: UserId) -> Option<PersonStanding> = standing_in;
    }

    reads! {
        /// The labels of every removed person, for drawing a label nobody has
        /// (`bb_policy::former_user_label`).
        fn former_labels() -> Vec<String>;
    }

    writes! {
        /// Whether `person` is somebody: a replay into a backup older than the account finds
        /// nobody.
        fn person_exists(person: UserId) -> bool;
        /// Writes the standing of its person, replacing the one they had.
        fn set_standing(standing: &PersonStanding) -> ();
        /// Deletes the standing of a switched-off person (*Switch on*); says whether there was one.
        fn clear_standing(person: UserId) -> bool;
        /// Clears the legal-hold field of a removed person: their last kept work days are deleted
        /// (ADR-0044, decision 6). Says whether there was a name to clear.
        fn clear_legal_hold(person: UserId) -> bool;
    }
}
