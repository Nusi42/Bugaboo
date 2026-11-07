//! The port of the history: the outbox consumer that builds it, and the read of it.

use std::future::Future;

use bb_core::event::DomainEvent;
use bb_core::history::{HistoryEntry, HistoryKind};
use bb_core::id::{EventId, IssueId, UserId};
use jiff::Timestamp;

use crate::hashed_key::HashedKey;
use crate::issues::IssueStore;
use crate::store_macros::reads;

/// Where a row stands in the order for a reader who may not see the moment of a time entry: the
/// moment the row is shown at (microseconds) and a hash of its event that tells nothing. It is what
/// a cursor of such a page holds, and nothing else: no place in the outbox.
pub type ShownKey = HashedKey<i64>;

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
        /// The name of the AI app behind each of these events, for those that were made in a
        /// session of an app: the event names its run, and the run the app. Events of nobody's app
        /// are left out.
        fn apps_of_events(events: &[EventId]) -> Vec<(EventId, String)>;
        /// The rows of one issue, newest first: the ones before `before` (a place in the outbox),
        /// of one kind when `kind` is given, leaving out the first `skip`, at most `limit`.
        fn history_page(
            issue: IssueId,
            kind: Option<HistoryKind>,
            before: Option<i64>,
            skip: u64,
            limit: u64
        ) -> Vec<HistoryEntry>;
        /// The rows of one issue for a reader who may not see the moment of a time entry of
        /// another person, the latest shown moment first and, among rows of one moment, by the hash
        /// of the event: a row made by `reader` is shown at its own moment and every other row of
        /// time at the start of its date. The rows after `after`, leaving out the first `skip`, at
        /// most `limit`, each with its place.
        fn history_page_shown(
            issue: IssueId,
            kind: Option<HistoryKind>,
            reader: UserId,
            after: Option<ShownKey>,
            skip: u64,
            limit: u64
        ) -> Vec<(HistoryEntry, ShownKey)>;
    }
}
