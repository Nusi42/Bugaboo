//! The port of the offers: what the use cases that say where an issue type or a group of labels is
//! offered need from storage.
//!
//! Like the other ports, reads take `&self` and **writes** are associated functions that run inside
//! `record_change`, so a change and its domain event are committed together (ARCH-EVENTS). The lock
//! that a change of the offers takes is the one of the workflows (`lock_workflows`): a change to
//! where things are offered, like a new workflow, must not happen between the look at a place and
//! the write of an issue there.

use bb_core::id::ProjectId;
use bb_core::offer::{Offer, OfferSubject};

use crate::store_macros::{queries, writes};
use crate::tags::TagStore;
use crate::workflows::WorkflowStore;

/// What the offers use cases need from storage.
pub trait OfferStore: WorkflowStore + TagStore + 'static {
    queries! {
        /// Every row that says where an issue type or a group of labels is offered.
        fn offers() -> Vec<Offer> = offers_in;
    }

    writes! {
        /// Says whether the subject is offered at the place from now on (the place had a row or
        /// not).
        fn put_offer(offer: &Offer) -> ();
        /// Takes the row at exactly this place away, so the place follows its parent again.
        fn remove_offer(subject: OfferSubject, project: Option<ProjectId>) -> ();
    }
}
