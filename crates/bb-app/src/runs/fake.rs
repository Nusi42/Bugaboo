//! The runs of the tests: a store in memory that keeps the runs opened and closed and the changes a
//! test adds to a run, and undoes a run by taking those changes back, so the packages that use
//! `RunStore` test against it before the real store is wired.

use std::collections::BTreeMap;
use std::convert::Infallible;
use std::sync::{Mutex, MutexGuard, PoisonError};

use bb_core::id::{EventId, RunId};
use bb_core::many_changes::ManyChangeStatus;
use bb_core::run::{
    RunActor, RunChange, RunEnd, RunKind, RunSummary, UndoOutcome, UndoPreview, UndoResult,
    UndoSkip, UndoVersion,
};
use jiff::Timestamp;

use super::RunStore;

#[derive(Default)]
struct State {
    runs: BTreeMap<RunId, RunSummary>,
    changes: BTreeMap<RunId, Vec<RunChange>>,
    skips: BTreeMap<RunId, Vec<UndoSkip>>,
    undone: Vec<RunId>,
}

/// The fake store of runs.
#[derive(Default)]
pub struct FakeRunStore(Mutex<State>);

impl FakeRunStore {
    fn state(&self) -> MutexGuard<'_, State> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Adds a change to a run, as if an event had carried it.
    pub fn add_change(&self, run: RunId, change: RunChange) {
        self.state().changes.entry(run).or_default().push(change);
    }

    /// Says that a field of the run stays when the run is undone.
    pub fn add_skip(&self, run: RunId, skip: UndoSkip) {
        self.state().skips.entry(run).or_default().push(skip);
    }

    /// The runs that were undone, in order.
    #[must_use]
    pub fn undone_runs(&self) -> Vec<RunId> {
        self.state().undone.clone()
    }

    fn preview_of(state: &State, run: RunId) -> UndoPreview {
        let fields: u64 = state.changes.get(&run).map_or(0, |changes| {
            changes
                .iter()
                .map(|change| change.changes.len() as u64)
                .sum()
        });
        let skips = state.skips.get(&run).cloned().unwrap_or_default();
        let skipped = skips.len() as u64;
        UndoPreview {
            version: UndoVersion(format!("{fields}:{skipped}")),
            goes_back: fields.saturating_sub(skipped),
            skips,
            skipped,
        }
    }
}

impl RunStore for FakeRunStore {
    type Error = Infallible;

    async fn open_run(
        &self,
        run: RunId,
        kind: RunKind,
        actor: &RunActor,
        at: Timestamp,
    ) -> Result<(), Infallible> {
        let summary = RunSummary {
            id: run,
            kind,
            actor: actor.clone(),
            status: ManyChangeStatus::Running,
            started_at: at,
            ended_at: None,
        };
        self.state().runs.insert(run, summary);
        Ok(())
    }

    async fn close_run(
        &self,
        run: RunId,
        ended: RunEnd,
        at: Timestamp,
    ) -> Result<bool, Infallible> {
        let mut state = self.state();
        Ok(state.runs.get_mut(&run).is_some_and(|summary| {
            let open = !summary.status.is_over();
            if open {
                summary.status = ended.status();
                summary.ended_at = Some(at);
            }
            open
        }))
    }

    async fn run(&self, run: RunId) -> Result<Option<RunSummary>, Infallible> {
        Ok(self.state().runs.get(&run).cloned())
    }

    async fn run_changes(
        &self,
        run: RunId,
        after: Option<EventId>,
        limit: u64,
    ) -> Result<Vec<RunChange>, Infallible> {
        let state = self.state();
        let all = state.changes.get(&run).map_or(&[][..], Vec::as_slice);
        let start = after.map_or(0, |after| {
            all.iter()
                .position(|change| change.event == after)
                .map_or(all.len(), |at| at + 1)
        });
        Ok(all
            .iter()
            .skip(start)
            .take(usize::try_from(limit).unwrap_or(usize::MAX))
            .cloned()
            .collect())
    }

    async fn preview_undo(&self, run: RunId) -> Result<UndoPreview, Infallible> {
        Ok(Self::preview_of(&self.state(), run))
    }

    async fn undo_run(
        &self,
        run: RunId,
        version: &UndoVersion,
        _by: &RunActor,
        _at: Timestamp,
    ) -> Result<UndoOutcome, Infallible> {
        let mut state = self.state();
        let now = Self::preview_of(&state, run);
        if &now.version != version {
            return Ok(UndoOutcome::Changed(now));
        }
        state.undone.push(run);
        Ok(UndoOutcome::Undone(UndoResult {
            undone: now.goes_back,
            skips: now.skips,
            skipped: now.skipped,
        }))
    }
}

#[cfg(test)]
mod tests {
    use bb_core::id::{AgentId, SequentialIds, UserId};

    use super::*;

    fn actor(ids: &mut SequentialIds) -> RunActor {
        RunActor {
            person: UserId::generate(ids),
            app: None,
            agent: Some(AgentId::generate(ids)),
        }
    }

    #[tokio::test]
    async fn a_run_closes_once_and_an_undo_needs_the_current_version() {
        let mut ids = SequentialIds::default();
        let store = FakeRunStore::default();
        let (run, who, at) = (
            RunId::generate(&mut ids).unwrap(),
            actor(&mut ids),
            Timestamp::UNIX_EPOCH,
        );
        store
            .open_run(run, RunKind::Attempt, &who, at)
            .await
            .unwrap();
        let first = store.preview_undo(run).await.unwrap();
        store.add_skip(
            run,
            UndoSkip {
                entity: bb_core::event::EntityRef {
                    kind: bb_core::event::RecordKind::Comment.into(),
                    id: UserId::generate(&mut ids).into(),
                },
                field: "text".to_owned(),
                kind: bb_core::run::UndoSkipKind::Gone,
                by: None,
                at: None,
            },
        );
        let stale = store.undo_run(run, &first.version, &who, at).await.unwrap();
        assert!(matches!(stale, UndoOutcome::Changed(now) if now.skipped == 1));
        assert!(store.undone_runs().is_empty());
        assert!(store.close_run(run, RunEnd::Done, at).await.unwrap());
        assert!(!store.close_run(run, RunEnd::Failed, at).await.unwrap());
        let status = store.run(run).await.unwrap().unwrap().status;
        assert_eq!(status, ManyChangeStatus::Done);
    }
}
