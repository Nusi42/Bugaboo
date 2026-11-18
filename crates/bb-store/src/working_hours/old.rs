//! The work days older than the keep period (ADR-0035, decision 7): counted when asked, and deleted
//! with everything that tells about them, in one change: their periods and breaks, the events of
//! their changes (with the old values and the reasons) with their entries in the outbox, and the
//! notifications about them.

use bb_app::keeping::OldCount;
use bb_core::event::RecordKind;
use bb_core::id::UserId;
use bb_core::notification::NotificationKind;
use jiff::civil::Date;
use sea_query::{Expr, ExprTrait as _, Order, Query, SelectStatement};

use crate::db::{Reader, StoredId, Transaction};
use crate::error::StoreError;
use crate::keeping::old_records::choose;
use crate::keeping::{delete_events, events_about};
use crate::schema::{Computed, NotificationRow, WorkBreakRow, WorkDayRow, WorkPeriodRow};

/// The days older than `before`, of one person or of everybody (no column chosen yet).
fn past(before: Date, person: Option<UserId>) -> SelectStatement {
    let mut select = Query::select();
    select
        .from(WorkDayRow::Table)
        .and_where(Expr::col(WorkDayRow::Date).lt(before.to_string()));
    if let Some(person) = person {
        select.and_where(Expr::col(WorkDayRow::Person).eq(person.stored()));
    }
    select
}

/// How many days older than `before` each person has, by person.
pub(crate) async fn count(
    reader: &mut impl Reader,
    before: Date,
    person: Option<UserId>,
) -> Result<Vec<OldCount>, StoreError> {
    let mut select = past(before, person);
    select
        .column(WorkDayRow::Person)
        .expr_as(Expr::col(WorkDayRow::Id).count(), Computed::Total)
        .group_by_col(WorkDayRow::Person)
        .order_by(WorkDayRow::Person, Order::Asc);
    reader
        .fetch_all(&select)
        .await?
        .iter()
        .map(|row| {
            let total: i64 = row.get(Computed::Total)?;
            Ok(OldCount {
                person: row.id(WorkDayRow::Person)?,
                count: u64::try_from(total).map_err(|_| StoreError::Corrupt("a count"))?,
            })
        })
        .collect()
}

/// One step: deletes at most `limit` days older than `before` with everything that tells about
/// them; how many of whose.
pub(crate) async fn delete(
    transaction: &mut Transaction,
    before: Date,
    person: Option<UserId>,
    limit: u64,
) -> Result<Vec<OldCount>, StoreError> {
    let (keys, counted) = choose(
        transaction,
        past(before, person),
        (WorkDayRow::Id, WorkDayRow::Person),
        limit,
    )
    .await?;
    if keys.is_empty() {
        return Ok(counted);
    }
    let mut chosen = Query::select();
    chosen
        .column(WorkDayRow::Id)
        .from(WorkDayRow::Table)
        .and_where(Expr::col(WorkDayRow::Id).is_in(keys.clone()));
    let mut notifications = Query::delete();
    notifications
        .from_table(NotificationRow::Table)
        .and_where(Expr::col(NotificationRow::Kind).eq(NotificationKind::WorkDayChanged.code()))
        .and_where(Expr::col(NotificationRow::Record).is_in(keys.clone()));
    transaction.execute(&notifications).await?;
    delete_events(transaction, &events_about(RecordKind::WorkDay, chosen)).await?;
    let mut periods = Query::delete();
    periods
        .from_table(WorkPeriodRow::Table)
        .and_where(Expr::col(WorkPeriodRow::Day).is_in(keys.clone()));
    transaction.execute(&periods).await?;
    let mut breaks = Query::delete();
    breaks
        .from_table(WorkBreakRow::Table)
        .and_where(Expr::col(WorkBreakRow::Day).is_in(keys.clone()));
    transaction.execute(&breaks).await?;
    let mut days = Query::delete();
    days.from_table(WorkDayRow::Table)
        .and_where(Expr::col(WorkDayRow::Id).is_in(keys));
    transaction.execute(&days).await?;
    Ok(counted)
}
