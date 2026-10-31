//! Time spent in SQL (ADR-0035, decision 4): the minutes booked on an issue, the sum of the lengths
//! of its time entries, written once. The port of time spent reads it for a list of issues and for
//! the issues a search finds, and the field *Time spent* of the search language and the totals
//! build on the same expression; a second sum of time entries anywhere in the store is a finding.
//!
//! The worked-out *Time left* ([`time_left`]) is built on it: the second place of the rule of
//! `bb_core::time::left`, the approved pair of coding-standards §1, held to it by the vectors of
//! `tools/testdata/time-left.json` and the test `time_03_the_compiled_time_left_matches_the_vectors`.
//! The totals of the lengths of a search ([`lengths_statement`]) sum the estimate and the time left.

use bb_app::search::{FieldTarget, LengthTotals, SearchFailure, SearchRequest};
use bb_core::fields::Field;
use bb_core::time::Minutes;
use sea_query::{CaseStatement, Expr, ExprTrait as _, Func, Iden, Query};

use super::order::bigint;
use super::source::Subject;
use super::statements::{Built, finish, found, read_spent, spent_statement};
use super::{CompileError, Compiler, SEARCH_TIME, failed, scalar, stopped};
use crate::auth::AuthStorage;
use crate::db::{Dialect, Row};
use crate::error::StoreError;
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
/// of `bb_core::time::left`, as one expression.
pub(crate) fn time_left() -> Expr {
    let typed = Expr::col((IssueRow::Table, IssueRow::TimeLeft));
    let estimated = Expr::col((IssueRow::Table, IssueRow::EstimatedTime));
    let spent = minutes_spent();
    CaseStatement::new()
        .case(typed.clone().is_not_null(), typed)
        .case(estimated.clone().is_null(), Expr::null())
        .case(estimated.clone().gt(spent.clone()), estimated.sub(spent))
        .finally(Expr::val(0_i64))
        .into()
}

/// The names of the columns of the totals.
#[derive(Clone, Copy)]
enum Total {
    Found,
    Estimated,
    Left,
}

impl Iden for Total {
    fn unquoted(&self) -> &'static str {
        match self {
            Self::Found => "found",
            Self::Estimated => "estimated",
            Self::Left => "time_left",
        }
    }
}

impl Compiler<'_> {
    /// The sum of a length of the issues of the statement, by the word of its field; `NULL` for a
    /// field that is not among the readable ones, which is never summed.
    fn summed(&self, field: Field) -> Expr {
        match self.target(&field.search_word()) {
            Ok(target @ (FieldTarget::EstimatedTime | FieldTarget::TimeLeft)) => {
                match self.subject_of(target).0 {
                    Subject::Column(value) => bigint(Func::sum(value).into()),
                    Subject::Linked(..) => Expr::null(),
                }
            }
            _ => Expr::null(),
        }
    }
}

/// Whether the search finds any issue, and the sums of *Estimated time* and *Time left* of the
/// issues it finds: one statement, from the same sight and filter as the page.
pub(crate) fn lengths_statement(
    request: &SearchRequest,
    dialect: Dialect,
) -> Result<Built, CompileError> {
    let compiler = Compiler::new(request, dialect);
    let mut select = Query::select();
    select
        .expr_as(
            Expr::case(Func::count(Expr::col(sea_query::Asterisk)).gt(0), 1_i64).finally(0_i64),
            Total::Found,
        )
        .expr_as(compiler.summed(Field::EstimatedTime), Total::Estimated)
        .expr_as(compiler.summed(Field::TimeLeft), Total::Left)
        .from(IssueRow::Table)
        .cond_where(found(&compiler)?);
    Ok(finish(&compiler, select))
}

/// The totals from the row of [`lengths_statement`].
pub(crate) fn read_lengths(row: &Row) -> Result<LengthTotals, StoreError> {
    let minutes = |column: Total| -> Result<Option<Minutes>, StoreError> {
        row.get::<Option<i64>>(column)?
            .map(|sum| Minutes::try_from(sum).map_err(|_| StoreError::Corrupt("a total")))
            .transpose()
    };
    Ok(LengthTotals {
        found_any: row.get::<i64>(Total::Found)? > 0,
        estimated: minutes(Total::Estimated)?,
        time_left: minutes(Total::Left)?,
    })
}

impl AuthStorage {
    /// The minutes booked on the issues a search finds, in one statement with the reader's sight
    /// and fields in it (the port's `spent_on_search`).
    pub(crate) async fn spent_on_search_in(
        &self,
        request: &SearchRequest,
    ) -> Result<Minutes, SearchFailure> {
        let built = spent_statement(request, self.database().dialect()).map_err(failed)?;
        let rows = self
            .database()
            .fetch_all_within(&built, SEARCH_TIME)
            .await
            .map_err(stopped)?;
        rows.first()
            .map(read_spent)
            .transpose()
            .map_err(failed)
            .map(Option::unwrap_or_default)
    }
}
