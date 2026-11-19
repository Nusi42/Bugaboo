//! The port of the removal of a person (PRIV-04, ADR-0044, decision 6): what *Remove a person*
//! needs from storage beyond each module's own deletion function. Both writes find their rows by
//! the walk's conditions (`PersonData`), so the removal never searches rows by person a second way.
//!
//! Like the other ports, **writes** are associated functions that run inside `record_change`, so
//! the removal and its domain event are committed together (ARCH-EVENTS).

use std::future::Future;

use bb_core::event::{EntityRef, RecordKind};
use bb_core::id::UserId;
use bb_core::personal::Link;

use crate::ports::ChangeStore;
use crate::store_macros::writes;

/// What the changes of events, the history and notifications are scrubbed of (M2-F75, M5-F63):
/// the values of a change about one of these records lose their before and after, and so does
/// every member whose value holds one of the words. The names of the members stay, so the
/// history still says what changed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scrub {
    /// The kinds whose records belong to the person, as `bb_core::whose` says: every change
    /// about such a record of the person is scrubbed.
    pub kinds: Vec<RecordKind>,
    /// Records named one by one: the person's own settings, their private searches.
    pub records: Vec<EntityRef>,
    /// Texts no change may keep: the person's name and e-mail address.
    pub words: Vec<String>,
}

/// What the removal of a person needs from storage.
pub trait RemovalStore: ChangeStore + 'static {
    writes! {
        /// Deletes the rows of `kind` that name `person` by a link that is one of `links`, found
        /// by the walk's conditions; how many went. A kind that is not a table deletes nothing.
        fn delete_linked(person: UserId, kind: &str, links: &[Link]) -> u64;
        /// Scrubs the changes of the events, the history and the notifications of what `scrub`
        /// names, about `person`; how many rows changed.
        fn scrub_changes(person: UserId, scrub: &Scrub) -> u64;
    }
}
