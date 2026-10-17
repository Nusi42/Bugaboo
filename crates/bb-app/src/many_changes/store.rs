//! The port of the runs of changes to many issues (ISS-09): what the use cases need from storage
//! beyond the jobs port.
//!
//! The run itself is bookkeeping of a piece of work that goes on in the background: it is written
//! when the work is queued (in the change that queues it), and its progress is saved as the work
//! goes on, outside a change, because every issue it changes is a change of its own with its own
//! event.

use std::future::Future;

use bb_core::id::ManyChangeRunId;
use bb_core::many_changes::{ManyChangeRun, ManyChangeStatus, RunsToEnd};
use jiff::Timestamp;

use crate::jobs::JobStore;
use crate::store_macros::{queries, reads, writes};

/// What the use cases of changing many issues need from storage.
pub trait ManyChangeStore: JobStore + 'static {
    queries! {
        /// The run with this ID.
        fn many_changes_run(id: ManyChangeRunId) -> Option<ManyChangeRun> = many_changes_run_in;
    }

    reads! {
        /// Saves how far a run is (its status, the number of issues looked at, the number changed,
        /// the issues left alone, the end) if it is still at `expected_done`, and says whether it
        /// was. A run that is over (done or stopped) is never changed again, so a stop
        /// is not undone by a step that was already going. Two workers that took the same step cannot both count it.
        fn save_many_changes_progress(run: &ManyChangeRun, expected_done: i64) -> bool;
        /// Whether the run is over (done, failed or stopped): a step looks before each issue, so a run
        /// that was stopped changes nothing more.
        fn many_changes_run_is_over(id: ManyChangeRunId) -> bool;
        /// Whether an event with this reason was written at or after `since`: how a run that went
        /// on after a worker stopped finds out that the worker changed an issue before it stopped.
        fn event_with_reason(reason: &str, since: Timestamp) -> bool;
    }

    writes! {
        /// Adds a run.
        fn insert_many_changes_run(run: &ManyChangeRun) -> ();
        /// Ends the runs named by `which` that are not over yet, as `status` (failed or stopped), at
        /// `at`, and says how many ended. The one way a run is ended: a step that cannot go on, a
        /// person who stops a run, and a person who signed out everywhere or changed their password.
        fn end_many_changes_runs(which: &RunsToEnd, status: ManyChangeStatus, at: Timestamp) -> u64;
    }
}
