//! The adapter of time on issues (ADR-0035): the port of the timer and the time entries
//! (`bb_app::time::TimeStore`) and the port of time spent (`bb_app::time::spent::TimeSpentStore`).
//!
//! A date is stored as the number of days since 1970 (`bb_core::own_fields_value::day_of`, the way
//! a day of an own field is stored), so a range of dates is a range of numbers on both databases.
//! Time spent is read through the one expression of the search compiler (`search::spent`).

mod entries;
mod timer;

use bb_app::search::{SearchFailure, SearchRequest};
use bb_app::time::spent::TimeSpentStore;
use bb_core::id::{IssueId, UserId};
use bb_core::own_fields_value::{date_of, day_of};
use bb_core::time::Minutes;
use jiff::civil::Date;
use sea_query::{Expr, ExprTrait as _, Iden, Order, Query};

use crate::auth::AuthStorage;
use crate::db::StoredId as _;
use crate::error::StoreError;
use crate::schema::{IssueRow, TimeEntryRow};
use crate::search::spent::{minutes_spent, whole_sum_of_minutes};

/// A date as it is stored: the days since 1970.
fn stored_day(date: Date) -> Result<i64, StoreError> {
    day_of(date).ok_or(StoreError::Corrupt("a date"))
}

/// The date a stored number of days stands for.
fn read_day(days: i64) -> Result<Date, StoreError> {
    date_of(days).ok_or(StoreError::Corrupt("a date"))
}

/// A number of minutes as it is stored.
fn read_minutes(minutes: i64) -> Result<Minutes, StoreError> {
    Minutes::try_from(minutes).map_err(|_| StoreError::Corrupt("a number of minutes"))
}

/// The name of the one column an answer has besides its key: a sum of minutes, a count, a moment.
#[derive(Clone, Copy, Iden)]
struct Answered;

impl TimeSpentStore for AuthStorage {
    async fn spent_on_issues(
        &self,
        issues: &[IssueId],
    ) -> Result<Vec<(IssueId, Minutes)>, StoreError> {
        if issues.is_empty() {
            return Ok(Vec::new());
        }
        let mut select = Query::select();
        select
            .column((IssueRow::Table, IssueRow::Id))
            .expr_as(minutes_spent(), Answered)
            .from(IssueRow::Table)
            .and_where(
                Expr::col((IssueRow::Table, IssueRow::Id))
                    .is_in(issues.iter().map(|id| id.stored()).collect::<Vec<_>>()),
            );
        let found = self
            .database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| {
                Ok((
                    row.id::<IssueId>(IssueRow::Id)?,
                    read_minutes(row.get(Answered)?)?,
                ))
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        // One row for each issue asked about, in the order asked; one the store does not have
        // has no entries either.
        Ok(issues
            .iter()
            .map(|issue| {
                let minutes = found
                    .iter()
                    .find(|(id, _)| id == issue)
                    .map_or_else(Minutes::default, |(_, minutes)| *minutes);
                (*issue, minutes)
            })
            .collect())
    }

    async fn spent_on_search(&self, request: &SearchRequest) -> Result<Minutes, SearchFailure> {
        self.spent_on_search_in(request).await
    }

    async fn booked_per_date(
        &self,
        person: UserId,
        first: Date,
        last: Date,
    ) -> Result<Vec<(Date, Minutes)>, StoreError> {
        let mut select = Query::select();
        select
            .column((TimeEntryRow::Table, TimeEntryRow::Day))
            .expr_as(whole_sum_of_minutes(), Answered)
            .from(TimeEntryRow::Table)
            .and_where(Expr::col((TimeEntryRow::Table, TimeEntryRow::Person)).eq(person.stored()))
            .and_where(
                Expr::col((TimeEntryRow::Table, TimeEntryRow::Day))
                    .between(stored_day(first)?, stored_day(last)?),
            )
            .group_by_col((TimeEntryRow::Table, TimeEntryRow::Day))
            .order_by((TimeEntryRow::Table, TimeEntryRow::Day), Order::Asc);
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| {
                Ok((
                    read_day(row.get(TimeEntryRow::Day)?)?,
                    read_minutes(row.get(Answered)?)?,
                ))
            })
            .collect()
    }
}
