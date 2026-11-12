//! The port of the plan: what the timeline keeps beyond issues, relations and work times (PLAN-13,
//! PLAN-14): the time factor of an issue and the plan warnings people accepted. Like the other
//! ports, reads take `&self` and writes are associated functions that run inside `record_change`,
//! so a change and its domain event are committed together (ARCH-EVENTS).

use std::future::Future;

use bb_core::id::IssueId;
use bb_core::plan::record::{AcceptedWarning, IssueTimeFactor};
use bb_core::plan::warning::WarningKind;

use crate::store_macros::{queries, reads, writes};

/// What the plan needs from storage.
pub trait TimelineStore: crate::issues::IssueStore + 'static {
    queries! {
        /// The time factor of an issue, when it is not ×1.
        fn time_factor(issue: IssueId) -> Option<IssueTimeFactor> = time_factor_in;
    }

    reads! {
        /// The time factors of these issues that are not ×1.
        fn time_factors_of(issues: Vec<IssueId>) -> Vec<IssueTimeFactor>;
        /// The accepted plan warnings of these issues.
        fn accepted_warnings_of(issues: Vec<IssueId>) -> Vec<AcceptedWarning>;
    }

    writes! {
        /// Writes the time factor of an issue, in place of the one it had.
        fn put_time_factor(factor: &IssueTimeFactor) -> ();
        /// Takes the time factor of an issue away (back to ×1); `false` when it had none.
        fn delete_time_factor(issue: IssueId) -> bool;
        /// Writes an accepted plan warning, in place of the one of the same issue and kind.
        fn put_accepted_warning(warning: &AcceptedWarning) -> ();
        /// Takes the accepted warning of this issue and kind away; `false` when there was none.
        fn delete_accepted_warning(issue: IssueId, kind: WarningKind) -> bool;
    }
}
