//! The job `privacy.finish-removal` (PRIV-04, ADR-0044, decision 6, M5-F52): what a removal cannot
//! do inside its one change. It is queued in that change, with the comments it deleted, the issues
//! they were on and the exports of the person it deleted, and it works in steps, the way the
//! deleting jobs do (`Ctx::keep_step`):
//!
//! 1. the files of the deleted exports leave the folder of the exports (a file that is gone already
//!    is no error);
//! 2. the full-text index forgets the deleted texts: the rows of the deleted comments go and the
//!    rows of their issues are made again from what is stored now, at most
//!    [`FINISH_REMOVAL_ISSUES`] issues in a step, and a step that leaves issues queues the next;
//! 3. after the last step the database lets go of the old text: on SQLite the index merges its
//!    segments and the write-ahead log is folded back and emptied (`compact_removed_texts`).
//!
//! Every step is safe to run again: deleting a file that is gone, reindexing an issue and merging
//! the segments change nothing the second time.

use bb_core::id::{CommentId, DataExportId, IssueId, JobId};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::jobs::JobKind;

/// The most issues one step of the job reindexes: few enough that a step is a short write.
pub const FINISH_REMOVAL_ISSUES: usize = 500;

/// What is left to do of a removal: the payload of its job.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinishRemoval {
    /// The comments the removal deleted, whose rows of the index go.
    #[serde(default)]
    pub comments: Vec<CommentId>,
    /// The issues whose texts changed, whose rows of the index are made again.
    #[serde(default)]
    pub issues: Vec<IssueId>,
    /// The exports of the person the removal deleted, whose files go.
    #[serde(default)]
    pub exports: Vec<DataExportId>,
}

impl FinishRemoval {
    /// The payload of the job.
    ///
    /// # Errors
    /// An internal error when the IDs cannot be written as JSON (they always can).
    pub fn payload(&self) -> Result<Value, AppError> {
        serde_json::to_value(self).map_err(|error| AppError::Internal(error.to_string()))
    }

    /// The step this run does, and what is left for the next one: the files and the comments go
    /// in the first step, the issues a share at a time.
    fn split(mut self) -> (Self, Option<Self>) {
        if self.issues.len() <= FINISH_REMOVAL_ISSUES {
            return (self, None);
        }
        let rest = self.issues.split_off(FINISH_REMOVAL_ISSUES);
        (
            self,
            Some(Self {
                issues: rest,
                ..Self::default()
            }),
        )
    }
}

/// Puts the job of a removal in the queue inside the removal's change.
///
/// # Errors
/// The error of the store.
pub(crate) async fn queue_finish_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    job: JobId,
    left: &FinishRemoval,
    now: Timestamp,
) -> Result<(), S::Error> {
    let kind = JobKind::FinishRemoval.name().map_err(|_| S::aborted())?;
    let payload = left.payload().map_err(|_| S::aborted())?;
    S::enqueue_job(transaction, job, &kind, &payload, now, now).await
}

impl<S: AuthStore> Ctx<'_, S> {
    /// One step of the job `privacy.finish-removal` (see the module). A payload that names nothing
    /// still lets go of the old text: the scrub of the changes freed pages too.
    ///
    /// # Errors
    /// `request.invalid` for a payload that is not one of this job; otherwise the error of the
    /// store or of the files.
    pub(crate) async fn run_finish_removal(&self, payload: &Value) -> Result<(), AppError> {
        let left: FinishRemoval = serde_json::from_value(payload.clone())
            .map_err(|_| AppError::Refused(bb_core::ErrorCode::InvalidRequest))?;
        let (step, rest) = left.split();
        for export in &step.exports {
            self.exports()
                .delete(*export)
                .await
                .map_err(|error| AppError::Internal(error.to_string()))?;
        }
        let failed = |error| internal::<S>(&error);
        self.store()
            .reindex_removed_texts(&step.issues, &step.comments)
            .await
            .map_err(failed)?;
        if let Some(rest) = rest {
            let id = JobId::generate(&mut self.ids())
                .map_err(|error| AppError::Internal(error.to_string()))?;
            self.store()
                .queue_job(
                    id,
                    &JobKind::FinishRemoval.name()?,
                    &rest.payload()?,
                    self.now(),
                    self.now(),
                )
                .await
                .map_err(failed)?;
        } else {
            self.store().compact_removed_texts().await.map_err(failed)?;
            tracing::info!("a removal of a person is finished");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use bb_core::id::SequentialIds;

    use super::*;

    #[test]
    fn a_large_removal_is_split_into_steps_of_issues() {
        let mut ids = SequentialIds::default();
        let issues: Vec<IssueId> = (0..=FINISH_REMOVAL_ISSUES)
            .map(|_| IssueId::generate(&mut ids).unwrap_or_else(|_| unreachable!()))
            .collect();
        let export = DataExportId::generate(&mut ids).unwrap_or_else(|_| unreachable!());
        let (step, rest) = FinishRemoval {
            comments: Vec::new(),
            issues: issues.clone(),
            exports: vec![export],
        }
        .split();
        assert_eq!(step.issues.len(), FINISH_REMOVAL_ISSUES);
        assert_eq!(step.exports, vec![export]);
        let rest = rest.unwrap_or_default();
        assert_eq!(rest.issues, issues[FINISH_REMOVAL_ISSUES..].to_vec());
        assert!(rest.exports.is_empty(), "the files go in the first step");
        assert!(rest.split().1.is_none());
    }
}
