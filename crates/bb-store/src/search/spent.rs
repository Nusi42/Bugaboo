//! Time spent in SQL (ADR-0035, decision 4): the minutes booked on an issue, the sum of the lengths
//! of its time entries, written once. The port of time spent reads it for a list of issues and for
//! the issues a search finds, and the field *Time spent* of the search language and the totals
//! build on the same expression; a second sum of time entries anywhere in the store is a finding.

use bb_app::search::{SearchFailure, SearchRequest};
use bb_core::time::Minutes;
use sea_query::{Expr, ExprTrait as _, Func, Query};

use super::order::bigint;
use super::statements::{read_minutes, spent_statement};
use super::{SEARCH_TIME, failed, scalar, stopped};
use crate::auth::AuthStorage;
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
            .map(read_minutes)
            .transpose()
            .map_err(failed)
            .map(Option::unwrap_or_default)
    }
}
