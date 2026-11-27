//! The arms of the walk (M9-F144): the rows of a table that name a person, as conditions that one
//! index each answers. A table holds a person in several ways (an author, an actor, a reference to
//! a record of one of many kinds, an ID inside its changes), and one condition that ORs them reads
//! the whole table on both databases as soon as one of them has no index of its own. So each way
//! is an arm of its own, an arm leaves out the rows of the arms before it, and the arms are put
//! together with `UNION ALL`: the database answers each from its index, and counts or pages the
//! rows once.

use sea_query::{Alias, Cond, Expr, ExprTrait as _, Order, Query, SelectStatement, UnionType};

use super::link_parts;
use crate::db::Dialect;
use crate::error::StoreError;
use crate::schema::{Computed, StoredTable};
use bb_core::id::UserId;
use bb_core::personal::{LinkScope, PersonLink};

/// The name of the rows of the arms, as one table.
const FOUND: &str = "found";

/// The arms of the rows of `table` that name `person` by one of the links that `wanted` keeps: the
/// parts of each link (`link_parts`), each without the rows of the links before it.
///
/// # Errors
/// The error of a link that cannot be turned into a condition.
pub(super) fn arms(
    table: &StoredTable,
    all: &[StoredTable],
    person: UserId,
    dialect: Dialect,
    wanted: impl Fn(&PersonLink) -> bool,
) -> Result<Vec<Cond>, StoreError> {
    let mut arms = Vec::new();
    let mut before: Vec<Cond> = Vec::new();
    // A reference to a record is asked of every kind, each with a subquery of its own: it comes
    // last, so no other arm carries all of them in what it leaves out, and the plan of every arm
    // stays as small as its own condition.
    let mut links: Vec<&PersonLink> = table.marks.person_links().links().iter().collect();
    links.sort_by_key(|link| matches!(link.scope(), LinkScope::Record { .. }));
    for link in links {
        if !wanted(link) {
            continue;
        }
        let parts = link_parts(table, all, link, person, dialect)?;
        for part in &parts {
            let arm = before
                .iter()
                .fold(Cond::all().add(part.clone()), |arm, earlier| {
                    arm.add(not_true(earlier.clone()))
                });
            arms.push(arm);
        }
        before.push(parts.into_iter().fold(Cond::any(), Cond::add));
    }
    Ok(arms)
}

/// `condition` is not true: false, or unknown because a column is empty. A row whose column of an
/// earlier link is empty is not that link's row, so it stays in the arm of its own link. Written as
/// a condition of its own, not as a comparison: PostgreSQL then takes it to keep half of the rows,
/// not one in two hundred, and still looks the rows of a subquery up by its index instead of
/// reading the table (the values of the time entries of a person with a thousand of them: 130 ms
/// read whole, 3 ms looked up).
pub(super) fn not_true(condition: Cond) -> Expr {
    Expr::case(condition, Expr::val(false))
        .finally(Expr::val(true))
        .into()
}

/// `select` for each arm, the arms put together with `UNION ALL`.
fn joined(arms: Vec<Cond>, select: impl Fn(Cond) -> SelectStatement) -> Option<SelectStatement> {
    let mut arms = arms.into_iter();
    let mut joined = select(arms.next()?);
    for arm in arms {
        joined.union(UnionType::All, select(arm));
    }
    Some(joined)
}

/// The statement that counts the rows of `arms` of `table` (as [`Computed::Total`]).
pub(super) fn counted(table: &StoredTable, arms: Vec<Cond>) -> SelectStatement {
    let name = Alias::new(table.name.as_str());
    let mut count = Query::select();
    count.expr_as(Expr::cust("COUNT(*)"), Computed::Total);
    match joined(arms, |arm| {
        let mut one = Query::select();
        one.expr(Expr::val(1_i64))
            .from(name.clone())
            .cond_where(arm);
        one
    }) {
        Some(found) => {
            count.from_subquery(found, Alias::new(FOUND));
        }
        // No arm: no row. A count of an empty table says so on both databases.
        None => {
            count
                .from(name)
                .and_where(Expr::val(1_i64).eq(Expr::val(0_i64)));
        }
    }
    count
}

/// The statement that reads the rows of `arms` of `table`, every column and the `flags` after them,
/// in a fixed order (every column, from the first), so the pages neither repeat nor skip a row.
pub(super) fn rows(
    table: &StoredTable,
    arms: Vec<Cond>,
    flags: &[(Expr, Alias)],
) -> SelectStatement {
    let name = Alias::new(table.name.as_str());
    let columns: Vec<Alias> = table
        .columns
        .iter()
        .map(|(column, _)| Alias::new(column.as_str()))
        .collect();
    let arm_rows = |arm: Cond| {
        let mut one = Query::select();
        one.columns(columns.clone())
            .from(name.clone())
            .cond_where(arm);
        for (flag, alias) in flags {
            one.expr_as(flag.clone(), alias.clone());
        }
        one
    };
    let mut rows = Query::select();
    rows.columns(columns.clone());
    for (_, alias) in flags {
        rows.column(alias.clone());
    }
    match joined(arms, arm_rows) {
        Some(found) => {
            rows.from_subquery(found, Alias::new(FOUND));
        }
        None => {
            rows.from(name)
                .and_where(Expr::val(1_i64).eq(Expr::val(0_i64)));
        }
    }
    for column in columns {
        rows.order_by(column, Order::Asc);
    }
    rows
}
