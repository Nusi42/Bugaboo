//! The time entries past their keep period (M9-Q2, ADR-0045, decision 6): counted when asked, and
//! deleted only when an administrator says so, a step at a time, each with everything that tells
//! about its entries: the values of their fields, the notifications about them, and the events of
//! their changes (with the note and the fields, M6-F45) with their entries in the outbox and the
//! history of the issue.

use bb_app::keeping::OldCount;
use bb_core::event::RecordKind;
use bb_core::id::UserId;
use jiff::civil::Date;
use sea_query::{Expr, ExprTrait as _, Order, Query, SelectStatement};

use super::stored_day;
use crate::db::{Reader, StoredId, Transaction};
use crate::error::StoreError;
use crate::keeping::old_records::choose;
use crate::keeping::{delete_events, events_about};
use crate::schema::{Computed, FieldValueRow, NotificationRow, TimeEntryRow, TimeEntryValueRow};

/// The entries with a day before `before`, of one person or of everybody (no column chosen yet).
fn past(before: Date, person: Option<UserId>) -> Result<SelectStatement, StoreError> {
    let mut select = Query::select();
    select
        .from(TimeEntryRow::Table)
        .and_where(Expr::col(TimeEntryRow::Day).lt(stored_day(before)?));
    if let Some(person) = person {
        select.and_where(Expr::col(TimeEntryRow::Person).eq(person.stored()));
    }
    Ok(select)
}

/// How many entries with a day before `before` each person has, by person.
pub(crate) async fn count(
    reader: &mut impl Reader,
    before: Date,
    person: Option<UserId>,
) -> Result<Vec<OldCount>, StoreError> {
    let mut select = past(before, person)?;
    select
        .column(TimeEntryRow::Person)
        .expr_as(Expr::col(TimeEntryRow::Id).count(), Computed::Total)
        .group_by_col(TimeEntryRow::Person)
        .order_by(TimeEntryRow::Person, Order::Asc);
    reader
        .fetch_all(&select)
        .await?
        .iter()
        .map(|row| {
            let total: i64 = row.get(Computed::Total)?;
            Ok(OldCount {
                person: row.id(TimeEntryRow::Person)?,
                count: u64::try_from(total).map_err(|_| StoreError::Corrupt("a count"))?,
            })
        })
        .collect()
}

/// One step: deletes at most `limit` entries with a day before `before` with everything that tells
/// about them; how many of whose.
pub(crate) async fn delete(
    transaction: &mut Transaction,
    before: Date,
    person: Option<UserId>,
    limit: u64,
) -> Result<Vec<OldCount>, StoreError> {
    let (keys, counted) = choose(
        transaction,
        past(before, person)?,
        (TimeEntryRow::Id, TimeEntryRow::Person),
        limit,
    )
    .await?;
    if keys.is_empty() {
        return Ok(counted);
    }
    let mut chosen = Query::select();
    chosen
        .column(TimeEntryRow::Id)
        .from(TimeEntryRow::Table)
        .and_where(Expr::col(TimeEntryRow::Id).is_in(keys.clone()));
    let mut notifications = Query::delete();
    notifications
        .from_table(NotificationRow::Table)
        .and_where(Expr::col(NotificationRow::Record).is_in(keys.clone()));
    transaction.execute(&notifications).await?;
    delete_events(transaction, &events_about(RecordKind::TimeEntry, chosen)).await?;
    let mut values = Query::delete();
    values.from_table(TimeEntryValueRow::Table).and_where(
        Expr::col((TimeEntryValueRow::Table, FieldValueRow::EntityId)).is_in(keys.clone()),
    );
    transaction.execute(&values).await?;
    let mut entries = Query::delete();
    entries
        .from_table(TimeEntryRow::Table)
        .and_where(Expr::col(TimeEntryRow::Id).is_in(keys));
    transaction.execute(&entries).await?;
    Ok(counted)
}
