//! The first cards of every cell of a board (PLAN-06, PERF-N1), in one statement whatever the
//! number of columns, rows and cards. The counts of the cells are the totals of the search,
//! grouped by the rows and then by the field of the columns (`totals.rs`).
//!
//! The column of an issue is the place of its value of the field of the columns among the columns
//! (one `CASE`, so values of the same name in several workflows are one column); its row is the
//! group of the totals by the field of the rows (`Compiler::split`, people shown alike one row).
//! The cards of a cell are numbered with `ROW_NUMBER() OVER (PARTITION BY row, column ORDER BY` the
//! order of the search`)`, which SQLite (3.25 and later) and PostgreSQL both have. The statement
//! starts from the same sight and filter as a page of the search.

use bb_app::search::{CellAt, CellHit, CellWindow, Cells, GroupKey, SearchRequest};
use sea_query::{
    Alias, CaseStatement, Cond, Expr, ExprTrait as _, OverStatement as _, Query, WindowStatement,
};

use super::order::GroupValue;
use super::statements::{Built, GROUP_COLUMN, Sorted, finish, found, key_value, read_key};
use super::{CompileError, Compiler};
use crate::db::{Dialect, Row};
use crate::error::StoreError;
use crate::issues::{issue_select, read_issue};
use crate::schema::IssueRow;

/// The column of the place of the column of an issue.
const COLUMN_PLACE: Sorted = Sorted(1);
/// The column of the number of an issue in its cell.
const NUMBER: &str = "cell_number";
/// The name of the table of the numbered issues.
const NUMBERED: &str = "cells";
/// The name of the table of the cells of each issue, and of its column of the issue.
const PLACED: &str = "placed";
const PLACED_ISSUE: &str = "placed_issue";

/// `value` is the value of `key`.
fn is_key(value: &Expr, key: &GroupKey) -> Cond {
    Cond::all().add(match key_value(key) {
        Some(stored) => value.clone().eq(Expr::val(stored)),
        None => value.clone().is_null(),
    })
}

/// The place of the column of an issue, by its value of the field of the columns; `NULL` for a
/// value that is in no column.
fn column_place(value: &Expr, column_of: &[(GroupKey, usize)]) -> Expr {
    let mut case: Option<CaseStatement> = None;
    for (key, at) in column_of {
        let then = Expr::val(i64::try_from(*at).unwrap_or(i64::MAX));
        let when = is_key(value, key);
        case = Some(match case {
            None => Expr::case(when, then),
            Some(case) => case.case(when, then),
        });
    }
    case.map_or_else(Expr::null, |case| case.finally(Expr::null()).into())
}

/// The issues of the cells in `window`, each with its cell and in the order of the search within
/// it, and what the value of a row is.
pub(super) fn cell_pages_statement(
    request: &SearchRequest,
    dialect: Dialect,
    cells: &Cells,
    window: &CellWindow,
) -> Result<(Built, Option<GroupValue>), CompileError> {
    let compiler = Compiler::new(request, dialect);
    let keys = compiler.keys()?;
    let column = column_place(&compiler.one_grouping(&cells.columns)?.0, &cells.column_of);
    let split = cells
        .rows
        .as_ref()
        .map(|rows| compiler.split(rows))
        .transpose()?;
    // Each issue once in each of its cells: people shown alike are one row, so an issue of two of
    // them is one card there, as it is one issue in the count of the cell.
    let mut placed = Query::select();
    placed
        .distinct()
        .expr_as(
            Expr::col((IssueRow::Table, IssueRow::Id)),
            Alias::new(PLACED_ISSUE),
        )
        .expr_as(column.clone(), COLUMN_PLACE)
        .from(IssueRow::Table);
    let mut chosen = found(&compiler)?.add(column.clone().is_not_null());
    if let Some(split) = &split {
        if let Some(rows) = &split.rows {
            rows.join_to(&mut placed);
        }
        placed.expr_as(split.value.clone(), GROUP_COLUMN);
        // Only the rows the board shows: the cells of the others are not read.
        chosen = chosen.add(
            window
                .rows
                .iter()
                .fold(Cond::any(), |any, key| any.add(is_key(&split.value, key))),
        );
        if let Some(CellAt { row: Some(key), .. }) = &window.only {
            chosen = chosen.add(is_key(&split.value, key));
        }
    }
    if let Some(only) = &window.only {
        chosen = chosen.add(
            column
                .clone()
                .eq(i64::try_from(only.column).unwrap_or(i64::MAX)),
        );
    }
    placed.cond_where(chosen);
    // The cards of each cell numbered in the order of the search, the issue joined again for the
    // keys of the order and for the card.
    let of_placed = |name: Sorted| Expr::col((Alias::new(PLACED), name));
    let mut numbering = WindowStatement::new();
    if split.is_some() {
        numbering.add_partition_by(of_placed(GROUP_COLUMN));
    }
    numbering.add_partition_by(of_placed(COLUMN_PLACE));
    Compiler::order_by(&mut numbering, &keys);
    let mut inner = issue_select();
    inner
        .from_clear()
        .from_subquery(placed, Alias::new(PLACED))
        .inner_join(
            IssueRow::Table,
            Expr::col((IssueRow::Table, IssueRow::Id))
                .equals((Alias::new(PLACED), Alias::new(PLACED_ISSUE))),
        )
        .expr_as(of_placed(COLUMN_PLACE), COLUMN_PLACE);
    if split.is_some() {
        inner.expr_as(of_placed(GROUP_COLUMN), GROUP_COLUMN);
    }
    inner.expr_window_as(Expr::cust("ROW_NUMBER()"), numbering, Alias::new(NUMBER));
    let first = i64::try_from(window.skip).unwrap_or(i64::MAX);
    let last = i64::try_from(window.skip.saturating_add(window.take)).unwrap_or(i64::MAX);
    let number = || Expr::col((Alias::new(NUMBERED), Alias::new(NUMBER)));
    let mut select = Query::select();
    select
        .expr(Expr::col(sea_query::Asterisk))
        .from_subquery(inner, Alias::new(NUMBERED))
        .and_where(number().gt(first))
        .and_where(number().lte(last))
        .order_by_expr(number(), sea_query::Order::Asc);
    let rows = split.map(|split| split.what());
    Ok((finish(&compiler, select), rows))
}

/// An issue and its cell from a row of the statement; none for a row of no row (a person in no
/// set of people).
pub(super) fn read_cell_hit(
    row: &Row,
    rows: Option<GroupValue>,
) -> Result<Option<CellHit>, StoreError> {
    let column = usize::try_from(row.get::<i64>(COLUMN_PLACE)?)
        .map_err(|_| StoreError::Corrupt("a column of a board"))?;
    let row_key = match rows {
        None => None,
        Some(what) => match read_key(row, what)? {
            Some(key) => Some(key),
            None => return Ok(None),
        },
    };
    Ok(Some(CellHit {
        at: CellAt {
            column,
            row: row_key,
        },
        issue: read_issue(row)?,
    }))
}
