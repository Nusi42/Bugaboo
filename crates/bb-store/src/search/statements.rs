//! The statements of a search: a page of issues, how many there are, how many in each group, and the
//! place of an issue in the order (to continue after it). All of them start from the same sight, so
//! the count line and the groups always agree with the page.

use bb_app::search::{GroupCount, GroupKey, SearchHit, SearchRequest};
use bb_core::id::{ChoiceId, GroupId, IssueId, IssueTypeId, LabelId, ProjectId, TagId, UserId};
use bb_core::issue::{Priority, StatusGroup};
use bb_core::tags::IssueSight;
use bb_core::text_index::TextSource;
use bb_core::text_words::{SearchWord, search_words};
use bb_searchlang::{MAX_CONTAINS_WORDS, MAX_SORT_KEYS};
use sea_query::{
    Alias, Cond, Expr, ExprTrait as _, Func, Iden, NullOrdering, Order, Query, SelectStatement,
    UnionType, WithClause, WithQuery,
};

use super::order::{GroupValue, Grouping, IdOf, Key, Kind, SortValue};
use super::{CompileError, Compiler, Form};
use crate::db::{Dialect, Row, Statement, StoredId as _};
use crate::error::StoreError;
use crate::issues::{issue_select, read_issue};
use crate::schema::IssueRow;
use crate::sight::sight_condition;
use crate::text_index::{TextScope, text_rows_up_to};

/// A statement, with the walks it starts with when the search follows sub-issues or relations.
pub(crate) enum Built {
    Plain(SelectStatement),
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

fn finish(compiler: &Compiler<'_>, select: SelectStatement) -> Built {
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
    /// Whether the first key is "the title has the words": the page is then the issues that have
    /// them in the title, newest first, and after them the others, newest first (two streams).
    titled: bool,
    /// Whether the issues of one stream come in the order of their creation (either way), which the
    /// index of the project and the creation gives.
    by_creation: bool,
}

impl SortPlan {
    /// The keys of the order of the request.
    pub(crate) fn of(request: &SearchRequest, dialect: Dialect) -> Result<Self, CompileError> {
        // The words are found one issue at a time: the key is asked for one issue, or for the few
        // rows of a page.
        let compiler = Compiler::new(request, dialect).paged();
        let keys = compiler.keys()?;
        let kinds = keys.iter().map(|key| key.kind).collect();
        let titled = compiler.orders_by_title();
        let created = Expr::col((IssueRow::Table, IssueRow::CreatedAt));
        let by_creation = keys.len() == 1 + usize::from(titled)
            && keys.last().is_some_and(|key| key.value == created);
        Ok(Self {
            keys,
            kinds,
            titled,
            by_creation,
        })
    }

    /// The kinds of the keys, which tell how to read the values of a row.
    pub(crate) fn kinds(&self) -> &[Kind] {
        &self.kinds
    }

    /// Whether the search has the order by title hits (the first key).
    pub(crate) const fn titled(&self) -> bool {
        self.titled
    }

    /// The keys that order the issues of one stream: all, but the title hit that the stream has
    /// alone.
    fn stream_keys(&self) -> &[Key] {
        self.keys
            .get(usize::from(self.titled)..)
            .unwrap_or_default()
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

/// The most projects of the sight that a page is asked of one by one (`branches`).
const MOST_BRANCHES: usize = 16;

/// How many rows of the text index a word may have for the issues that have it to be made once
/// (`Form::Driven`); with more, the issues are asked about one at a time (`Form::Each`).
pub(crate) const SET_ROWS_AT_MOST: i64 = 2_000;

/// How the words that decide the order by title hits are found for each of its two streams.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Forms {
    /// The issues that have the words in the title.
    pub(crate) title: Form,
    /// The issues that have the words in some text, and not in the title.
    pub(crate) rest: Form,
}

impl Forms {
    /// Every issue is asked.
    pub(crate) const EACH: Self = Self {
        title: Form::Each,
        rest: Form::Each,
    };
}

/// The sights of the projects of the sight, one each, when they are few and more than one: the
/// first page of a search over a few projects (a project and its sub-projects) is then the first
/// page of each, put together, and each of them is read from the index of its project and the
/// creation, which no database can merge from several projects at once.
fn branches(sight: &IssueSight) -> Option<Vec<IssueSight>> {
    let count = sight.all.len() + sight.own.len();
    if !(2..=MOST_BRANCHES).contains(&count) {
        return None;
    }
    let all = sight.all.iter().map(|project| IssueSight {
        all: vec![*project],
        own: Vec::new(),
        ..sight.clone()
    });
    let own = sight.own.iter().map(|project| IssueSight {
        all: Vec::new(),
        own: vec![*project],
        ..sight.clone()
    });
    Some(all.chain(own).collect())
}

/// Where the issue a page continues after stands among the two streams of the title hits.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Standing {
    /// The first page.
    First,
    /// The last issue has the words in its title: the rest of that stream, then the other.
    Hit,
    /// The last issue has not: the rest of the other stream.
    Rest,
}

/// The conditions of the streams of a page: one, or for the order by title hits the issues that
/// have the words in the title and then the others, each after the issue the page continues after.
fn streams(
    compiler: &Compiler<'_>,
    plan: &SortPlan,
    last: Option<(&[SortValue], IssueId)>,
    forms: Forms,
) -> Result<Vec<Cond>, CompileError> {
    if !plan.titled() {
        let mut cond = found_by(compiler)?;
        if let Some((values, id)) = last {
            cond = cond.add(Compiler::after(&plan.keys, values, id));
        }
        return Ok(vec![cond]);
    }
    let (standing, later) = match last {
        None => (Standing::First, None),
        Some((values, id)) => (
            if values.first() == Some(&SortValue::Whole(1)) {
                Standing::Hit
            } else {
                Standing::Rest
            },
            Some(Compiler::after(
                plan.stream_keys(),
                values.get(1..).unwrap_or_default(),
                id,
            )),
        ),
    };
    let unsupported = || CompileError::Unsupported("an order by title hits without words");
    let mut streams = Vec::new();
    if standing != Standing::Rest {
        // The issues of this stream have the words in the title, so they have them.
        compiler.has_the_words.set(true);
        let found = found_by(compiler);
        compiler.has_the_words.set(false);
        let found = found?;
        compiler.form.set(forms.title);
        let hit = compiler.title_hit().ok_or_else(unsupported)?;
        let mut cond = Cond::all().add(found).add(hit);
        if let (Standing::Hit, Some(later)) = (standing, &later) {
            cond = cond.add(later.clone());
        }
        streams.push(cond);
    }
    compiler.form.set(forms.rest);
    let found = found_by(compiler)?;
    compiler.form.set(Form::Each);
    let hit = compiler.title_hit().ok_or_else(unsupported)?;
    let mut cond = Cond::all().add(found).add(hit.not());
    if let (Standing::Rest, Some(later)) = (standing, later) {
        cond = cond.add(later);
    }
    streams.push(cond);
    Ok(streams)
}

/// A page of the issues the search finds, in its order, after the issue whose place is `last`.
///
/// The issues of a stream come in the order of their creation, from the index, and the page is
/// cut after its first rows. Several streams (the title hits and the others) and several projects
/// are each cut after the rows of a page and put together, so what comes first is never decided by
/// a cap: a page of 40 is made of the first 40 of each, of which the first 40 are kept.
pub(crate) fn page_statement(
    request: &SearchRequest,
    dialect: Dialect,
    plan: &SortPlan,
    last: Option<(&[SortValue], IssueId)>,
    forms: Forms,
) -> Result<Built, CompileError> {
    let compiler = Compiler::new(request, dialect).paged();
    let streams = streams(&compiler, plan, last, forms)?;
    let sights = if compiler.text_used() || !plan.by_creation {
        None
    } else {
        branches(&request.sight)
    }
    .unwrap_or_else(|| vec![request.sight.clone()]);
    let mut pieces = Vec::new();
    for stream in &streams {
        for sight in &sights {
            pieces.push(Cond::all().add(sight_condition(sight)).add(stream.clone()));
        }
    }
    let mut select = issue_select();
    if let [only] = pieces.as_slice() {
        select.cond_where(only.clone()).limit(request.limit);
        Compiler::order_by(&mut select, plan.stream_keys());
        return Ok(finish(&compiler, select));
    }
    // The pieces are the first rows of each stream of each project; the page is the first rows of
    // them all.
    let mut ids: Option<SelectStatement> = None;
    for (at, piece) in pieces.into_iter().enumerate() {
        let mut first = Query::select();
        first
            .column((IssueRow::Table, IssueRow::Id))
            .from(IssueRow::Table)
            .cond_where(piece)
            .limit(request.limit);
        Compiler::order_by(&mut first, plan.stream_keys());
        let mut wrapped = Query::select();
        wrapped
            .column(IssueRow::Id)
            .from_subquery(first, Alias::new(format!("piece{at}")));
        match ids.as_mut() {
            None => ids = Some(wrapped),
            Some(all) => {
                all.union(UnionType::All, wrapped);
            }
        }
    }
    let ids = ids.ok_or(CompileError::Unsupported("a page without a piece"))?;
    select
        .and_where(Expr::col((IssueRow::Table, IssueRow::Id)).in_subquery(ids))
        .limit(request.limit);
    Compiler::order_by(&mut select, &plan.keys);
    Ok(finish(&compiler, select))
}

/// The filter of the search as the compiler is set (the sight is added by the caller, once for
/// each project of a branch).
fn found_by(compiler: &Compiler<'_>) -> Result<Cond, CompileError> {
    match &compiler.request.search.search().filter {
        Some(filter) => compiler.filter(filter),
        None => Ok(Cond::all()),
    }
}

/// How many issues the search finds.
pub(crate) fn count_statement(
    request: &SearchRequest,
    dialect: Dialect,
) -> Result<Built, CompileError> {
    let compiler = Compiler::new(request, dialect);
    let mut select = Query::select();
    select
        .expr_as(Func::count(Expr::col(sea_query::Asterisk)), Counted)
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
                .expr_as(Func::count(Expr::col(sea_query::Asterisk)), Counted)
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
                IdOf::Tag => GroupKey::Tag(TagId::read(bytes)?),
                IdOf::Label => GroupKey::Label(LabelId::read(bytes)?),
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

/// The names of the columns of the probe: the stream, the word and how many rows it has.
#[derive(Clone, Copy)]
enum Probed {
    Stream,
    Word,
    Rows,
}

impl Iden for Probed {
    fn unquoted(&self) -> &'static str {
        match self {
            Self::Stream => "stream",
            Self::Word => "word",
            Self::Rows => "found",
        }
    }
}

/// The streams of a page by title hits, with the scope their texts are in.
const STREAMS: [(i64, TextScope); 2] = [
    (0, TextScope::Source(TextSource::Title)),
    (1, TextScope::Everywhere),
];

/// How many rows of the text index each word of the order by title hits has in the title and in
/// all texts, up to `at_most` each: a row for each word and stream. None without such an
/// order.
pub(crate) fn probe_statement(
    request: &SearchRequest,
    dialect: Dialect,
    at_most: i64,
) -> Option<SelectStatement> {
    let compiler = Compiler::new(request, dialect);
    let words: Vec<SearchWord> = search_words(compiler.title_words()?)
        .into_iter()
        .take(MAX_CONTAINS_WORDS)
        .collect();
    let mut probe: Option<SelectStatement> = None;
    for (stream, scope) in STREAMS {
        for (at, word) in words.iter().enumerate() {
            let counted = text_rows_up_to(dialect, word, scope, at_most.unsigned_abs());
            let mut one = Query::select();
            one.expr_as(Expr::val(stream), Probed::Stream)
                .expr_as(
                    Expr::val(i64::try_from(at).unwrap_or(i64::MAX)),
                    Probed::Word,
                )
                .expr_as(super::scalar(counted), Probed::Rows);
            match probe.as_mut() {
                None => probe = Some(one),
                Some(all) => {
                    all.union(UnionType::All, one);
                }
            }
        }
    }
    probe
}

/// How to find the issues of each stream, from the rows of the probe: the word that has fewest
/// rows when it has fewer than `at_most`, else one issue at a time.
pub(crate) fn read_forms(rows: &[Row], at_most: i64) -> Result<Forms, StoreError> {
    let mut fewest: [Option<(i64, i64)>; 2] = [None, None];
    for row in rows {
        let stream = usize::try_from(row.get::<i64>(Probed::Stream)?)
            .map_err(|_| StoreError::Corrupt("a probe"))?;
        let word = row.get::<i64>(Probed::Word)?;
        let found = row.get::<i64>(Probed::Rows)?;
        let slot = fewest
            .get_mut(stream)
            .ok_or(StoreError::Corrupt("a probe"))?;
        if found < at_most && slot.is_none_or(|(least, _)| found < least) {
            *slot = Some((found, word));
        }
    }
    let form = |slot: Option<(i64, i64)>| match slot {
        Some((_, word)) => usize::try_from(word).map_or(Form::Each, Form::Driven),
        None => Form::Each,
    };
    Ok(Forms {
        title: form(fewest[0]),
        rest: form(fewest[1]),
    })
}
