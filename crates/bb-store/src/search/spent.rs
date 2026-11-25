//! Time spent in SQL (ADR-0035, decision 4): the minutes booked on an issue, the sum of the lengths
//! of its time entries, written once. The port of time spent reads it for a list of issues, and the
//! field *Time spent* of the search language and the totals build on the same expression; a second sum of time entries anywhere in the store is a finding.
//!
//! The worked-out *Time left* ([`time_left`]) is built on it: the second place of the rule of
//! `bb_core::time::left`, the approved pair of coding-standards §1, held to it by the vectors of
//! `tools/testdata/time-left.json` and the test `time_03_the_compiled_time_left_matches_the_vectors`.
//! The totals of a search (`totals.rs`) sum the estimate, the time left and the time spent through
//! the same expressions, reading each issue once.

use sea_query::{CaseStatement, Expr, ExprTrait as _, Func, Query};

use super::order::bigint;
use super::scalar;
use crate::schema::{IssueRow, TimeEntryRow};

/// The sum of the lengths of the time entries a statement reads, in whole minutes (`NULL` when it
/// reads none). The one sum of time entries: the minutes spent on an issue and a person's minutes
/// of a date are both this sum.
pub(crate) fn sum_of_minutes() -> Expr {
    Func::sum(Expr::col((TimeEntryRow::Table, TimeEntryRow::Minutes))).into()
}

/// [`sum_of_minutes`] as a whole number on both databases, for a statement that groups the entries
/// (a person's minutes of each date).
pub(crate) fn whole_sum_of_minutes() -> Expr {
    bigint(sum_of_minutes())
}

/// The minutes spent on the issue of the statement (`issue.id`): the sum of the lengths of its time
/// entries, 0 for an issue without any. A whole number on both databases (PostgreSQL sums whole
/// numbers into a decimal, so the sum is cast back).
pub(crate) fn minutes_spent() -> Expr {
    let mut entries = Query::select();
    entries
        .expr(sum_of_minutes())
        .from(TimeEntryRow::Table)
        .and_where(
            Expr::col((TimeEntryRow::Table, TimeEntryRow::Issue))
                .equals((IssueRow::Table, IssueRow::Id)),
        );
    bigint(Func::coalesce([scalar(entries), Expr::val(0_i64)]).into())
}

/// *Time left* of the issue of the statement, in whole minutes: the typed value while there is one,
/// else the estimate minus the minutes spent and never below zero, `NULL` without either. The rule
/// of `bb_core::time::left`, as one expression. The minutes spent are read once for an issue, as
/// the larger of the difference and zero, not once to compare and again to subtract: the totals of
/// a big project work it out for every issue that has an estimate.
pub(crate) fn time_left() -> Expr {
    let typed = Expr::col((IssueRow::Table, IssueRow::TimeLeft));
    let estimated = Expr::col((IssueRow::Table, IssueRow::EstimatedTime));
    CaseStatement::new()
        .case(typed.clone().is_not_null(), typed)
        .case(estimated.clone().is_null(), Expr::null())
        .finally(Func::greatest([
            estimated.sub(minutes_spent()),
            Expr::val(0_i64),
        ]))
        .into()
}
