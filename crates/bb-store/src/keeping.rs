//! The store's part of *Keep and delete rules* (PRIV-05, ADR-0045, decision 5): every purge deletes
//! in steps of a bounded number of rows, so one step is one short write and a write of people never
//! waits long behind it (M4-F104). [`limited_rows`] is the one way a purge chooses its rows, and
//! [`delete_limited`] the one way it deletes them.

use sea_query::{
    Condition, DeleteStatement, Expr, ExprTrait as _, IntoIden, IntoTableRef, Order, Query,
};

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
