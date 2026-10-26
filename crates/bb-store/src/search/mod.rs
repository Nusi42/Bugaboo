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

use std::cell::RefCell;

use bb_app::search::{
    FieldTarget, GroupCount, SearchFailure, SearchHit, SearchRequest, SearchStore, target_of,
};
use sea_query::{CommonTableExpression, Cond, Expr, ExprTrait as _};

use self::statements::{
    count_statement, groups_statement, kinds_of, page_statement, read_count, read_group, read_hit,
};
use crate::auth::AuthStorage;
use crate::db::Dialect;
use crate::error::StoreError;

mod filter;
mod order;
mod source;
mod statements;
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
    tables: RefCell<Vec<CommonTableExpression>>,
}

impl<'a> Compiler<'a> {
    pub(crate) const fn new(request: &'a SearchRequest, dialect: Dialect) -> Self {
        Self {
            request,
            dialect,
            tables: RefCell::new(Vec::new()),
        }
    }

    /// What the word reads. The word of a field that is not among the readable ones cannot be
    /// compiled.
    pub(crate) fn target(&self, word: &str) -> Result<FieldTarget, CompileError> {
        target_of(&self.request.fields, word).ok_or(CompileError::UnknownField)
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

/// The place of a group in the order: the groups with a value first, by the bytes of a text or the
/// size of a number.
fn group_order(kind: order::Kind, group: &GroupCount) -> (bool, Option<i64>, Option<String>) {
    let number = match kind {
        order::Kind::Whole => group.value.as_deref().and_then(|text| text.parse().ok()),
        order::Kind::Text => None,
    };
    let text = match kind {
        order::Kind::Text => group.value.clone(),
        order::Kind::Whole => None,
    };
    (group.value.is_none(), number, text)
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

impl SearchStore for AuthStorage {
    async fn search_page(&self, request: &SearchRequest) -> Result<Vec<SearchHit>, SearchFailure> {
        let dialect = self.database().dialect();
        let kinds = kinds_of(request, dialect).map_err(failed)?;
        let built = page_statement(request, dialect).map_err(failed)?;
        let rows = self
            .database()
            .fetch_all_within(&built, SEARCH_TIME)
            .await
            .map_err(stopped)?;
        rows.iter()
            .map(|row| read_hit(row, &kinds))
            .collect::<Result<_, _>>()
            .map_err(failed)
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
        let (built, kind) =
            groups_statement(request, self.database().dialect(), by).map_err(failed)?;
        let rows = self
            .database()
            .fetch_all_within(&built, SEARCH_TIME)
            .await
            .map_err(stopped)?;
        let mut groups: Vec<GroupCount> = rows
            .iter()
            .map(|row| read_group(row, kind))
            .collect::<Result<_, _>>()
            .map_err(failed)?;
        // The groups of a field with many values come from two reads put together, so they are
        // put in order here: by their value as text or number, the group without a value last.
        groups.sort_by(|one, other| group_order(kind, one).cmp(&group_order(kind, other)));
        Ok(groups)
    }
}

#[cfg(test)]
mod tests;
