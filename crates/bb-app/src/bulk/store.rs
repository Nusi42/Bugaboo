//! The port of the runs of changes to many issues (ISS-09): what the use cases need from storage
//! beyond the jobs port.
//!
//! The run itself is bookkeeping of a piece of work that goes on in the background: it is written
//! when the work is queued (in the change that queues it), and its progress is saved as the work
//! goes on, outside a change, because every issue it changes is a change of its own with its own
//! event.

use std::future::Future;

use bb_core::bulk::BulkRun;
use bb_core::id::BulkRunId;

use crate::jobs::JobStore;
use crate::store_macros::{queries, reads, writes};

/// What the use cases of changing many issues need from storage.
pub trait BulkStore: JobStore + 'static {
    queries! {
        /// The run with this ID.
        fn bulk_run(id: BulkRunId) -> Option<BulkRun> = bulk_run_in;
    }

    reads! {
        /// Saves how far a run is (its status, the number of issues looked at, the number changed,
        /// the issues left alone, the end) if it is still at `expected_done`, and says whether it
        /// was. Two workers that took the same step cannot both count it.
        fn save_bulk_progress(run: &BulkRun, expected_done: i64) -> bool;
    }

    writes! {
        /// Adds a run.
        fn insert_bulk_run(run: &BulkRun) -> ();
    }
}
