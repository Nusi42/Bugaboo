//! The plan part of the in-memory store (PLAN-13, PLAN-14): the time factors of issues and the
//! accepted plan warnings.

use bb_core::id::IssueId;
use bb_core::plan::record::{AcceptedWarning, IssueTimeFactor};
use bb_core::plan::warning::WarningKind;

use super::fake::{FakeError, FakeStore, Tx};
use crate::plan::TimelineStore;

impl TimelineStore for FakeStore {
    async fn time_factor_in(
        tx: &mut Tx,
        issue: IssueId,
    ) -> Result<Option<IssueTimeFactor>, FakeError> {
        Ok(tx
            .state
            .time_factors
            .iter()
            .find(|factor| factor.issue == issue)
            .cloned())
    }

    async fn time_factors_of(
        &self,
        issues: Vec<IssueId>,
    ) -> Result<Vec<IssueTimeFactor>, FakeError> {
        Ok(self.read(|state| {
            state
                .time_factors
                .iter()
                .filter(|factor| issues.contains(&factor.issue))
                .cloned()
                .collect()
        }))
    }

    async fn accepted_warnings_of(
        &self,
        issues: Vec<IssueId>,
    ) -> Result<Vec<AcceptedWarning>, FakeError> {
        Ok(self.read(|state| {
            state
                .accepted_warnings
                .iter()
                .filter(|warning| issues.contains(&warning.issue))
                .cloned()
                .collect()
        }))
    }

    async fn put_time_factor(tx: &mut Tx, factor: &IssueTimeFactor) -> Result<(), FakeError> {
        tx.state
            .time_factors
            .retain(|kept| kept.issue != factor.issue);
        tx.state.time_factors.push(factor.clone());
        Ok(())
    }

    async fn delete_time_factor(tx: &mut Tx, issue: IssueId) -> Result<bool, FakeError> {
        let before = tx.state.time_factors.len();
        tx.state.time_factors.retain(|kept| kept.issue != issue);
        Ok(tx.state.time_factors.len() != before)
    }

    async fn put_accepted_warning(tx: &mut Tx, warning: &AcceptedWarning) -> Result<(), FakeError> {
        tx.state
            .accepted_warnings
            .retain(|kept| (kept.issue, kept.kind) != (warning.issue, warning.kind));
        tx.state.accepted_warnings.push(warning.clone());
        Ok(())
    }

    async fn delete_accepted_warning(
        tx: &mut Tx,
        issue: IssueId,
        kind: WarningKind,
    ) -> Result<bool, FakeError> {
        let before = tx.state.accepted_warnings.len();
        tx.state
            .accepted_warnings
            .retain(|kept| (kept.issue, kept.kind) != (issue, kind));
        Ok(tx.state.accepted_warnings.len() != before)
    }
}
