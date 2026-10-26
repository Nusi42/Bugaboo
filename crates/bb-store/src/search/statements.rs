//! The statements of a search: a page of issues, how many there are, how many in each group, and the
//! place of an issue in the order (to continue after it). All of them start from the same sight, so
//! the count line and the groups always agree with the page.

use bb_app::search::{GroupCount, GroupKey, SearchHit, SearchRequest};
use bb_core::id::{ChoiceId, GroupId, IssueId, IssueTypeId, ProjectId, UserId};
use bb_core::issue::{Priority, StatusGroup};
use bb_searchlang::MAX_SORT_KEYS;
use sea_query::{
    Alias, Cond, Expr, ExprTrait as _, Func, Iden, NullOrdering, Order, Query, UnionType,
    WithClause, WithQuery,
};

use super::order::{GroupValue, Grouping, IdOf, Key, Kind, SortValue};
use super::{CompileError, Compiler};
use crate::db::{Dialect, Row, Statement, StoredId as _};
use crate::error::StoreError;
use crate::issues::{issue_select, read_issue};
use crate::schema::IssueRow;
use crate::sight::sight_condition;

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

/// The names of the columns that hold the values of the keys of the order, as many as a search may
/// have keys.
const SORT_COLUMNS: [&str; MAX_SORT_KEYS] = ["s0", "s1", "s2", "s3", "s4"];

/// The name of the column that holds the nth sort value of a row.
#[derive(Clone, Copy)]
struct Sorted(usize);

impl Iden for Sorted {
    fn unquoted(&self) -> &'static str {
        SORT_COLUMNS.get(self.0).copied().unwrap_or(SORT_COLUMNS[0])
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
    let mut cond = Cond::all().add(sight_condition(&compiler.request.sight));
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

/// The order of a search: its keys, compiled once, and what each holds.
pub(crate) struct SortPlan {
    keys: Vec<Key>,
    kinds: Vec<Kind>,
}

impl SortPlan {
    /// The keys of the order of the request.
    pub(crate) fn of(request: &SearchRequest, dialect: Dialect) -> Result<Self, CompileError> {
        let keys = Compiler::new(request, dialect).keys()?;
        let kinds = keys.iter().map(|key| key.kind).collect();
        Ok(Self { keys, kinds })
    }

    /// The kinds of the keys, which tell how to read the values of a row.
    pub(crate) fn kinds(&self) -> &[Kind] {
        &self.kinds
    }
}

/// The values of the keys of the order for one issue the person reads, to continue after it. No
/// row means an issue that is not there or not readable: not a cursor of this list.
pub(crate) fn place_statement(request: &SearchRequest, plan: &SortPlan, after: IssueId) -> Built {
    let mut select = Query::select();
    for (at, key) in plan.keys.iter().enumerate() {
        select.expr_as(key.value.clone(), Sorted(at));
    }
    select
        .from(IssueRow::Table)
        .cond_where(sight_condition(&request.sight))
        .and_where(Expr::col((IssueRow::Table, IssueRow::Id)).eq(after.stored()))
        .limit(1);
    Built::Plain(select)
}

/// A page of the issues the search finds, in its order, after the issue whose place is `last`.
pub(crate) fn page_statement(
    request: &SearchRequest,
    dialect: Dialect,
    plan: &SortPlan,
    last: Option<(&[SortValue], IssueId)>,
) -> Result<Built, CompileError> {
    let compiler = Compiler::new(request, dialect);
    let mut cond = found(&compiler)?;
    if let Some((values, id)) = last {
        cond = cond.add(Compiler::after(&plan.keys, values, id));
    }
    let mut select = issue_select();
    select.cond_where(cond).limit(request.limit);
    Compiler::order_by(&mut select, &plan.keys);
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
) -> Result<(Built, GroupValue), CompileError> {
    let compiler = Compiler::new(request, dialect);
    let grouping = compiler.grouping(by)?;
    let cond = found(&compiler)?;
    let issue = || Expr::col((IssueRow::Table, IssueRow::Id));
    match grouping {
        Grouping::One(value, what) => {
            let mut select = Query::select();
            select
                .expr_as(value, Sorted(0))
                .expr_as(Func::count(issue()), Counted)
                .from(IssueRow::Table)
                .cond_where(cond)
                .group_by_col(Sorted(0));
            select.order_by_expr_with_nulls(Expr::col(Sorted(0)), Order::Asc, NullOrdering::Last);
            Ok((finish(&compiler, select), what))
        }
        Grouping::Many {
            mut rows,
            value,
            what,
            without,
        } => {
            // The rows of the values of an issue, joined to the issue by their own condition.
            rows.expr_as(value, Sorted(0))
                .expr_as(Func::count_distinct(issue()), Counted)
                .from(IssueRow::Table)
                .cond_where(cond.clone())
                .group_by_col(Sorted(0));
            let empty = match (what, dialect) {
                (GroupValue::Text, _) => "TEXT",
                (GroupValue::Id(_), Dialect::Sqlite) => "BLOB",
                (GroupValue::Id(_), Dialect::Postgres) => "BYTEA",
                (GroupValue::Whole | GroupValue::Priority | GroupValue::StatusGroup, _) => "BIGINT",
            };
            let mut none = Query::select();
            none.expr_as(Func::cast_as(Expr::null(), Alias::new(empty)), Sorted(0))
                .expr_as(Func::count(issue()), Counted)
                .from(IssueRow::Table)
                .cond_where(cond.add(without));
            rows.union(UnionType::All, none);
            Ok((finish(&compiler, *rows), what))
        }
    }
}

/// The issue of a row of a page.
pub(crate) fn read_hit(row: &Row) -> Result<SearchHit, StoreError> {
    Ok(SearchHit {
        issue: read_issue(row)?,
    })
}

/// The values of the keys of the order in a row of the place of an issue.
pub(crate) fn read_place(row: &Row, kinds: &[Kind]) -> Result<Vec<SortValue>, StoreError> {
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
    Ok(values)
}

/// The number of a count statement.
pub(crate) fn read_count(row: &Row) -> Result<u64, StoreError> {
    u64::try_from(row.get::<i64>(Counted)?).map_err(|_| StoreError::Corrupt("a count"))
}

/// The key of a group from the value of its row.
fn read_key(row: &Row, what: GroupValue) -> Result<GroupKey, StoreError> {
    let corrupt = || StoreError::Corrupt("a group");
    Ok(match what {
        GroupValue::Text => row
            .get::<Option<String>>(Sorted(0))?
            .map_or(GroupKey::Empty, GroupKey::Text),
        GroupValue::Whole => row
            .get::<Option<i64>>(Sorted(0))?
            .map_or(GroupKey::Empty, GroupKey::Whole),
        GroupValue::Priority => match row.get::<Option<i64>>(Sorted(0))? {
            None => GroupKey::Empty,
            Some(place) => Priority::ALL
                .get(usize::try_from(place).map_err(|_| corrupt())?)
                .map(|each| GroupKey::Priority(*each))
                .ok_or_else(corrupt)?,
        },
        GroupValue::StatusGroup => match row.get::<Option<i64>>(Sorted(0))? {
            None => GroupKey::Empty,
            Some(place) => StatusGroup::ALL
                .get(usize::try_from(place).map_err(|_| corrupt())?)
                .map(|each| GroupKey::StatusGroup(*each))
                .ok_or_else(corrupt)?,
        },
        GroupValue::Id(of) => match row.get::<Option<Vec<u8>>>(Sorted(0))? {
            None => GroupKey::Empty,
            Some(bytes) => match of {
                IdOf::Person => GroupKey::Person(UserId::read(bytes)?),
                IdOf::Group => GroupKey::Group(GroupId::read(bytes)?),
                IdOf::Issue => GroupKey::Issue(IssueId::read(bytes)?),
                IdOf::Project => GroupKey::Project(ProjectId::read(bytes)?),
                IdOf::IssueType => GroupKey::IssueType(IssueTypeId::read(bytes)?),
                IdOf::Choice => GroupKey::Choice(ChoiceId::read(bytes)?),
            },
        },
    })
}

/// A group and its count from a row of the groups statement.
pub(crate) fn read_group(row: &Row, what: GroupValue) -> Result<GroupCount, StoreError> {
    Ok(GroupCount {
        key: read_key(row, what)?,
        issues: read_count(row)?,
    })
}
