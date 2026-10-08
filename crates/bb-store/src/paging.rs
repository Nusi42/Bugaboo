//! Keyset paging: a page is read from the position the page before ended at, not by counting
//! rows, so a list that changes while somebody pages through it neither repeats nor skips.

use sea_query::{Cond, Expr, ExprTrait as _, Iden, Order};

/// What comes after the last row of the page before, in a list ordered by `primary` (as `order`
/// says) and then by `id` ascending: `primary` is further on than `last`, or equal to it with a
/// greater ID. `primary` and `last` are made on demand because each is used twice.
pub(crate) fn after_last(
    primary: impl Fn() -> Expr,
    last: impl Fn() -> Expr,
    order: &Order,
    id: impl Iden + 'static,
    last_id: Vec<u8>,
) -> Cond {
    let further = if matches!(order, Order::Desc) {
        primary().lt(last())
    } else {
        primary().gt(last())
    };
    Cond::any().add(further).add(
        Cond::all()
            .add(primary().eq(last()))
            .add(Expr::col(id).gt(last_id)),
    )
}
