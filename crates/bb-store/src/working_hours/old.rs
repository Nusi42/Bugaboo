//! The work days older than the keep period (ADR-0035, decision 7): counted when asked, and deleted
//! with everything that tells about them, in one change: their periods and breaks, the events of
//! their changes (with the old values and the reasons) with their entries in the outbox, and the
//! notifications about them.

use bb_app::working_hours::OldDays;
use bb_core::event::RecordKind;
use bb_core::id::UserId;
use bb_core::notification::NotificationKind;
use jiff::civil::Date;
use sea_query::{Expr, ExprTrait as _, Order, Query, SelectStatement};

use crate::db::{Reader, StoredId, Transaction};
use crate::error::StoreError;
use crate::schema::{
    Computed, Event, NotificationRow, Outbox, WorkBreakRow, WorkDayRow, WorkPeriodRow,
};

/// The IDs of the days older than `before`, of one person or of everybody.
fn doomed(before: Date, person: Option<UserId>) -> SelectStatement {
    let mut select = Query::select();
    select
        .column(WorkDayRow::Id)
        .from(WorkDayRow::Table)
        .and_where(Expr::col(WorkDayRow::Date).lt(before.to_string()));
    if let Some(person) = person {
        select.and_where(Expr::col(WorkDayRow::Person).eq(person.stored()));
    }
    select
}

/// The IDs of the events of the changes of those days.
fn their_events(before: Date, person: Option<UserId>) -> SelectStatement {
    let mut select = Query::select();
    select
        .column(Event::Id)
        .from(Event::Table)
        .and_where(Expr::col(Event::EntityKind).eq(RecordKind::WorkDay.code()))
        .and_where(Expr::col(Event::EntityId).in_subquery(doomed(before, person)));
    select
}

/// How many days older than `before` each person has, by person.
pub(super) async fn count(
    reader: &mut impl Reader,
    before: Date,
    person: Option<UserId>,
) -> Result<Vec<OldDays>, StoreError> {
    let mut select = Query::select();
    select
        .column(WorkDayRow::Person)
        .expr_as(Expr::col(WorkDayRow::Id).count(), Computed::Total)
        .from(WorkDayRow::Table)
        .and_where(Expr::col(WorkDayRow::Date).lt(before.to_string()))
        .group_by_col(WorkDayRow::Person)
        .order_by(WorkDayRow::Person, Order::Asc);
    if let Some(person) = person {
        select.and_where(Expr::col(WorkDayRow::Person).eq(person.stored()));
    }
    reader
        .fetch_all(&select)
        .await?
        .iter()
        .map(|row| {
            let total: i64 = row.get(Computed::Total)?;
            Ok(OldDays {
                person: row.id(WorkDayRow::Person)?,
                count: u64::try_from(total).map_err(|_| StoreError::Corrupt("a count"))?,
            })
        })
        .collect()
}

/// Deletes the days older than `before` with everything that tells about them; how many of whose.
pub(super) async fn delete(
    transaction: &mut Transaction,
    before: Date,
    person: Option<UserId>,
) -> Result<Vec<OldDays>, StoreError> {
    let counted = count(transaction, before, person).await?;
    if counted.is_empty() {
        return Ok(counted);
    }
    let mut notifications = Query::delete();
    notifications
        .from_table(NotificationRow::Table)
        .and_where(Expr::col(NotificationRow::Kind).eq(NotificationKind::WorkDayChanged.code()))
        .and_where(Expr::col(NotificationRow::Record).in_subquery(doomed(before, person)));
    transaction.execute(&notifications).await?;
    let mut outbox = Query::delete();
    outbox
        .from_table(Outbox::Table)
        .and_where(Expr::col(Outbox::EventId).in_subquery(their_events(before, person)));
    transaction.execute(&outbox).await?;
    let mut events = Query::delete();
    events
        .from_table(Event::Table)
        .and_where(Expr::col(Event::Id).in_subquery(their_events(before, person)));
    transaction.execute(&events).await?;
    let mut periods = Query::delete();
    periods
        .from_table(WorkPeriodRow::Table)
        .and_where(Expr::col(WorkPeriodRow::Day).in_subquery(doomed(before, person)));
    transaction.execute(&periods).await?;
    let mut breaks = Query::delete();
    breaks
        .from_table(WorkBreakRow::Table)
        .and_where(Expr::col(WorkBreakRow::Day).in_subquery(doomed(before, person)));
    transaction.execute(&breaks).await?;
    let mut days = Query::delete();
    days.from_table(WorkDayRow::Table)
        .and_where(Expr::col(WorkDayRow::Date).lt(before.to_string()));
    if let Some(person) = person {
        days.and_where(Expr::col(WorkDayRow::Person).eq(person.stored()));
    }
    transaction.execute(&days).await?;
    Ok(counted)
}
