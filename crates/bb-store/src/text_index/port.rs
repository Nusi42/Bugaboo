//! The port of the full-text search: the part of a search statement that matches words (ADR-0016,
//! amendment M5). Pure query-builder code, no access to a database, so the compiler of the search
//! (WP-04) and its tests call it without one.
//!
//! The port matches and nothing else. Who may see what is decided in one place, the compiler:
//! it joins [`TextMatch::select`] to `issue`, applies its visibility predicates there (the issue,
//! its project, and for a search in a field that the field is readable in the project and for the
//! type of that issue), and so a hit never comes from a text the reader may not see. The port
//! never reads a permission.
//!
//! An issue matches when every word of the search is in some text of the scope, not necessarily
//! the same one (a title with "login" and a comment with "error" find `login error`): one
//! sub-select for each word, all in the one statement of the search. The order is not the port's
//! either: "title hits first, then the newest" (ADR-0016, amendment M5) is asked by the compiler
//! with the scope [`TextScope::Source`] on the title, with no ranking function, so both databases
//! give the same order.
//!
//! The cost of one search is bounded here, for a caller that did not run the checker: a word that
//! comes twice counts once (`search_words`), no more than [`MAX_CONTAINS_WORDS`] words are looked
//! up, and a start of fewer than `MIN_PREFIX_CHARS` letters is looked up as the whole word.

use bb_core::id::FieldId;
use bb_core::text_index::TextSource;
use bb_core::text_words::SearchWord;
use bb_searchlang::MAX_CONTAINS_WORDS;
use sea_query::{Alias, BinOper, Expr, ExprTrait as _, Func, Query, SelectStatement, SimpleExpr};

use crate::db::{Dialect, StoredId as _};
use crate::schema::{TextIndexExtra, TextIndexRow};

/// Where a search looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextScope {
    /// A bare `contains`: titles, descriptions, comments and file names; never own fields.
    Everywhere,
    /// `<field> contains`: the rows of that own long-text field only. The index holds the value
    /// of every issue whatever the rules of the field, so the compiler must add, in the same
    /// statement, that the field is readable in the project and shown for the type of the issue.
    Field(FieldId),
    /// `title contains` and `description contains`: the rows of that one built-in source of an
    /// issue. Every word must be in that one text.
    Source(TextSource),
}

/// What to find.
#[derive(Clone, Copy, Debug)]
pub struct TextSearch<'a> {
    /// The prepared words (`bb_core::text_words::search_words`); every one must be in the scope.
    /// No words match nothing.
    pub words: &'a [SearchWord],
    /// Where to look.
    pub scope: TextScope,
}

/// The sub-select of a search: for each issue that has every word, one row with the columns
/// [`TextMatch::ISSUE`] (the issue ID). Whether the title alone has the words is asked of the scope
/// [`TextScope::Source`] by the compiler of the search, which is the one place that orders.
#[derive(Clone, Debug)]
pub struct TextMatch {
    /// The sub-select. The compiler uses it as a derived table (`FROM (...) AS text_hit`, joined
    /// on `issue.id`).
    pub select: SelectStatement,
}

impl TextMatch {
    /// The name of the issue column of [`TextMatch::select`].
    pub const ISSUE: &'static str = "issue_id";
}

/// One word as it is looked up: the text, and whether it matches by its start.
type Lookup<'a> = (&'a str, bool);

/// The conditions on a row of the index that make it one of the texts of an issue that has the
/// first word in the scope and whose issue also has every other word in the scope. No words match
/// nothing.
fn row_conditions(dialect: Dialect, search: &TextSearch<'_>) -> Vec<SimpleExpr> {
    let words = lookups(search.words);
    let Some((first, rest)) = words.split_first() else {
        return vec![Expr::val(1).eq(0)];
    };
    let mut conditions = vec![word_matches(dialect, *first), in_scope(search.scope)];
    for word in rest {
        conditions.push(Expr::col(TextIndexRow::Issue).in_subquery(issues_with(
            dialect,
            *word,
            &in_scope(search.scope),
        )));
    }
    conditions
}

/// The sub-select of a search for one database.
#[must_use]
pub fn text_matches(dialect: Dialect, search: &TextSearch<'_>) -> TextMatch {
    let mut select = Query::select();
    select.from(TextIndexRow::Table);
    for condition in row_conditions(dialect, search) {
        select.and_where(condition);
    }
    TextMatch {
        select: with_columns(select),
    }
}

/// The issues that match, one row for each text of theirs that does (`SELECT issue FROM
/// text_index WHERE …`), for a condition `issue.id IN (…)` that asks about all issues at once: the
/// database makes the set once. An issue is in it as often as it has texts that match.
#[must_use]
pub fn text_issues(dialect: Dialect, search: &TextSearch<'_>) -> SelectStatement {
    let mut select = Query::select();
    select.column(TextIndexRow::Issue).from(TextIndexRow::Table);
    for condition in row_conditions(dialect, search) {
        select.and_where(condition);
    }
    select
}

/// The same search asked of one issue: `SELECT 1 FROM text_index WHERE issue = <that issue> AND …`
/// for the compiler to put in `EXISTS (…)`, where a page is cut after the first rows and an issue
/// is asked about at a time. `issue` is the column of the issue it asks about.
#[must_use]
pub fn text_of_issue(
    dialect: Dialect,
    search: &TextSearch<'_>,
    issue: SimpleExpr,
) -> SelectStatement {
    let mut select = Query::select();
    select
        .expr(Expr::val(1))
        .from(TextIndexRow::Table)
        .and_where(Expr::col(TextIndexRow::Issue).eq(issue));
    for condition in row_conditions(dialect, search) {
        select.and_where(condition);
    }
    select
}

/// The columns of the sub-select, and the grouping that makes one row for an issue.
fn with_columns(mut select: SelectStatement) -> SelectStatement {
    select
        .expr_as(Expr::col(TextIndexRow::Issue), Alias::new(TextMatch::ISSUE))
        .group_by_col(TextIndexRow::Issue);
    select
}

/// The words as they are looked up: the first ten (`search_words` has made each one once), a
/// short start as a whole word.
fn lookups(words: &[SearchWord]) -> Vec<Lookup<'_>> {
    words
        .iter()
        .take(MAX_CONTAINS_WORDS)
        .map(|word| (word.text(), word.matches_by_start()))
        .collect()
}

/// The condition that a row is from the scope.
fn in_scope(scope: TextScope) -> SimpleExpr {
    match scope {
        TextScope::Everywhere => Expr::col(TextIndexRow::Source).is_in([
            TextSource::Title.code(),
            TextSource::Description.code(),
            TextSource::Comment.code(),
            TextSource::File.code(),
        ]),
        TextScope::Source(source) => in_source(source),
        TextScope::Field(field) => {
            in_source(TextSource::Field).and(Expr::col(TextIndexRow::SourceRef).eq(field.stored()))
        }
    }
}

/// The condition that a row is of one source.
fn in_source(source: TextSource) -> SimpleExpr {
    Expr::col(TextIndexRow::Source).eq(source.code())
}

/// The issues that have a text of the scope with this word.
fn issues_with(dialect: Dialect, word: Lookup<'_>, scope: &SimpleExpr) -> SelectStatement {
    let mut select = Query::select();
    select
        .column(TextIndexRow::Issue)
        .from(TextIndexRow::Table)
        .and_where(word_matches(dialect, word))
        .and_where(scope.clone());
    select
}

/// The condition that the words of a row have this word, as the database says it. The word is
/// letters and digits and nothing else (`SearchWord` is made by `search_words` only), so it carries
/// no syntax of the query of the database.
fn word_matches(dialect: Dialect, (text, prefix): Lookup<'_>) -> SimpleExpr {
    match dialect {
        Dialect::Sqlite => {
            // FTS5: `"word"` or `"word"*`.
            let query = if prefix {
                ["\"", text, "\"*"].concat()
            } else {
                ["\"", text, "\""].concat()
            };
            let mut inner = Query::select();
            inner
                .expr(Expr::col(Alias::new("rowid")))
                .from(TextIndexExtra::Fts)
                .and_where(Expr::col(TextIndexExtra::Fts).binary(BinOper::Custom("MATCH"), query));
            Expr::col(TextIndexRow::Id).in_subquery(inner)
        }
        Dialect::Postgres => {
            // tsquery: `word` or `word:*`.
            let query = if prefix {
                [text, ":*"].concat()
            } else {
                text.to_owned()
            };
            Expr::col(TextIndexExtra::WordsTsv).binary(
                BinOper::Custom("@@"),
                Func::cust(Alias::new("to_tsquery"))
                    .arg(Expr::cust("CAST('simple' AS regconfig)"))
                    .arg(query),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use bb_core::id::SequentialIds;
    use bb_core::text_words::search_words;
    use sea_query::{PostgresQueryBuilder, SqliteQueryBuilder};

    use super::*;

    /// The statement with its values written in, for a test to read (the store never does this).
    fn text_of(dialect: Dialect, input: &str, scope: TextScope) -> String {
        let words = search_words(input);
        let select = text_matches(
            dialect,
            &TextSearch {
                words: &words,
                scope,
            },
        )
        .select;
        match dialect {
            Dialect::Sqlite => select.to_string(SqliteQueryBuilder),
            Dialect::Postgres => select.to_string(PostgresQueryBuilder),
        }
    }

    #[test]
    fn each_word_is_one_lookup_of_the_database() {
        let sqlite = text_of(Dialect::Sqlite, "crash stor*", TextScope::Everywhere);
        assert!(sqlite.contains(r#"MATCH '"crash"'"#), "{sqlite}");
        assert!(sqlite.contains(r#"MATCH '"stor"*'"#), "{sqlite}");
        let postgres = text_of(Dialect::Postgres, "crash stor*", TextScope::Everywhere);
        assert!(
            postgres.contains("to_tsquery(CAST('simple' AS regconfig), 'crash')"),
            "{postgres}"
        );
        assert!(
            postgres.contains("to_tsquery(CAST('simple' AS regconfig), 'stor:*')"),
            "{postgres}"
        );
    }

    #[test]
    fn the_words_may_be_in_different_texts_of_the_scope() {
        let text = text_of(Dialect::Postgres, "login error", TextScope::Everywhere);
        // One lookup in the scope for each word: the first in the statement, the second in a
        // sub-select, and nothing about the title (the order is the compiler's).
        assert_eq!(text.matches("'error')").count(), 1, "{text}");
        assert_eq!(text.matches("'login')").count(), 1, "{text}");
        assert!(!text.contains("source\" = 'title'"), "{text}");
    }

    #[test]
    fn syntax_in_a_search_cannot_reach_the_query() {
        let text = text_of(
            Dialect::Sqlite,
            "a\" OR b; -- title:x NEAR(y)",
            TextScope::Everywhere,
        );
        assert!(!text.contains("NEAR"), "{text}");
        assert!(!text.contains("--"), "{text}");
        assert!(text.contains(r#"MATCH '"or"'"#), "{text}");
    }

    #[test]
    fn no_words_match_nothing() {
        let text = text_of(Dialect::Postgres, " ... ", TextScope::Everywhere);
        assert!(text.contains("1 = 0"), "{text}");
    }

    #[test]
    fn a_word_that_comes_twice_is_looked_up_once() {
        let text = text_of(
            Dialect::Postgres,
            "crash crash CRASH",
            TextScope::Everywhere,
        );
        assert_eq!(text.matches("'crash')").count(), 1, "{text}");
    }

    #[test]
    fn no_more_than_ten_words_are_looked_up() {
        let many: Vec<String> = (0..25).map(|n| format!("word{n}")).collect();
        let text = text_of(Dialect::Postgres, &many.join(" "), TextScope::Everywhere);
        assert!(text.contains("'word9'"), "{text}");
        assert!(!text.contains("'word10'"), "{text}");
    }

    #[test]
    fn a_start_of_one_letter_is_looked_up_as_the_whole_word() {
        let text = text_of(Dialect::Postgres, "a* ab*", TextScope::Everywhere);
        assert!(!text.contains("'a:*'"), "{text}");
        assert!(text.contains("'a')"), "{text}");
        assert!(text.contains("'ab:*'"), "{text}");
    }

    #[test]
    fn a_search_in_a_field_looks_at_the_rows_of_that_field_only() {
        let field = FieldId::generate(&mut SequentialIds::default()).unwrap();
        let text = text_of(Dialect::Postgres, "notes", TextScope::Field(field));
        assert!(text.contains("source\" = 'field'"), "{text}");
        assert!(!text.contains("source\" = 'title'"), "{text}");
    }
}
