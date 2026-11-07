//! How a row of the history belongs to a run (AI-38): made in one, or the undo of one, and the run
//! to link to for a reader who may open it.

use bb_core::history::HistoryEntry;
use bb_core::id::RunId;

use super::read::Reading;
use super::types::RunMarkKind;
use crate::auth::{AppError, AuthStore, Ctx};

impl<S: AuthStore> Ctx<'_, S> {
    /// The mark of a row and the run it links to: a run is linked to only for a reader who may open
    /// it, so the others see the mark alone.
    pub(super) async fn run_mark_of(
        &self,
        reading: &mut Reading<'_>,
        row: &HistoryEntry,
    ) -> Result<(Option<RunMarkKind>, Option<RunId>), AppError> {
        let mark = match (row.undid_run, row.run) {
            (Some(_), _) => Some(RunMarkKind::Undid),
            (None, Some(_)) => Some(RunMarkKind::Made),
            (None, None) => None,
        };
        let run = match row.undid_run.or(row.run) {
            Some(id) => self.run_may_open(reading, id).await?.then_some(id),
            None => None,
        };
        Ok((mark, run))
    }

    /// Whether the reader may open the run (it is their own), read once for each run.
    async fn run_may_open(&self, reading: &mut Reading<'_>, run: RunId) -> Result<bool, AppError> {
        if let Some(known) = reading.runs.get(&run) {
            return Ok(*known);
        }
        let may = match self.own_run(reading.actor, run).await {
            Ok(_) => true,
            Err(AppError::Refused(_)) => false,
            Err(error) => return Err(error),
        };
        reading.runs.insert(run, may);
        Ok(may)
    }
}
