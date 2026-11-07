//! What the use cases of runs need from storage beyond the runs themselves (which are the rows of
//! `ManyChangeStore`): the events a run made, the events that changed the same records behind
//! its back, and the marks of texts written with AI.

use std::future::Future;

use bb_core::event::{DomainEvent, EntityKind};
use bb_core::id::{EntityId, EventId, RunId};
use bb_core::mark::TextMark;

use crate::many_changes::ManyChangeStore;
use crate::store_macros::reads;

/// The events of runs, read for the list of a run's changes and for *Undo the run*.
pub trait RunRecordStore: ManyChangeStore + 'static {
    reads! {
        /// The events made inside the run, with their places in the order they were committed, after `after` (none
        /// for the start), at most `limit`.
        fn run_events(run: RunId, after: Option<EventId>, limit: u64) -> Vec<(i64, DomainEvent)>;
        /// The events about these records of one kind that were committed after `since` and were
        /// not made inside `run`, with their places in the order they were committed.
        fn events_outside_run_after(
            run: RunId,
            since: EventId,
            kind: &EntityKind,
            ids: &[EntityId]
        ) -> Vec<(i64, DomainEvent)>;
        /// The marks of the texts of these records of one kind (COMP-08, COMP-09); a text without
        /// a mark has none.
        fn text_marks(kind: &EntityKind, ids: &[EntityId]) -> Vec<TextMark>;
    }
}
