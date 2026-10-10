//! The port of the history: the outbox consumer that builds it, and the read of it.

use std::future::Future;

use bb_core::event::DomainEvent;
use bb_core::history::{HistoryEntry, HistoryKind};
use bb_core::id::IssueId;
use jiff::Timestamp;

use crate::issues::IssueStore;
use crate::store_macros::reads;

/// What the history needs from storage.
pub trait HistoryStore: IssueStore + 'static {
    reads! {
        /// The next events the history consumer has not finished, with their places in the outbox,
        /// in the order they were committed, at most `limit`.
        fn history_events(limit: u64) -> Vec<(i64, DomainEvent)>;
        /// Stores the rows made from events, each once (a row that is there already is left as it
        /// is), and records that the consumer has finished everything up to `position`. A
        /// consumer that stops half way reads the same events again and makes the same rows.
        fn store_history(entries: &[HistoryEntry], position: i64, now: Timestamp) -> ();
        /// The rows of one issue, newest first: the ones before `before` (a place in the outbox),
        /// of one kind when `kind` is given, at most `limit`.
        fn history_page(
            issue: IssueId,
            kind: Option<HistoryKind>,
            before: Option<i64>,
            limit: u64
        ) -> Vec<HistoryEntry>;
    }
}
