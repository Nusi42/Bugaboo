//! The store's part of *Keep and delete rules* (PRIV-05, ADR-0045, decision 5): every purge deletes
//! in steps of a bounded number of rows, so one step is one short write and a write of people never
//! waits long behind it (M4-F104). [`limited_rows`] is the one way a purge chooses its rows, and
//! [`delete_limited`] the one way it deletes them.

mod old_records;

use bb_core::event::RecordKind;
use sea_query::{
    Condition, DeleteStatement, Expr, ExprTrait as _, IntoIden, IntoTableRef, Order, Query,
    SelectStatement,
};

use crate::db::Transaction;
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

/// Deletes the events that `events` finds, with what was made from them: their entries in the
/// outbox and the rows of the history of an issue (ADR-0045, decision 4: a deleted record takes the
/// events that carry its values along). `events` is read again for each table, so it must find the
/// same events each time within the change.
pub(crate) async fn delete_events(
    transaction: &mut Transaction,
    events: &SelectStatement,
) -> Result<(), StoreError> {
    let mut outbox = Query::delete();
    outbox
        .from_table(Outbox::Table)
        .and_where(Expr::col(Outbox::EventId).in_subquery(events.clone()));
    transaction.execute(&outbox).await?;
    let mut history = Query::delete();
    history
        .from_table(HistoryEntryRow::Table)
        .and_where(Expr::col(HistoryEntryRow::Event).in_subquery(events.clone()));
    transaction.execute(&history).await?;
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
