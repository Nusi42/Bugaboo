//! The store's part of *Keep and delete rules* (PRIV-05, ADR-0045, decision 5): every purge deletes
//! in steps of a bounded number of rows, so one step is one short write and a write of people never
//! waits long behind it (M4-F104). [`limited_rows`] is the one way a purge chooses its rows, and
//! [`delete_limited`] the one way it deletes them.

mod old_records;
pub(crate) mod trash;

use bb_core::event::RecordKind;
use sea_query::{
    Condition, DeleteStatement, Expr, ExprTrait as _, Iden, IntoIden, IntoTableRef, Order, Query,
    SelectStatement,
};

use jiff::Timestamp;

use crate::db::{Transaction, to_micros};
use crate::error::StoreError;
use crate::schema::{Event, HistoryEntryRow, Outbox};

/// The condition that holds for at most `limit` rows of `table` that match `condition`, chosen by
/// their key columns `keys` in order. The rows are chosen in a subquery, so the limit holds on both
/// databases: PostgreSQL has no `DELETE … LIMIT` or `UPDATE … LIMIT`.
pub(crate) fn limited_rows<T, K>(table: T, keys: &[K], condition: Condition, limit: u64) -> Expr
where
    T: IntoTableRef,
    K: IntoIden + Copy,
{
    let mut chosen = Query::select();
    chosen.from(table).cond_where(condition).limit(limit);
    for key in keys {
        chosen.column(*key).order_by(*key, Order::Asc);
    }
    let key = match keys {
        [one] => Expr::col(*one),
        many => Expr::tuple(many.iter().map(|key| Expr::col(*key))),
    };
    key.in_subquery(chosen)
}

/// A delete of at most `limit` rows of `table` that match `condition` ([`limited_rows`]).
pub(crate) fn delete_limited<T, K>(
    table: T,
    keys: &[K],
    condition: Condition,
    limit: u64,
) -> DeleteStatement
where
    T: IntoTableRef + Clone,
    K: IntoIden + Copy,
{
    let mut delete = Query::delete();
    delete
        .from_table(table.clone())
        .and_where(limited_rows(table, keys, condition, limit));
    delete
}

/// Chooses at most `limit` rows of `table` that went into the trash before `before` and match
/// `also`, and holds them for the change, in one statement; their keys `key`, in order. The outer
/// statement asks again that each row is in the trash: on PostgreSQL a row that somebody took out of
/// the trash after the choice, and before the hold got its lock, is checked again as it is now and
/// left out; SQLite has one writer at a time. A row taken out of the trash after the hold waits
/// until the purge is done and finds it gone.
pub(crate) async fn hold_trashed<T, C>(
    transaction: &mut Transaction,
    table: T,
    (key, deleted_at): (C, C),
    also: Condition,
    before: Timestamp,
    limit: u64,
) -> Result<Vec<Vec<u8>>, StoreError>
where
    T: IntoTableRef + Clone,
    C: Iden + Copy + 'static,
{
    let in_the_trash = || {
        Condition::all()
            .add(Expr::col(deleted_at).is_not_null())
            .add(Expr::col(deleted_at).lt(to_micros(before)))
    };
    let mut chosen = Query::select();
    chosen
        .column(key)
        .from(table.clone())
        .cond_where(in_the_trash().add(also))
        .order_by(key, Order::Asc)
        .limit(limit);
    let mut held = Query::update();
    held.table(table)
        .value(deleted_at, Expr::col(deleted_at))
        .cond_where(in_the_trash().add(Expr::col(key).in_subquery(chosen)))
        .returning_col(key);
    transaction
        .fetch_all(&held)
        .await?
        .iter()
        .map(|row| row.get(key))
        .collect()
}

/// Deletes the events that `events` finds, with what was made from them: their entries in the
/// outbox and the rows of the history of an issue (ADR-0045, decision 4: a deleted record takes the
/// events that carry its values along). `events` is read again for each table, so it must find the
/// same events each time within the change.
pub(crate) async fn delete_events(
    transaction: &mut Transaction,
    events: &SelectStatement,
) -> Result<(), StoreError> {
    let mut history = Query::delete();
    history
        .from_table(HistoryEntryRow::Table)
        .and_where(Expr::col(HistoryEntryRow::Event).in_subquery(events.clone()));
    transaction.execute(&history).await?;
    forget_events(transaction, events).await
}

/// Deletes the events that `events` finds with their entries in the outbox, and keeps the rows of
/// the history made from them: for a record whose issue is still there, whose history rows hold
/// none of the record's values (a file deleted for good, M4-F93). The same rule for `events` as
/// [`delete_events`].
pub(crate) async fn forget_events(
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

/// The IDs of the events about the records of `kind` that `records` finds.
pub(crate) fn events_about(kind: RecordKind, records: SelectStatement) -> SelectStatement {
    let mut select = Query::select();
    select
        .column(Event::Id)
        .from(Event::Table)
        .and_where(Expr::col(Event::EntityKind).eq(kind.code()))
        .and_where(Expr::col(Event::EntityId).in_subquery(records));
    select
}
