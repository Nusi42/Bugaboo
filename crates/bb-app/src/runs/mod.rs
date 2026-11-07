//! Runs (AI-38, ADR-0029 decision 7): the port `RunStore` that the use cases of MCP and of agent
//! work open and close runs with and undo them through, and the test fake of it. The shape is
//! written in architecture §4 ("The port of runs").

use std::future::Future;

use bb_core::id::{EventId, RunId};
use bb_core::run::{
    RunActor, RunChange, RunEnd, RunKind, RunSummary, UndoOutcome, UndoPreview, UndoVersion,
};
use jiff::Timestamp;

mod marks;
mod records;
mod restore;
mod service;
mod use_cases;
mod weigh;

#[cfg(any(test, feature = "testing"))]
mod fake;

pub(crate) use marks::Marks;
pub use marks::WrittenByView;
pub use records::RunRecordStore;
pub use service::RunsOf;
pub use use_cases::{
    RunChangeView, RunChangesPage, RunChangesQuery, RunQuery, RunView, UndoRun, UndoSkipView,
    UndoView, get_run, preview_undo_run, run_changes, undo_run,
};

#[cfg(any(test, feature = "testing"))]
pub use fake::FakeRunStore;

/// What the use cases need from storage to keep runs and undo them. Every event written while a
/// run is open carries its ID.
///
/// Who may undo is not decided here: the use case judges it, and each change is judged again as the
/// person who undoes it. This port holds the one evaluation of what goes back and what is skipped,
/// which the preview and the undo share.
pub trait RunStore: Send + Sync {
    /// What a failed read or write reports.
    type Error: Send;

    /// Opens the run `run` (an ID the caller made, as the store makes none) of `kind`
    /// (`AppSession` or `Attempt`; a change to many issues keeps its own start) for `actor` at
    /// `at`.
    ///
    /// # Errors
    /// The error of the database.
    fn open_run(
        &self,
        run: RunId,
        kind: RunKind,
        actor: &RunActor,
        at: Timestamp,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Closes the run as `ended`. A run that is closed already stays as it is, so closing twice is
    /// no error; the answer says whether this call closed it.
    ///
    /// # Errors
    /// The error of the database.
    fn close_run(
        &self,
        run: RunId,
        ended: RunEnd,
        at: Timestamp,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send;

    /// The run with this ID.
    ///
    /// # Errors
    /// The error of the database.
    fn run(
        &self,
        run: RunId,
    ) -> impl Future<Output = Result<Option<RunSummary>, Self::Error>> + Send;

    /// The changes of the run, oldest first, after `after` (the ID of the last event of the page
    /// before; a cursor holds nothing else), at most `limit`. All of them: the use case leaves out
    /// what the reader may not see.
    ///
    /// # Errors
    /// The error of the database.
    fn run_changes(
        &self,
        run: RunId,
        after: Option<EventId>,
        limit: u64,
    ) -> impl Future<Output = Result<Vec<RunChange>, Self::Error>> + Send;

    /// What *Undo the run* would do now: each field goes back, or stays with its reason (somebody
    /// changed it afterwards, or the record is gone), with the version of this preview.
    ///
    /// # Errors
    /// The error of the database.
    fn preview_undo(
        &self,
        run: RunId,
    ) -> impl Future<Output = Result<UndoPreview, Self::Error>> + Send;

    /// Undoes the run as the preview with `version` said. When anything changed since, nothing is
    /// written and the newer preview comes back. It writes new events that put the old values back,
    /// marked as an undo of the run, and never deletes history; `by` is the person who undoes.
    ///
    /// # Errors
    /// The error of the database.
    fn undo_run(
        &self,
        run: RunId,
        version: &UndoVersion,
        by: &RunActor,
        at: Timestamp,
    ) -> impl Future<Output = Result<UndoOutcome, Self::Error>> + Send;
}
