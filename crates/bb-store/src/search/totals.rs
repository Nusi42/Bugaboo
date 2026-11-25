//! The totals of a search (QRY-08, PRIV-13, ADR-0050 decision 7): the count of the issues the
//! search finds and the sums asked for, for each group of one field and for all of them together.
//!
//! A total starts from the same sight and filter as the page, so it never holds an issue the reader
//! cannot see. Each value of a sum is taken only where the use case says the reader may read the
//! field (the projects, and there the issue types): elsewhere it is `NULL`, which no sum adds and
//! no count of values counts. The rows of the issues are read once, in an inner statement that
//! works out the group and each value of an issue; the outer one adds them up. An issue with
//! several values of the grouped field stands in the group of each, and in the total of all of them
//! once. A page of groups is cut in the statement where the order of the values is the order of
//! their places (`GroupKey::place`).

use bb_app::search::{
    FieldPlace, FieldTarget, GroupBy, GroupKey, GroupPage, GroupPlace, GroupTotal, ReadableWhere,
    SUMS_MAX, SearchRequest, Sum, SumAsked, Total, Totals, TotalsAsked,
};
use bb_core::id::UserId;
use bb_core::own_fields::FieldType;
use sea_query::{
    Alias, CaseStatement, Cond, Expr, ExprTrait as _, Func, Iden, NullOrdering, Order, Query,
    SelectStatement,
};

use super::order::{GroupValue, Grouping, bigint};
use super::source::{Repeats, Rows};
use super::statements::{Built, GROUP_COLUMN, Sorted, finish, found, read_key, read_key_at};
use super::{CompileError, Compiler, nothing};
use crate::db::{Dialect, Row, StoredId as _};
use crate::error::StoreError;
use crate::schema::IssueRow;

/// The column of the value of the second field of a group ([`GroupBy::then`]).
const THEN_COLUMN: Sorted = Sorted(1);

/// The name of a column of the statements of the totals.
#[derive(Clone, Copy)]
struct Name(&'static str);

impl Iden for Name {
    fn unquoted(&self) -> &'static str {
        self.0
    }
}

/// The ID of the issue of a row of the inner statement.
const ISSUE: Name = Name("issue");
/// How many issues.
const ISSUES: Name = Name("issues");

/// The columns of one sum: its value on an issue, the sum, and how many issues gave a value.
#[derive(Clone, Copy)]
struct SumColumns {
    value: Name,
    sum: Name,
    valued: Name,
}

const fn sum_columns(value: &'static str, sum: &'static str, valued: &'static str) -> SumColumns {
    SumColumns {
        value: Name(value),
        sum: Name(sum),
        valued: Name(valued),
    }
}

/// The columns of each sum a total may ask for, as many as `SUMS_MAX`: the sums are paired with
/// them, so a sum past the limit has no column to be read from.
const SUM_COLUMNS: [SumColumns; SUMS_MAX] = [
    sum_columns("v0", "t0", "n0"),
    sum_columns("v1", "t1", "n1"),
    sum_columns("v2", "t2", "n2"),
    sum_columns("v3", "t3", "n3"),
    sum_columns("v4", "t4", "n4"),
    sum_columns("v5", "t5", "n5"),
    sum_columns("v6", "t6", "n6"),
    sum_columns("v7", "t7", "n7"),
];

/// The name of the inner statement in the outer one.
const FOUND: &str = "found";

/// The statements of the totals: of the groups (with what their keys are) and of all the issues.
pub(crate) struct TotalsPlan {
    pub(crate) groups: Option<(Built, GroupValue, Option<GroupValue>)>,
    pub(crate) all: Option<Built>,
    /// Whether the total of all is the groups added up instead of read by its statement.
    pub(crate) from_groups: FromGroups,
}

/// Whether the total of all is the groups added up, which holds when every issue stands in one
/// group and every group was read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FromGroups {
    /// Never: it was not asked for, an issue may stand in several groups, or the page read starts
    /// after a group. Its statement is read when it was asked for.
    Never,
    /// Always: every group is read.
    Always,
    /// When the first page read holds fewer rows than it may: then it holds every group, and the
    /// statement of all is not read.
    UnlessCut(u64),
}

impl TotalsPlan {
    /// Whether the statement of all is to be read, after the groups gave `rows` rows.
    pub(crate) fn reads_all(&self, rows: usize) -> bool {
        self.all.is_some()
            && match self.from_groups {
                FromGroups::Never => true,
                FromGroups::Always => false,
                FromGroups::UnlessCut(limit) => !whole(rows, limit),
            }
    }

    /// Whether the total of all is the groups added up, after the groups gave `rows` rows.
    fn adds_up_groups(&self, rows: usize) -> bool {
        match self.from_groups {
            FromGroups::Never => false,
            FromGroups::Always => true,
            FromGroups::UnlessCut(limit) => whole(rows, limit),
        }
    }
}

/// Whether a page of `rows` rows read with this limit holds every group.
fn whole(rows: usize, limit: u64) -> bool {
    u64::try_from(rows).is_ok_and(|rows| rows < limit)
}

/// How the issues are split into groups, as the inner statement needs it.
pub(super) struct Split {
    pub(super) value: Expr,
    pub(super) rows: Option<Rows>,
    what: GroupValue,
    /// The value of the second field of one value, and what it is, when each group is split again.
    then: Option<(Expr, GroupValue)>,
    /// Whether a row of an issue may stand twice in one group, which the inner statement then
    /// reads once.
    distinct: bool,
}

impl Split {
    /// What the value of a group is.
    pub(super) const fn what(&self) -> GroupValue {
        self.what
    }
}

impl Compiler<'_> {
    /// The value of a field that a total adds up, for the issue of the statement.
    fn sum_value(&self, target: FieldTarget) -> Result<Expr, CompileError> {
        let summable = matches!(
            target,
            FieldTarget::EstimatedTime
                | FieldTarget::TimeLeft
                | FieldTarget::TimeSpent
                | FieldTarget::Own {
                    kind: FieldType::WholeNumber | FieldType::Decimal,
                    ..
                }
        );
        if !summable {
            return Err(CompileError::Unsupported(
                "a sum of a field that is no number",
            ));
        }
        Ok(self.subject_of(target)?.0.one_value())
    }

    /// The value of a sum where the reader may read it, `NULL` elsewhere. A field that is to count
    /// everywhere must be among the readable fields of the request, as for a filter.
    fn readable_value(&self, sum: &SumAsked) -> Result<Expr, CompileError> {
        let value = self.sum_value(sum.target)?;
        let readable = match &sum.readable {
            ReadableWhere::Everywhere => {
                if self
                    .request
                    .fields
                    .iter()
                    .any(|each| each.target == sum.target)
                {
                    return Ok(value);
                }
                nothing()
            }
            ReadableWhere::In(places) => places_condition(places),
        };
        Ok(CaseStatement::new()
            .case(readable, value)
            .finally(Expr::null())
            .into())
    }

    /// How the field of `by` splits the issues, with the people of a set as one group, and each
    /// group again by the second field when there is one.
    pub(super) fn split(&self, by: &GroupBy) -> Result<Split, CompileError> {
        let merged = !by.people.is_empty();
        let then = by
            .then
            .as_deref()
            .map(|word| self.one_grouping(word))
            .transpose()?;
        let (value, rows, what) = match self.grouping(&by.word)? {
            Grouping::One(value, what) => (value, None, what),
            Grouping::Many { rows, value, what } => (value, Some(rows), what),
        };
        let distinct = rows
            .as_ref()
            .is_some_and(|rows| merged || rows.repeats() == Repeats::Maybe);
        if !merged {
            return Ok(Split {
                value,
                rows,
                what,
                then,
                distinct,
            });
        }
        Ok(Split {
            value: people_sets(&value, &by.people),
            rows,
            what: GroupValue::People,
            then,
            distinct,
        })
    }

    /// The rows of the issues the search finds, each with its group (when split) and the value of
    /// each sum.
    fn inner(
        &self,
        split: Option<&Split>,
        sums: &[SumAsked],
    ) -> Result<SelectStatement, CompileError> {
        let mut select = Query::select();
        if let Some(split) = split {
            select.expr_as(split.value.clone(), GROUP_COLUMN);
            if let Some((then, _)) = &split.then {
                select.expr_as(then.clone(), THEN_COLUMN);
            }
        }
        select.expr_as(Expr::col((IssueRow::Table, IssueRow::Id)), ISSUE);
        for (sum, columns) in sums.iter().zip(SUM_COLUMNS) {
            select.expr_as(self.readable_value(sum)?, columns.value);
        }
        select.from(IssueRow::Table);
        if let Some(rows) = split.and_then(|split| split.rows.as_ref()) {
            rows.join_to(&mut select);
        }
        select.cond_where(found(self)?);
        if split.is_some_and(|split| split.distinct) {
            select.distinct();
        }
        Ok(select)
    }

    /// The statement that adds up the rows of [`Compiler::inner`], by group when split, and only
    /// the groups of `page` when there is one.
    fn added_up(
        &self,
        split: Option<&Split>,
        sums: &[SumAsked],
        page: Option<&GroupPage>,
    ) -> Result<Built, CompileError> {
        let inner = self.inner(split, sums)?;
        let mut outer = Query::select();
        if let Some(split) = split {
            outer.column(GROUP_COLUMN);
            if split.then.is_some() {
                outer.column(THEN_COLUMN);
            }
        }
        outer.expr_as(Func::count(Expr::col(sea_query::Asterisk)), ISSUES);
        for columns in SUM_COLUMNS.iter().take(sums.len()) {
            outer
                .expr_as(
                    bigint(Func::sum(Expr::col(columns.value)).into()),
                    columns.sum,
                )
                .expr_as(Func::count(Expr::col(columns.value)), columns.valued);
        }
        outer.from_subquery(inner, Alias::new(FOUND));
        if let Some(split) = split {
            outer.group_by_col(GROUP_COLUMN);
            if split.then.is_some() {
                outer.group_by_col(THEN_COLUMN);
            }
            if let Some(page) = page {
                self.cut_to(&mut outer, split.what, page)?;
            }
        }
        Ok(finish(self, outer))
    }

    /// Keeps the groups of `page`: those after the place it starts after, in the order of their
    /// values, the group without a value last, at most its limit.
    fn cut_to(
        &self,
        outer: &mut SelectStatement,
        what: GroupValue,
        page: &GroupPage,
    ) -> Result<(), CompileError> {
        let value = || match what {
            GroupValue::Text => self.bytes(Expr::col(GROUP_COLUMN)),
            _ => Expr::col(GROUP_COLUMN),
        };
        if let Some(after) = &page.after {
            outer.cond_where(if after.last {
                // Nothing comes after the group without a value.
                nothing()
            } else {
                Cond::any()
                    .add(value().gt(after_value(what, after)?))
                    .add(value().is_null())
            });
        }
        outer
            .order_by_expr_with_nulls(value(), Order::Asc, NullOrdering::Last)
            .limit(page.limit);
        Ok(())
    }
}

/// Whether the order of the values of a group in the statement is the order of their places
/// (`GroupKey::place`), so a page can be cut there: texts and IDs by their bytes, numbers and
/// lengths by their size. A priority or a status group is ordered by a list and has few values.
const fn pages_in_the_statement(what: GroupValue) -> bool {
    matches!(
        what,
        GroupValue::Text | GroupValue::Whole | GroupValue::Length | GroupValue::Id(_)
    )
}

/// The value of a group whose place is `after`, as the statement compares it.
fn after_value(what: GroupValue, after: &GroupPlace) -> Result<Expr, CompileError> {
    let other = || CompileError::Unsupported("a page of groups after a place of another kind");
    Ok(match what {
        GroupValue::Text => Expr::val(after.text.clone()),
        GroupValue::Whole | GroupValue::Length => Expr::val(after.number),
        GroupValue::Id(_) => Expr::val(bytes_of_hex(&after.text).ok_or_else(other)?),
        GroupValue::Priority | GroupValue::StatusGroup | GroupValue::People => {
            return Err(other());
        }
    })
}

/// The bytes of an ID from the hex of its place.
fn bytes_of_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(text.get(at..at + 2)?, 16).ok())
        .collect()
}

/// The condition of the issues of these places: in one of the projects, and there of one of the
/// issue types when only some count. Never true without a place.
fn places_condition(places: &[FieldPlace]) -> Cond {
    if places.is_empty() {
        return nothing();
    }
    let project = || Expr::col((IssueRow::Table, IssueRow::Project));
    let mut any = Cond::any();
    let whole: Vec<_> = places
        .iter()
        .filter(|place| place.types.is_none())
        .map(|place| place.project.stored())
        .collect();
    if !whole.is_empty() {
        any = any.add(project().is_in(whole));
    }
    for place in places {
        if let Some(types) = &place.types {
            any = any.add(
                Cond::all().add(project().eq(place.project.stored())).add(
                    Expr::col((IssueRow::Table, IssueRow::IssueType))
                        .is_in(types.iter().map(|each| each.stored())),
                ),
            );
        }
    }
    any
}

/// The place of the set of people a person is in, as the value of a group: `NULL` for no person
/// (the group without a value), -1 for a person in no set, which is no group.
fn people_sets(person: &Expr, sets: &[Vec<UserId>]) -> Expr {
    let mut case = CaseStatement::new().case(person.clone().is_null(), Expr::null());
    for (at, set) in sets.iter().enumerate() {
        case = case.case(
            person.clone().is_in(set.iter().map(|each| each.stored())),
            Expr::val(i64::try_from(at).unwrap_or(i64::MAX)),
        );
    }
    case.finally(Expr::val(-1_i64)).into()
}

/// The statements of the totals that `asked` asks of the request.
pub(crate) fn totals_statements(
    request: &SearchRequest,
    dialect: Dialect,
    asked: &TotalsAsked,
) -> Result<TotalsPlan, CompileError> {
    if asked.sums.len() > SUMS_MAX {
        return Err(CompileError::Unsupported(
            "more sums than a total may ask for",
        ));
    }
    // The compiler of each statement holds the walks of its own.
    let all = || Compiler::new(request, dialect).added_up(None, &asked.sums, None);
    let Some(by) = &asked.by else {
        return Ok(TotalsPlan {
            groups: None,
            all: asked.all.then(all).transpose()?,
            from_groups: FromGroups::Never,
        });
    };
    let compiler = Compiler::new(request, dialect);
    // A page is cut by LIMIT over the rows of the statement: with a second field a row is part
    // of a group, so the cut would end inside one and its total would be part of it.
    if by.then.is_some() && by.page.is_some() {
        return Err(CompileError::Unsupported(
            "a page of groups split by a second field",
        ));
    }
    let split = compiler.split(by)?;
    let paged = by
        .page
        .as_ref()
        .filter(|_| pages_in_the_statement(split.what));
    let groups = compiler.added_up(Some(&split), &asked.sums, paged)?;
    // An issue may stand in several groups: all of them are read again, each issue once. Else they
    // are the groups added up, when every group is read: always without a page, and on a first page
    // that holds fewer groups than it may.
    let from_groups = match (asked.all && split.rows.is_none(), paged) {
        (true, None) => FromGroups::Always,
        (true, Some(page)) if page.after.is_none() => FromGroups::UnlessCut(page.limit),
        _ => FromGroups::Never,
    };
    let statement = asked.all && from_groups != FromGroups::Always;
    Ok(TotalsPlan {
        groups: Some((
            groups,
            split.what,
            split.then.as_ref().map(|(_, what)| *what),
        )),
        all: statement.then(all).transpose()?,
        from_groups,
    })
}

/// The count and the sums of a row of a statement of the totals.
fn read_total(row: &Row, sums: usize) -> Result<Total, StoreError> {
    let count = |column: Name| -> Result<u64, StoreError> {
        u64::try_from(row.get::<i64>(column)?).map_err(|_| StoreError::Corrupt("a total"))
    };
    Ok(Total {
        issues: count(ISSUES)?,
        sums: SUM_COLUMNS
            .iter()
            .take(sums)
            .map(|columns| {
                Ok(Sum {
                    sum: row.get::<Option<i64>>(columns.sum)?,
                    valued: count(columns.valued)?,
                })
            })
            .collect::<Result<_, StoreError>>()?,
    })
}

/// The total of no issue.
fn nothing_counted(sums: usize) -> Total {
    Total {
        issues: 0,
        sums: vec![Sum::default(); sums],
    }
}

/// The groups from the rows of their statement, the total of the rows of no group (people in no
/// set), which counts in the total of all but in no group, and how many rows there were.
pub(crate) fn read_groups(
    rows: &[Row],
    (what, then): (GroupValue, Option<GroupValue>),
    sums: usize,
) -> Result<(Vec<GroupTotal>, Total), StoreError> {
    let mut groups = Vec::with_capacity(rows.len());
    let mut outside = nothing_counted(sums);
    for row in rows {
        let total = read_total(row, sums)?;
        let Some(key) = read_key(row, what)? else {
            outside = outside.plus(&total);
            continue;
        };
        let then = match then {
            Some(then) => {
                let Some(key) = read_key_at(row, then, THEN_COLUMN)? else {
                    outside = outside.plus(&total);
                    continue;
                };
                Some(key)
            }
            None => None,
        };
        groups.push(GroupTotal { key, then, total });
    }
    Ok((groups, outside))
}

/// The total of all the issues: from the row of its statement, or the groups and the rows of no
/// group added up when they hold every issue once, or nothing when it was not asked for. `read`
/// is how many rows the groups gave.
pub(crate) fn read_all(
    plan: &TotalsPlan,
    rows: Option<&[Row]>,
    (groups, outside, read): (&[GroupTotal], &Total, usize),
    sums: usize,
) -> Result<Total, StoreError> {
    if plan.adds_up_groups(read) {
        return Ok(groups
            .iter()
            .fold(outside.clone(), |all, group| all.plus(&group.total)));
    }
    rows.and_then(<[Row]>::first)
        .map_or(Ok(nothing_counted(sums)), |row| read_total(row, sums))
}

/// The groups in the order of their places, put together with the total of all.
pub(crate) fn totals_of(mut groups: Vec<GroupTotal>, all: Total) -> Totals {
    groups
        .sort_by_cached_key(|group| (group.key.place(), group.then.as_ref().map(GroupKey::place)));
    Totals { groups, all }
}
