//! The statements of a search: a page of issues, how many there are, and how many in each group.
//! All three start from the same condition, so the count line and the groups always agree with the
//! page.

use bb_app::search::{GroupCount, SearchHit, SearchPosition, SearchRequest, SortValue};
use sea_query::{
    Alias, Cond, Expr, Func, Iden, NullOrdering, Order, Query, UnionType, WithClause, WithQuery,
};

use super::order::{Grouping, Kind};
use super::{CompileError, Compiler};
use crate::db::{Dialect, Row, Statement};
use crate::error::StoreError;
use crate::issues::{issue_select, read_issue};
use crate::schema::IssueRow;

/// A statement, with the walks it starts with when the search follows sub-issues or relations.
pub(crate) enum Built {
    Plain(sea_query::SelectStatement),
    With(WithQuery),
}

impl<'a> From<&'a Built> for Statement<'a> {
    fn from(built: &'a Built) -> Self {
        match built {
            Built::Plain(select) => Statement::Select(select),
            Built::With(query) => Statement::With(query),
        }
    }
}

/// The name of the column that holds the nth sort value of a row.
#[derive(Clone, Copy)]
struct Sorted(usize);

impl Iden for Sorted {
    fn unquoted(&self) -> &'static str {
        ["s0", "s1", "s2", "s3", "s4", "s5", "s6", "s7"]
            .get(self.0)
            .copied()
            .unwrap_or("s7")
    }
}

/// The name of the column of a count.
#[derive(Clone, Copy)]
struct Counted;

impl Iden for Counted {
    fn unquoted(&self) -> &'static str {
        "issues"
    }
}

/// The condition every statement of a search has: the issues the person reads, that the search
/// finds.
fn found(compiler: &Compiler<'_>) -> Result<Cond, CompileError> {
    let mut cond = Cond::all().add(crate::tags::sight_condition(&compiler.request.sight));
    if let Some(filter) = &compiler.request.search.search().filter {
        cond = cond.add(compiler.filter(filter)?);
    }
    Ok(cond)
}

fn finish(compiler: &Compiler<'_>, select: sea_query::SelectStatement) -> Built {
    let tables = compiler.take_tables();
    if tables.is_empty() {
        return Built::Plain(select);
    }
    let mut clause = WithClause::new();
    clause.recursive(true);
    for table in tables {
        clause.cte(table);
    }
    Built::With(select.with(clause))
}

/// A page of the issues the search finds, in its order, after the last one of the page before.
pub(crate) fn page_statement(
    request: &SearchRequest,
    dialect: Dialect,
) -> Result<Built, CompileError> {
    let compiler = Compiler::new(request, dialect);
    let keys = compiler.keys()?;
    let mut cond = found(&compiler)?;
    if let Some(position) = &request.after {
        cond = cond.add(Compiler::after(&keys, position)?);
    }
    let mut select = issue_select();
    for (at, key) in keys.iter().enumerate() {
        select.expr_as(key.value.clone(), Sorted(at));
    }
    select.cond_where(cond).limit(request.limit);
    Compiler::order_by(&mut select, &keys);
    Ok(finish(&compiler, select))
}

/// How many issues the search finds.
pub(crate) fn count_statement(
    request: &SearchRequest,
    dialect: Dialect,
) -> Result<Built, CompileError> {
    let compiler = Compiler::new(request, dialect);
    let mut select = Query::select();
    select
        .expr_as(
            Func::count(Expr::col((IssueRow::Table, IssueRow::Id))),
            Counted,
        )
        .from(IssueRow::Table)
        .cond_where(found(&compiler)?);
    Ok(finish(&compiler, select))
}

/// How many issues the search finds for each value of the field, the value in the column `s0`.
/// An issue with several values counts in the group of each; one with none in the group without a
/// value (the last).
pub(crate) fn groups_statement(
    request: &SearchRequest,
    dialect: Dialect,
    by: &str,
) -> Result<(Built, Kind), CompileError> {
    let compiler = Compiler::new(request, dialect);
    let grouping = compiler.grouping(by)?;
    let cond = found(&compiler)?;
    let issue = || Expr::col((IssueRow::Table, IssueRow::Id));
    match grouping {
        Grouping::One(value, kind) => {
            let mut select = Query::select();
            select
                .expr_as(value, Sorted(0))
                .expr_as(Func::count(issue()), Counted)
                .from(IssueRow::Table)
                .cond_where(cond)
                .group_by_col(Sorted(0));
            select.order_by_expr_with_nulls(Expr::col(Sorted(0)), Order::Asc, NullOrdering::Last);
            Ok((finish(&compiler, select), kind))
        }
        Grouping::Many {
            mut rows,
            value,
            kind,
            without,
        } => {
            // The rows of the values of an issue, joined to the issue by their own condition.
            rows.expr_as(value, Sorted(0))
                .expr_as(Func::count_distinct(issue()), Counted)
                .from(IssueRow::Table)
                .cond_where(cond.clone())
                .group_by_col(Sorted(0));
            let nothing_of_kind = match kind {
                Kind::Text => "TEXT",
                Kind::Whole => "BIGINT",
            };
            let mut none = Query::select();
            none.expr_as(
                Func::cast_as(Expr::null(), Alias::new(nothing_of_kind)),
                Sorted(0),
            )
            .expr_as(Func::count(issue()), Counted)
            .from(IssueRow::Table)
            .cond_where(cond.add(without));
            rows.union(UnionType::All, none);
            Ok((finish(&compiler, rows), kind))
        }
    }
}

/// The issue and where it stands, from a row of a page.
pub(crate) fn read_hit(row: &Row, kinds: &[Kind]) -> Result<SearchHit, StoreError> {
    let issue = read_issue(row)?;
    let mut values = Vec::with_capacity(kinds.len());
    for (at, kind) in kinds.iter().enumerate() {
        values.push(match kind {
            Kind::Text => row
                .get::<Option<String>>(Sorted(at))?
                .map_or(SortValue::Empty, SortValue::Text),
            Kind::Whole => row
                .get::<Option<i64>>(Sorted(at))?
                .map_or(SortValue::Empty, SortValue::Whole),
        });
    }
    let id = issue.id;
    Ok(SearchHit {
        issue,
        position: SearchPosition { values, id },
    })
}

/// The number of a count statement.
pub(crate) fn read_count(row: &Row) -> Result<u64, StoreError> {
    u64::try_from(row.get::<i64>(Counted)?).map_err(|_| StoreError::Corrupt("a count"))
}

/// A group and its count from a row of the groups statement.
pub(crate) fn read_group(row: &Row, kind: Kind) -> Result<GroupCount, StoreError> {
    let value = match kind {
        Kind::Text => row.get::<Option<String>>(Sorted(0))?,
        Kind::Whole => row
            .get::<Option<i64>>(Sorted(0))?
            .map(|number| number.to_string()),
    };
    Ok(GroupCount {
        value,
        issues: read_count(row)?,
    })
}

/// The kinds of the keys of the order of this search, which tell how to read the sort values.
pub(crate) fn kinds_of(
    request: &SearchRequest,
    dialect: Dialect,
) -> Result<Vec<Kind>, CompileError> {
    Ok(Compiler::new(request, dialect)
        .keys()?
        .iter()
        .map(|key| key.kind)
        .collect())
}
