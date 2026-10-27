//! The port of the index of the full-text search.

use std::future::Future;

use bb_core::event::DomainEvent;
use jiff::Timestamp;

use super::map::Touched;
use crate::issues::IssueStore;
use crate::store_macros::reads;

/// What the index needs from storage.
pub trait TextIndexStore: IssueStore + 'static {
    reads! {
        /// The next events the index consumer has not finished, with their places in the outbox,
        /// in the order they were committed, at most `limit`.
        fn text_index_events(limit: u64) -> Vec<(i64, DomainEvent)>;
        /// Makes the rows of every issue that `touched` names what the stored texts say now (a
        /// comment or a file is looked up to find its issue), and records that the consumer has
        /// finished everything up to `position`. A consumer that stops half way reads the same
        /// events again and makes the same rows.
        fn store_text_index(touched: &[Touched], position: i64, now: Timestamp) -> ();
        /// Builds the whole index again from the stored texts: takes the head of the outbox, makes
        /// the rows of every issue in pieces (one transaction for each), and puts the place of the
        /// consumer at that head if that is further than it is (it also clears the mark
        /// `bb_core::text_index::REBUILD_WANTED`), so the events after the head are read as always.
        /// The one function the start of the server, `bugaboo search-index rebuild` and the sample
        /// organization use. Returns how many issues it read.
        fn rebuild_text_index(now: Timestamp) -> u64;
        /// [`rebuild_text_index`](Self::rebuild_text_index) when the place of the consumer says the
        /// index has to be made again (the mark), else nothing. `None` when nothing was wanted,
        /// else how many issues it read.
        fn rebuild_text_index_if_wanted(now: Timestamp) -> Option<u64>;
    }
}
