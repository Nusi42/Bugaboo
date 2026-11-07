//! Time entries: their rows in `time_entry`, and the implementation of the port of the timer and the
//! time entries. The values of the own fields of an entry are in `field_value`, under the kind
//! `time_entry`, and go with it.

use bb_app::history::opaque_id;
use bb_app::time::{DayKey, EntryCursor, TimeStore};
use bb_core::id::{IssueId, TimeEntryId, UserId};
use bb_core::time::entry::{StarterFieldMark, TimeEntry, Timer};
use bb_core::version::Version;
use jiff::Timestamp;
use jiff::civil::Date;
use sea_query::{Cond, Expr, ExprTrait as _, Func, Order, Query, SelectStatement};

use super::timer::{hold_time_of, insert_starter_field, put_timer, starter_fields_in, timer_in};
use super::{Answered, read_day, read_minutes, stored_day};
use crate::auth::AuthStorage;
use crate::db::{Database, Row, StoredId as _, Transaction, from_micros, to_micros};
use crate::error::StoreError;
use crate::schema::{FieldValueRow, TimeEntryRow, TimeEntryValueRow};
use crate::search::scalar;

/// The microseconds in a minute: the end of an entry with a start is its start plus its length.
const MICROS_PER_MINUTE: i64 = 60_000_000;

/// The columns of an entry, in the order they are written.
const ENTRY_COLUMNS: [TimeEntryRow; 10] = [
    TimeEntryRow::Id,
    TimeEntryRow::Person,
    TimeEntryRow::Issue,
    TimeEntryRow::Day,
    TimeEntryRow::Minutes,
    TimeEntryRow::Note,
    TimeEntryRow::Billable,
    TimeEntryRow::StartedAt,
    TimeEntryRow::CreatedAt,
    TimeEntryRow::Version,
];

fn entry_select() -> SelectStatement {
    let mut select = Query::select();
    select.columns(ENTRY_COLUMNS).from(TimeEntryRow::Table);
    select
}

fn read_entry(row: &Row) -> Result<TimeEntry, StoreError> {
    Ok(TimeEntry {
        id: row.id::<TimeEntryId>(TimeEntryRow::Id)?,
        person: row.id::<UserId>(TimeEntryRow::Person)?,
        issue: row.id::<IssueId>(TimeEntryRow::Issue)?,
        date: read_day(row.get(TimeEntryRow::Day)?)?,
        minutes: read_minutes(row.get(TimeEntryRow::Minutes)?)?,
        note: row.get(TimeEntryRow::Note)?,
        billable: row.get(TimeEntryRow::Billable)?,
        started_at: row.optional_time(TimeEntryRow::StartedAt)?,
        created_at: row.time(TimeEntryRow::CreatedAt)?,
        version: row.get(TimeEntryRow::Version)?,
    })
}

/// A column of an entry, named with its table.
fn column(name: TimeEntryRow) -> Expr {
    Expr::col((TimeEntryRow::Table, name))
}

/// The time entries on an issue, newest first, a page after `after`.
async fn entries_of_issue(
    database: &Database,
    issue: IssueId,
    after: EntryCursor,
    limit: u64,
) -> Result<Vec<TimeEntry>, StoreError> {
    let mut select = entry_select();
    select.and_where(column(TimeEntryRow::Issue).eq(issue.stored()));
    if let Some(after) = after {
        select.and_where(column(TimeEntryRow::Id).lt(after.stored()));
    }
    select
        .order_by((TimeEntryRow::Table, TimeEntryRow::Id), Order::Desc)
        .limit(limit);
    database
        .fetch_all(&select)
        .await?
        .iter()
        .map(read_entry)
        .collect()
}

impl Database {
    /// Every time entry on an issue, newest first: what the check after a load run reads.
    ///
    /// # Errors
    /// Fails when the database fails or a row is unreadable.
    #[cfg(any(test, feature = "testing"))]
    pub async fn stored_time_entries(&self, issue: IssueId) -> Result<Vec<TimeEntry>, StoreError> {
        entries_of_issue(self, issue, None, u64::MAX >> 1).await
    }
}

impl TimeStore for AuthStorage {
    async fn timer_in(
        transaction: &mut Transaction,
        person: UserId,
    ) -> Result<Option<Timer>, StoreError> {
        timer_in(transaction, person).await
    }

    async fn time_entry_in(
        transaction: &mut Transaction,
        id: TimeEntryId,
    ) -> Result<Option<TimeEntry>, StoreError> {
        let mut select = entry_select();
        select.and_where(column(TimeEntryRow::Id).eq(id.stored()));
        transaction
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_entry)
            .transpose()
    }

    async fn starter_fields_in(
        transaction: &mut Transaction,
    ) -> Result<Vec<StarterFieldMark>, StoreError> {
        starter_fields_in(transaction).await
    }

    async fn entries_on_date_in(
        transaction: &mut Transaction,
        person: UserId,
        date: Date,
    ) -> Result<u64, StoreError> {
        let mut select = Query::select();
        select
            .expr_as(Func::count(Expr::col(sea_query::Asterisk)), Answered)
            .from(TimeEntryRow::Table)
            .and_where(column(TimeEntryRow::Person).eq(person.stored()))
            .and_where(column(TimeEntryRow::Day).eq(stored_day(date)?));
        let counted: i64 = match transaction.fetch_optional(&select).await? {
            Some(row) => row.get(Answered)?,
            None => 0,
        };
        u64::try_from(counted).map_err(|_| StoreError::Corrupt("a count"))
    }

    async fn last_timed_end_in(
        transaction: &mut Transaction,
        person: UserId,
    ) -> Result<Option<Timestamp>, StoreError> {
        let end = column(TimeEntryRow::StartedAt)
            .add(column(TimeEntryRow::Minutes).mul(MICROS_PER_MINUTE));
        let mut select = Query::select();
        select
            .expr_as(Func::max(end), Answered)
            .from(TimeEntryRow::Table)
            .and_where(column(TimeEntryRow::Person).eq(person.stored()))
            .and_where(column(TimeEntryRow::StartedAt).is_not_null());
        let end: Option<i64> = match transaction.fetch_optional(&select).await? {
            Some(row) => row.get(Answered)?,
            None => None,
        };
        end.map(from_micros).transpose()
    }

    async fn entries_of_person(
        &self,
        person: UserId,
        first: Date,
        last: Date,
        after: EntryCursor,
        limit: u64,
    ) -> Result<Vec<TimeEntry>, StoreError> {
        let mut select = entry_select();
        select
            .and_where(column(TimeEntryRow::Person).eq(person.stored()))
            .and_where(column(TimeEntryRow::Day).between(stored_day(first)?, stored_day(last)?));
        if let Some(after) = after {
            // After the entry the page before ended with: a later date, or the same date and made
            // later (the IDs of entries are in the order they were made).
            let mut day_of_after = Query::select();
            day_of_after
                .column(TimeEntryRow::Day)
                .from(TimeEntryRow::Table)
                .and_where(Expr::col(TimeEntryRow::Id).eq(after.stored()));
            let day = scalar(day_of_after);
            select.cond_where(
                Cond::any()
                    .add(column(TimeEntryRow::Day).gt(day.clone()))
                    .add(
                        Cond::all()
                            .add(column(TimeEntryRow::Day).eq(day))
                            .add(column(TimeEntryRow::Id).gt(after.stored())),
                    ),
            );
        }
        select
            .order_by((TimeEntryRow::Table, TimeEntryRow::Day), Order::Asc)
            .order_by((TimeEntryRow::Table, TimeEntryRow::Id), Order::Asc)
            .limit(limit);
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_entry)
            .collect()
    }

    async fn entries_of_issue_by_date(
        &self,
        issue: IssueId,
        after: Option<DayKey>,
        limit: u64,
    ) -> Result<Vec<(TimeEntry, DayKey)>, StoreError> {
        let mut select = entry_select();
        select
            .column((TimeEntryRow::Table, TimeEntryRow::ShownHash))
            .and_where(column(TimeEntryRow::Issue).eq(issue.stored()));
        if let Some(after) = after {
            let day = stored_day(after.day)?;
            select.and_where(
                column(TimeEntryRow::Day)
                    .lt(day)
                    .or(column(TimeEntryRow::Day)
                        .eq(day)
                        .and(column(TimeEntryRow::ShownHash).lt(after.hash))),
            );
        }
        select
            .order_by((TimeEntryRow::Table, TimeEntryRow::Day), Order::Desc)
            .order_by((TimeEntryRow::Table, TimeEntryRow::ShownHash), Order::Desc)
            .order_by((TimeEntryRow::Table, TimeEntryRow::Id), Order::Desc)
            .limit(limit);
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| {
                let entry = read_entry(row)?;
                let key = DayKey {
                    day: entry.date,
                    hash: row.get(TimeEntryRow::ShownHash)?,
                };
                Ok((entry, key))
            })
            .collect()
    }

    async fn entries_of_issue(
        &self,
        issue: IssueId,
        after: EntryCursor,
        limit: u64,
    ) -> Result<Vec<TimeEntry>, StoreError> {
        entries_of_issue(self.database(), issue, after, limit).await
    }

    async fn put_timer(
        transaction: &mut Transaction,
        timer: &Timer,
        version: Option<Version>,
    ) -> Result<bool, StoreError> {
        put_timer(transaction, timer, version).await
    }

    async fn hold_time_of(transaction: &mut Transaction, person: UserId) -> Result<(), StoreError> {
        hold_time_of(transaction, person).await
    }

    async fn insert_time_entry(
        transaction: &mut Transaction,
        entry: &TimeEntry,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(TimeEntryRow::Table)
            .columns(ENTRY_COLUMNS.into_iter().chain([TimeEntryRow::ShownHash]))
            .values([
                Expr::val(entry.id.stored()),
                Expr::val(entry.person.stored()),
                Expr::val(entry.issue.stored()),
                Expr::val(stored_day(entry.date)?),
                Expr::val(i64::from(entry.minutes)),
                Expr::val(entry.note.clone()),
                Expr::val(entry.billable),
                Expr::val(entry.started_at.map(to_micros)),
                Expr::val(to_micros(entry.created_at)),
                Expr::val(entry.version),
                Expr::val(opaque_id(&entry.id.to_string())),
            ])
            .map_err(|_| StoreError::Corrupt("a time entry"))?;
        transaction.execute(&insert).await?;
        Ok(())
    }

    async fn update_time_entry(
        transaction: &mut Transaction,
        entry: &TimeEntry,
        version: Version,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(TimeEntryRow::Table)
            .value(TimeEntryRow::Day, stored_day(entry.date)?)
            .value(TimeEntryRow::Minutes, i64::from(entry.minutes))
            .value(TimeEntryRow::Note, entry.note.clone())
            .value(TimeEntryRow::Billable, entry.billable)
            .value(TimeEntryRow::Version, entry.version)
            .and_where(Expr::col(TimeEntryRow::Id).eq(entry.id.stored()))
            .and_where(Expr::col(TimeEntryRow::Version).eq(version.get()));
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn delete_time_entry(
        transaction: &mut Transaction,
        id: TimeEntryId,
        version: Version,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(TimeEntryRow::Table)
            .and_where(Expr::col(TimeEntryRow::Id).eq(id.stored()))
            .and_where(Expr::col(TimeEntryRow::Version).eq(version.get()));
        if transaction.execute(&delete).await? == 0 {
            return Ok(false);
        }
        let mut values = Query::delete();
        values
            .from_table(TimeEntryValueRow::Table)
            .and_where(Expr::col(FieldValueRow::EntityId).eq(id.stored()));
        transaction.execute(&values).await?;
        Ok(true)
    }

    async fn insert_starter_field(
        transaction: &mut Transaction,
        mark: &StarterFieldMark,
    ) -> Result<bool, StoreError> {
        insert_starter_field(transaction, mark).await
    }
}
