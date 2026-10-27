//! The search compiler (QRY-04, QRY-05, SEC-17, ADR-0005): the checked tree of a search becomes one
//! parameterized statement that finds the issues the person may read, filtered, sorted and counted
//! only by the fields they may read, on both databases.
//!
//! The statement is made with the query builder only. A value in a search text is always a bound
//! value (`sec_17_a_search_text_never_becomes_sql`), a name is a table or column from `schema`, and
//! a word of the search is looked up among the readable fields: a word that is not one cannot be
//! compiled, so a field the person may not read is not reachable even if a caller forgets to check.
//! The visibility of ADR-0004 (the sight) is the first condition of every statement, and every
//! lookup of an issue by its ID (`under`, `blocks`, `parent is`, …) is held to it too, so a search
//! cannot tell a hidden issue from one that does not exist.
//!
//! The pieces: `filter` (the tree), `source` (what a field reads), `values` (what a value means),
//! `time` (relative times in the zone of the person), `order` (sorting and the cursor) and
//! `statements` (the statements and their results).

use std::cell::{Cell, RefCell};

use bb_app::search::{
    FieldTarget, GroupCount, GroupKey, SearchFailure, SearchHit, SearchRequest, SearchStore,
    target_of,
};
use bb_core::issue::{Priority, StatusGroup};
use sea_query::{CommonTableExpression, Cond, Expr, ExprTrait as _};

use self::statements::{
    Forms, SET_ROWS_AT_MOST, SortPlan, count_statement, groups_statement, page_statement,
    place_statement, probe_statement, read_count, read_forms, read_group, read_hit, read_place,
};
use crate::auth::AuthStorage;
use crate::db::{Dialect, Statement};
use crate::error::StoreError;

mod filter;
mod order;
mod source;
mod statements;
mod text;
mod time;
mod values;

/// Why a search cannot be compiled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CompileError {
    /// A word that is not among the readable fields.
    UnknownField,
    /// A test the compiler does not know for the field.
    Unsupported(&'static str),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownField => formatter.write_str("a word that is not a readable field"),
            Self::Unsupported(what) => write!(formatter, "not supported yet: {what}"),
        }
    }
}

/// The compiler of one request: it holds the common table expressions that the helpers of the
/// search need (`under`, `connected to`), which the statement starts with.
pub(crate) struct Compiler<'a> {
    pub(crate) request: &'a SearchRequest,
    pub(crate) dialect: Dialect,
    /// How the words of a `contains` are found (see [`Form`]); a page is cut after its first rows,
    /// so it asks about the issues one at a time.
    pub(crate) form: Cell<Form>,
    /// Whether the condition made so far asks the text index anything.
    text_used: Cell<bool>,
    /// Whether the issues are known to have the words that decide the order by title hits (they
    /// have them in the title), so a `contains` of those words anywhere is true without asking.
    pub(crate) has_the_words: Cell<bool>,
    tables: RefCell<Vec<CommonTableExpression>>,
}

/// How a statement finds the issues that have the words of a `contains`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Form {
    /// The issues that have the words, made once (`id IN (...)`): for a count or the groups, which
    /// ask about all of them.
    Set,
    /// Each issue is asked (`EXISTS`): a page that is cut after its first rows stops at the first
    /// ones, which is quick when many issues have the words.
    Each,
    /// The issues that have one word of the words, made once, and each of them asked about the
    /// other words: quick when that word is in few texts. The number is the place of the word.
    Driven(usize),
}

impl<'a> Compiler<'a> {
    pub(crate) const fn new(request: &'a SearchRequest, dialect: Dialect) -> Self {
        Self {
            request,
            dialect,
            form: Cell::new(Form::Set),
            text_used: Cell::new(false),
            has_the_words: Cell::new(false),
            tables: RefCell::new(Vec::new()),
        }
    }

    /// The compiler of a statement that is a page.
    pub(crate) const fn paged(mut self) -> Self {
        self.form = Cell::new(Form::Each);
        self
    }

    /// Whether the condition made so far asks the text index.
    pub(crate) fn text_used(&self) -> bool {
        self.text_used.get()
    }

    /// What the word reads. The word of a field that is not among the readable ones cannot be
    /// compiled.
    pub(crate) fn target(&self, word: &str) -> Result<FieldTarget, CompileError> {
        target_of(&self.request.fields, word).ok_or(CompileError::UnknownField)
    }

    /// What the target reads, with the sight and the reach of the own fields of this request.
    fn subject_of(&self, target: FieldTarget) -> (source::Subject, source::Domain) {
        source::subject_of(target, &self.request.sight, &self.request.own_reach)
    }

    /// The number the next walk is named with (`walk0`, `walk1`, …).
    pub(crate) fn next_table(&self) -> usize {
        self.tables.borrow().len()
    }

    /// Keeps a common table expression, already named, for the head of the statement.
    pub(crate) fn keep(&self, table: CommonTableExpression) {
        self.tables.borrow_mut().push(table);
    }

    pub(crate) fn take_tables(&self) -> Vec<CommonTableExpression> {
        self.tables.take()
    }
}

/// A select that answers one value, as an expression: the one place that turns a query into a value.
pub(crate) fn scalar(select: sea_query::SelectStatement) -> Expr {
    Expr::from(sea_query::SubQueryStatement::SelectStatement(select))
}

/// A condition that is never true.
pub(crate) fn nothing() -> Cond {
    Cond::all().add(Expr::val(1).eq(2))
}

/// How long a search may run on the database (SEC-17, architecture §7 *Search limits*).
const SEARCH_TIME: std::time::Duration = std::time::Duration::from_secs(2);

/// The place of a word among all the words of its kind.
fn place_in<T: PartialEq>(all: &[T], one: &T) -> i64 {
    all.iter()
        .position(|each| each == one)
        .and_then(|place| i64::try_from(place).ok())
        .unwrap_or(0)
}

/// The place of a group in the order: the group without a value last, a priority and a status group
/// by the order of a list (`LIST_ORDER`, not the order that sorting and comparing use), the others
/// by the bytes of their text or ID, or by the size of their number.
fn group_order(group: &GroupCount) -> (bool, String, i64) {
    match &group.key {
        GroupKey::Empty => (true, String::new(), 0),
        GroupKey::Whole(number) => (false, String::new(), *number),
        GroupKey::Text(text) => (false, text.clone(), 0),
        GroupKey::Person(id) => (false, id.to_string(), 0),
        GroupKey::Group(id) => (false, id.to_string(), 0),
        GroupKey::Issue(id) => (false, id.to_string(), 0),
        GroupKey::Project(id) => (false, id.to_string(), 0),
        GroupKey::Tag(id) => (false, id.to_string(), 0),
        GroupKey::Label(id) => (false, id.to_string(), 0),
        GroupKey::IssueType(id) => (false, id.to_string(), 0),
        GroupKey::Choice(id) => (false, id.to_string(), 0),
        GroupKey::Priority(each) => (false, String::new(), place_in(&Priority::LIST_ORDER, each)),
        GroupKey::StatusGroup(each) => (
            false,
            String::new(),
            place_in(&StatusGroup::LIST_ORDER, each),
        ),
    }
}

fn failed(error: impl std::fmt::Display) -> SearchFailure {
    SearchFailure::Failed(error.to_string())
}

fn stopped(error: StoreError) -> SearchFailure {
    match error {
        StoreError::TimedOut => SearchFailure::TooSlow,
        other => failed(other),
    }
}

impl AuthStorage {
    /// How the words that decide the order by title hits are found (see [`Form`]), by asking the
    /// text index how many rows each word has; every issue is asked about when the order is not by
    /// title hits.
    async fn forms_of(
        &self,
        request: &SearchRequest,
        at_most: i64,
    ) -> Result<Forms, SearchFailure> {
        let database = self.database();
        let Some(probe) = probe_statement(request, database.dialect(), at_most) else {
            return Ok(Forms::EACH);
        };
        let rows = database
            .fetch_all_within(Statement::Select(&probe), SEARCH_TIME)
            .await
            .map_err(stopped)?;
        read_forms(&rows, at_most).map_err(failed)
    }

    /// A page of a search, with the forms the words are found in.
    async fn search_page_in(
        &self,
        request: &SearchRequest,
        forms: Forms,
    ) -> Result<Vec<SearchHit>, SearchFailure> {
        let database = self.database();
        let dialect = database.dialect();
        let plan = SortPlan::of(request, dialect).map_err(failed)?;
        // To continue after an issue the store reads where it stands in the order: a cursor holds
        // nothing but its ID, so no value of any issue is ever written into one.
        let place = match request.after {
            Some(after) => {
                let built = place_statement(request, &plan, after);
                let rows = database
                    .fetch_all_within(&built, SEARCH_TIME)
                    .await
                    .map_err(stopped)?;
                let row = rows.first().ok_or(SearchFailure::BadCursor)?;
                Some((read_place(row, plan.kinds()).map_err(failed)?, after))
            }
            None => None,
        };
        let last = place.as_ref().map(|(values, id)| (values.as_slice(), *id));
        let built = page_statement(request, dialect, &plan, last, forms).map_err(failed)?;
        let rows = database
            .fetch_all_within(&built, SEARCH_TIME)
            .await
            .map_err(stopped)?;
        rows.iter()
            .map(read_hit)
            .collect::<Result<_, _>>()
            .map_err(failed)
    }
}

impl SearchStore for AuthStorage {
    async fn search_page(&self, request: &SearchRequest) -> Result<Vec<SearchHit>, SearchFailure> {
        let forms = self.forms_of(request, SET_ROWS_AT_MOST).await?;
        self.search_page_in(request, forms).await
    }

    async fn search_count(&self, request: &SearchRequest) -> Result<u64, SearchFailure> {
        let built = count_statement(request, self.database().dialect()).map_err(failed)?;
        let rows = self
            .database()
            .fetch_all_within(&built, SEARCH_TIME)
            .await
            .map_err(stopped)?;
        rows.first()
            .map(read_count)
            .transpose()
            .map_err(failed)
            .map(Option::unwrap_or_default)
    }

    async fn search_groups(
        &self,
        request: &SearchRequest,
        by: &str,
    ) -> Result<Vec<GroupCount>, SearchFailure> {
        let (built, what) =
            groups_statement(request, self.database().dialect(), by).map_err(failed)?;
        let rows = self
            .database()
            .fetch_all_within(&built, SEARCH_TIME)
            .await
            .map_err(stopped)?;
        let mut groups: Vec<GroupCount> = rows
            .iter()
            .map(|row| read_group(row, what))
            .collect::<Result<_, _>>()
            .map_err(failed)?;
        // The groups of a field with many values come from two reads put together, so they are
        // put in order here: by their value, the group without a value last.
        groups.sort_by_cached_key(group_order);
        Ok(groups)
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_more;
#[cfg(test)]
mod tests_rest;
#[cfg(test)]
mod tests_streams;
