//! The adapter of the port of working hours (`bb_app::working_hours::WorkingHoursStore`): work days
//! with their periods and breaks (TIME-08), and the deletion of the days older than the keep period
//! with everything that tells about them (ADR-0035, decision 7).

mod old;

use bb_app::working_hours::{OldDays, WorkingHoursStore};
use bb_core::DomainEvent;
use bb_core::event::RecordKind;
use bb_core::id::{UserId, WorkDayId};
use bb_core::working_time::record::{StoredDay, WorkDay};
use bb_core::working_time::{Span, Times, Warning};
use jiff::Timestamp;
use jiff::civil::Date;
use sea_query::{Expr, ExprTrait as _, Iden, Order, Query, SelectStatement};

use crate::auth::AuthStorage;
use crate::db::{Database, Reader, Row, StoredId, Transaction, from_micros, to_micros};
use crate::error::StoreError;
use crate::events::{event_columns, read_event};
use crate::locks::{LockClass, lock_key};
use crate::schema::{Event, WorkBreakRow, WorkDayRow, WorkPeriodRow};

fn day_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            WorkDayRow::Id,
            WorkDayRow::Person,
            WorkDayRow::Date,
            WorkDayRow::Zone,
            WorkDayRow::Warnings,
            WorkDayRow::UpdatedAt,
            WorkDayRow::Version,
        ])
        .from(WorkDayRow::Table);
    select
}

fn read_day(row: &Row) -> Result<WorkDay, StoreError> {
    let date: String = row.get(WorkDayRow::Date)?;
    Ok(WorkDay {
        id: row.id(WorkDayRow::Id)?,
        person: row.id(WorkDayRow::Person)?,
        date: date.parse().map_err(|_| StoreError::Corrupt("a date"))?,
        zone: row.get(WorkDayRow::Zone)?,
        warnings: serde_json::from_str(&row.get::<String>(WorkDayRow::Warnings)?)?,
        updated_at: row.time(WorkDayRow::UpdatedAt)?,
        version: row.get(WorkDayRow::Version)?,
    })
}

/// The table of the periods or of the breaks of a day, with its columns: the table, the day, the
/// start and the end.
const PERIODS: [WorkPeriodRow; 4] = [
    WorkPeriodRow::Table,
    WorkPeriodRow::Day,
    WorkPeriodRow::Start,
    WorkPeriodRow::End,
];
const BREAKS: [WorkBreakRow; 4] = [
    WorkBreakRow::Table,
    WorkBreakRow::Day,
    WorkBreakRow::Start,
    WorkBreakRow::End,
];

/// The stretches of one table (periods or breaks) of these days, each day's in the order they
/// started.
async fn spans_of<T: Iden + Copy + Send + Sync + 'static>(
    reader: &mut impl Reader,
    [table, day, start, end]: [T; 4],
    days: &[WorkDayId],
) -> Result<Vec<(WorkDayId, Span)>, StoreError> {
    if days.is_empty() {
        return Ok(Vec::new());
    }
    let mut select = Query::select();
    select
        .columns([day, start, end])
        .from(table)
        .and_where(Expr::col(day).is_in(days.iter().copied().map(StoredId::stored)))
        .order_by(day, Order::Asc)
        .order_by(start, Order::Asc);
    reader
        .fetch_all(&select)
        .await?
        .iter()
        .map(|row| {
            Ok((
                row.id(day)?,
                Span {
                    start: row.time(start)?,
                    end: row.optional_time(end)?,
                },
            ))
        })
        .collect()
}

/// Writes the stretches of one table of a day, in place of the ones it had.
async fn write_spans_of<T: Iden + Copy + Send + Sync + 'static>(
    transaction: &mut Transaction,
    [table, day, start, end]: [T; 4],
    id: WorkDayId,
    spans: &[Span],
) -> Result<(), StoreError> {
    let mut delete = Query::delete();
    delete
        .from_table(table)
        .and_where(Expr::col(day).eq(id.stored()));
    transaction.execute(&delete).await?;
    if spans.is_empty() {
        return Ok(());
    }
    let mut insert = Query::insert();
    insert.into_table(table).columns([day, start, end]);
    for span in spans {
        insert
            .values([
                Expr::val(id.stored()),
                Expr::val(to_micros(span.start)),
                Expr::val(span.end.map(to_micros)),
            ])
            .map_err(|_| StoreError::Corrupt("a stretch of a work day"))?;
    }
    transaction.execute(&insert).await.map(|_| ())
}

/// The days that `select` finds, with their periods and breaks: three statements, whatever the
/// number of days.
async fn stored_days(
    reader: &mut impl Reader,
    select: &SelectStatement,
) -> Result<Vec<StoredDay>, StoreError> {
    let days = reader
        .fetch_all(select)
        .await?
        .iter()
        .map(read_day)
        .collect::<Result<Vec<_>, _>>()?;
    let ids: Vec<WorkDayId> = days.iter().map(|day| day.id).collect();
    let periods = spans_of(reader, PERIODS, &ids).await?;
    let breaks = spans_of(reader, BREAKS, &ids).await?;
    let of = |spans: &[(WorkDayId, Span)], id: WorkDayId| -> Vec<Span> {
        spans
            .iter()
            .filter(|(day, _)| *day == id)
            .map(|(_, span)| *span)
            .collect()
    };
    Ok(days
        .into_iter()
        .map(|day| StoredDay {
            times: Times {
                periods: of(&periods, day.id),
                breaks: of(&breaks, day.id),
            },
            day,
        })
        .collect())
}

/// The one day that `select` finds, if any.
async fn one_day(
    reader: &mut impl Reader,
    select: &SelectStatement,
) -> Result<Option<StoredDay>, StoreError> {
    Ok(stored_days(reader, select).await?.into_iter().next())
}

/// The days of a person.
fn days_of(person: UserId) -> SelectStatement {
    let mut select = day_select();
    select.and_where(Expr::col(WorkDayRow::Person).eq(person.stored()));
    select
}

/// Writes the periods and breaks of a day, in place of the ones it had.
async fn write_spans(transaction: &mut Transaction, day: &StoredDay) -> Result<(), StoreError> {
    write_spans_of(transaction, PERIODS, day.day.id, &day.times.periods).await?;
    write_spans_of(transaction, BREAKS, day.day.id, &day.times.breaks).await
}

fn warnings_text(warnings: &[Warning]) -> Result<String, StoreError> {
    Ok(serde_json::to_string(warnings)?)
}

impl Database {
    /// Every work day of a person, oldest first: what the check after a load run reads.
    ///
    /// # Errors
    /// Fails when the database fails or a row is unreadable.
    #[cfg(any(test, feature = "testing"))]
    pub async fn stored_work_days(&self, person: UserId) -> Result<Vec<StoredDay>, StoreError> {
        let mut select = days_of(person);
        select.order_by(WorkDayRow::Date, Order::Asc);
        stored_days(&mut &*self, &select).await
    }
}

impl WorkingHoursStore for AuthStorage {
    async fn work_day_in(
        transaction: &mut Transaction,
        person: UserId,
        date: Date,
    ) -> Result<Option<StoredDay>, StoreError> {
        let mut select = days_of(person);
        select.and_where(Expr::col(WorkDayRow::Date).eq(date.to_string()));
        one_day(transaction, &select).await
    }

    async fn work_day_before_in(
        transaction: &mut Transaction,
        person: UserId,
        date: Date,
    ) -> Result<Option<StoredDay>, StoreError> {
        let mut select = days_of(person);
        select
            .and_where(Expr::col(WorkDayRow::Date).lt(date.to_string()))
            .order_by(WorkDayRow::Date, Order::Desc)
            .limit(1);
        one_day(transaction, &select).await
    }

    async fn work_day_after_in(
        transaction: &mut Transaction,
        person: UserId,
        date: Date,
    ) -> Result<Option<StoredDay>, StoreError> {
        let mut select = days_of(person);
        select
            .and_where(Expr::col(WorkDayRow::Date).gt(date.to_string()))
            .order_by(WorkDayRow::Date, Order::Asc)
            .limit(1);
        one_day(transaction, &select).await
    }

    async fn open_work_day_in(
        transaction: &mut Transaction,
        person: UserId,
    ) -> Result<Option<StoredDay>, StoreError> {
        let mut open = Query::select();
        open.column(WorkPeriodRow::Day)
            .from(WorkPeriodRow::Table)
            .and_where(Expr::col(WorkPeriodRow::End).is_null());
        let mut select = days_of(person);
        select
            .and_where(Expr::col(WorkDayRow::Id).in_subquery(open))
            .order_by(WorkDayRow::Date, Order::Desc)
            .limit(1);
        one_day(transaction, &select).await
    }

    async fn last_work_day_change_in(
        transaction: &mut Transaction,
        person: UserId,
    ) -> Result<Option<Timestamp>, StoreError> {
        let mut select = Query::select();
        select
            .column(WorkDayRow::UpdatedAt)
            .from(WorkDayRow::Table)
            .and_where(Expr::col(WorkDayRow::Person).eq(person.stored()))
            .order_by(WorkDayRow::UpdatedAt, Order::Desc)
            .limit(1);
        transaction
            .fetch_optional(&select)
            .await?
            .map(|row| row.get(WorkDayRow::UpdatedAt).and_then(from_micros))
            .transpose()
    }

    async fn work_days(
        &self,
        person: UserId,
        first: Date,
        last: Date,
    ) -> Result<Vec<StoredDay>, StoreError> {
        let mut select = days_of(person);
        select
            .and_where(Expr::col(WorkDayRow::Date).gte(first.to_string()))
            .and_where(Expr::col(WorkDayRow::Date).lte(last.to_string()))
            .order_by(WorkDayRow::Date, Order::Asc);
        stored_days(&mut self.database(), &select).await
    }

    async fn work_day_by_id(&self, id: WorkDayId) -> Result<Option<StoredDay>, StoreError> {
        let mut select = day_select();
        select.and_where(Expr::col(WorkDayRow::Id).eq(id.stored()));
        one_day(&mut self.database(), &select).await
    }

    async fn work_day_events(
        &self,
        id: WorkDayId,
        limit: u64,
    ) -> Result<Vec<DomainEvent>, StoreError> {
        let mut select = Query::select();
        select
            .columns(event_columns())
            .from(Event::Table)
            .and_where(Expr::col(Event::EntityKind).eq(RecordKind::WorkDay.code()))
            .and_where(Expr::col(Event::EntityId).eq(id.stored()))
            .order_by(Event::Id, Order::Desc)
            .limit(limit);
        let mut events = self
            .database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_event)
            .collect::<Result<Vec<_>, _>>()?;
        events.reverse();
        Ok(events)
    }

    async fn work_days_with_reasons(
        &self,
        ids: Vec<WorkDayId>,
    ) -> Result<Vec<WorkDayId>, StoreError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut select = Query::select();
        select
            .column(Event::EntityId)
            .from(Event::Table)
            .and_where(Expr::col(Event::EntityKind).eq(RecordKind::WorkDay.code()))
            .and_where(
                Expr::col(Event::EntityId)
                    .is_in(ids.iter().map(|id| id.stored()).collect::<Vec<_>>()),
            )
            .and_where(Expr::col(Event::Reason).is_not_null())
            .group_by_col(Event::EntityId);
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| row.id(Event::EntityId))
            .collect()
    }

    async fn old_work_days(&self, before: Date) -> Result<Vec<OldDays>, StoreError> {
        old::count(&mut self.database(), before, None).await
    }

    async fn lock_work_days_of(
        transaction: &mut Transaction,
        person: UserId,
    ) -> Result<(), StoreError> {
        transaction
            .advisory_lock(LockClass::WorkDays, lock_key(person.to_bytes()))
            .await
    }

    async fn insert_work_day(
        transaction: &mut Transaction,
        day: &StoredDay,
    ) -> Result<(), StoreError> {
        let stored = &day.day;
        let mut insert = Query::insert();
        insert
            .into_table(WorkDayRow::Table)
            .columns([
                WorkDayRow::Id,
                WorkDayRow::Person,
                WorkDayRow::Date,
                WorkDayRow::Zone,
                WorkDayRow::Warnings,
                WorkDayRow::UpdatedAt,
                WorkDayRow::Version,
            ])
            .values([
                Expr::val(stored.id.stored()),
                Expr::val(stored.person.stored()),
                Expr::val(stored.date.to_string()),
                Expr::val(stored.zone.clone()),
                Expr::val(warnings_text(&stored.warnings)?),
                Expr::val(to_micros(stored.updated_at)),
                Expr::val(stored.version),
            ])
            .map_err(|_| StoreError::Corrupt("a work day"))?;
        transaction.execute(&insert).await?;
        write_spans(transaction, day).await
    }

    async fn update_work_day(
        transaction: &mut Transaction,
        day: &StoredDay,
        version: i64,
    ) -> Result<bool, StoreError> {
        let stored = &day.day;
        let mut update = Query::update();
        update
            .table(WorkDayRow::Table)
            .values([
                (WorkDayRow::Zone, Expr::val(stored.zone.clone())),
                (
                    WorkDayRow::Warnings,
                    Expr::val(warnings_text(&stored.warnings)?),
                ),
                (
                    WorkDayRow::UpdatedAt,
                    Expr::val(to_micros(stored.updated_at)),
                ),
                (WorkDayRow::Version, Expr::val(stored.version)),
            ])
            .and_where(Expr::col(WorkDayRow::Id).eq(stored.id.stored()))
            .and_where(Expr::col(WorkDayRow::Version).eq(version));
        if transaction.execute(&update).await? == 0 {
            return Ok(false);
        }
        write_spans(transaction, day).await?;
        Ok(true)
    }

    async fn set_work_day_warnings(
        transaction: &mut Transaction,
        id: WorkDayId,
        warnings: &[Warning],
    ) -> Result<(), StoreError> {
        let mut update = Query::update();
        update
            .table(WorkDayRow::Table)
            .values([(WorkDayRow::Warnings, Expr::val(warnings_text(warnings)?))])
            .and_where(Expr::col(WorkDayRow::Id).eq(id.stored()));
        transaction.execute(&update).await.map(|_| ())
    }

    async fn delete_old_work_days(
        transaction: &mut Transaction,
        before: Date,
        person: Option<UserId>,
    ) -> Result<Vec<OldDays>, StoreError> {
        old::delete(transaction, before, person).await
    }
}
