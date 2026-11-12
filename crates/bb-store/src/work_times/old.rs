//! The deletion of work times (PRIV-05, ADR-0030, amendment M8): the away times older than a year,
//! and everything of one person who leaves. Each deletes the rows with everything that tells about
//! them, in one change: the events of their changes (with the days), the outbox entries of those
//! events and the notifications about them.

use bb_core::event::RecordKind;
use bb_core::id::{EntityId, UserId};
use bb_core::notification::NotificationKind;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use sea_query::{Expr, ExprTrait as _, Order, Query, SelectStatement};

use crate::db::{Reader, StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::schema::{
    AwayTimeRow, Computed, Event, NotificationRow, Outbox, ProjectShareRow, UsualWorkDayRow,
};

/// The IDs of the away times whose last day is before `before`.
fn doomed(before: Date) -> SelectStatement {
    let mut select = Query::select();
    select
        .column(AwayTimeRow::Id)
        .from(AwayTimeRow::Table)
        .and_where(Expr::col(AwayTimeRow::Last).lt(before.to_string()));
    select
}

/// The IDs of the away times of one person.
fn of_person(person: UserId) -> SelectStatement {
    let mut select = Query::select();
    select
        .column(AwayTimeRow::Id)
        .from(AwayTimeRow::Table)
        .and_where(Expr::col(AwayTimeRow::Person).eq(person.stored()));
    select
}

/// The IDs of the events of the kind `kind` that `condition` holds for.
fn events_where(kind: RecordKind, condition: sea_query::SimpleExpr) -> SelectStatement {
    let mut select = Query::select();
    select
        .column(Event::Id)
        .from(Event::Table)
        .and_where(Expr::col(Event::EntityKind).eq(kind.code()))
        .and_where(condition);
    select
}

/// Deletes the events that `events` finds, with their entries in the outbox.
async fn delete_events(
    transaction: &mut Transaction,
    events: &SelectStatement,
) -> Result<(), StoreError> {
    let mut outbox = Query::delete();
    outbox
        .from_table(Outbox::Table)
        .and_where(Expr::col(Outbox::EventId).in_subquery(events.clone()));
    transaction.execute(&outbox).await?;
    let mut delete = Query::delete();
    delete
        .from_table(Event::Table)
        .and_where(Expr::col(Event::Id).in_subquery(events.clone()));
    transaction.execute(&delete).await.map(|_| ())
}

/// Deletes the notifications of changed work times that `condition` holds for.
async fn delete_notifications(
    transaction: &mut Transaction,
    condition: sea_query::SimpleExpr,
) -> Result<(), StoreError> {
    let mut delete = Query::delete();
    delete
        .from_table(NotificationRow::Table)
        .and_where(Expr::col(NotificationRow::Kind).eq(NotificationKind::WorkTimesChanged.code()))
        .and_where(condition);
    transaction.execute(&delete).await.map(|_| ())
}

/// How many away times older than `before` each person has, by person.
async fn count(reader: &mut impl Reader, before: Date) -> Result<Vec<(UserId, u64)>, StoreError> {
    let mut select = Query::select();
    select
        .column(AwayTimeRow::Person)
        .expr_as(Expr::col(AwayTimeRow::Id).count(), Computed::Total)
        .from(AwayTimeRow::Table)
        .and_where(Expr::col(AwayTimeRow::Last).lt(before.to_string()))
        .group_by_col(AwayTimeRow::Person)
        .order_by(AwayTimeRow::Person, Order::Asc);
    reader
        .fetch_all(&select)
        .await?
        .iter()
        .map(|row| {
            let total: i64 = row.get(Computed::Total)?;
            Ok((
                row.id(AwayTimeRow::Person)?,
                u64::try_from(total).map_err(|_| StoreError::Corrupt("a count"))?,
            ))
        })
        .collect()
}

/// Deletes the away times whose last day is before `before`, with everything that tells about
/// them, and the events of the away times that were removed earlier and are older than `before`
/// too (a removed away time keeps its event for a year, no longer). The people who lost away
/// times, with how many.
pub(super) async fn delete(
    transaction: &mut Transaction,
    before: Date,
) -> Result<Vec<(UserId, u64)>, StoreError> {
    let counted = count(transaction, before).await?;
    if !counted.is_empty() {
        delete_notifications(
            transaction,
            Expr::col(NotificationRow::Record).in_subquery(doomed(before)),
        )
        .await?;
        delete_events(
            transaction,
            &events_where(
                RecordKind::AwayTime,
                Expr::col(Event::EntityId).in_subquery(doomed(before)),
            ),
        )
        .await?;
        let mut delete = Query::delete();
        delete
            .from_table(AwayTimeRow::Table)
            .and_where(Expr::col(AwayTimeRow::Last).lt(before.to_string()));
        transaction.execute(&delete).await?;
    }
    // An away time that is gone has an event only for the year after it was removed.
    let midnight = before
        .at(0, 0, 0, 0)
        .to_zoned(TimeZone::UTC)
        .map_err(|_| StoreError::Corrupt("a date"))?;
    let mut still_there = Query::select();
    still_there.column(AwayTimeRow::Id).from(AwayTimeRow::Table);
    delete_events(
        transaction,
        &events_where(
            RecordKind::AwayTime,
            Expr::col(Event::EntityId)
                .not_in_subquery(still_there)
                .and(Expr::col(Event::Time).lt(to_micros(midnight.timestamp()))),
        ),
    )
    .await?;
    Ok(counted)
}

/// Deletes everything of work times that belongs to a person: their usual work days, shares and
/// away times, the events of changes of those (also of away times that were removed) and of the
/// choice of *Holidays*, and the notifications about them.
pub(super) async fn delete_of_person(
    transaction: &mut Transaction,
    person: UserId,
    setting_record: EntityId,
) -> Result<(), StoreError> {
    delete_notifications(
        transaction,
        Expr::col(NotificationRow::Recipient).eq(person.stored()),
    )
    .await?;
    let id = person.stored();
    delete_events(
        transaction,
        &events_where(
            RecordKind::AwayTime,
            Expr::col(Event::EntityId)
                .in_subquery(of_person(person))
                // A removed away time has no row, but its events name the person.
                .or(Expr::col(Event::Changes).like(crate::like::containing(&person.to_string()))),
        ),
    )
    .await?;
    for kind in [RecordKind::WorkTimes, RecordKind::Share] {
        delete_events(
            transaction,
            &events_where(kind, Expr::col(Event::EntityId).eq(id.clone())),
        )
        .await?;
    }
    // The choice of *Holidays* is a setting of the person; the events of its changes go with it.
    delete_events(
        transaction,
        &events_where(
            RecordKind::Setting,
            Expr::col(Event::EntityId).eq(setting_record.stored()),
        ),
    )
    .await?;
    let mut away = Query::delete();
    away.from_table(AwayTimeRow::Table)
        .and_where(Expr::col(AwayTimeRow::Person).eq(id.clone()));
    transaction.execute(&away).await?;
    let mut usual = Query::delete();
    usual
        .from_table(UsualWorkDayRow::Table)
        .and_where(Expr::col(UsualWorkDayRow::Person).eq(id.clone()));
    transaction.execute(&usual).await?;
    let mut shares = Query::delete();
    shares
        .from_table(ProjectShareRow::Table)
        .and_where(Expr::col(ProjectShareRow::Person).eq(id));
    transaction.execute(&shares).await.map(|_| ())
}
