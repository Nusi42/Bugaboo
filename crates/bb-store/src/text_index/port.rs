//! The port of the full-text search: the part of a search statement that matches words (ADR-0016,
//! amendment M5). Pure query-builder code, no access to a database, so the compiler of the search
//! (WP-04) and its tests call it without one.
//!
//! The port matches and nothing else. Who may see what is decided in one place, the compiler:
//! it joins [`TextMatch::select`] to `issue`, applies its visibility predicates there (the issue,
//! its project, and for a search in a field that the field is readable in the project of the
//! issue), and so a hit never comes from a text the reader may not see. The port never reads a
//! permission.
//!
//! A text matches when it has every word of the search (all words in one text). The order is not
//! the port's either: it hands the compiler `title_hit` (1 when the title has the words) for
//! "title hits first, then the newest" (ADR-0016, amendment M5), with no ranking function, so both
//! databases give the same order.

use bb_core::id::FieldId;
use bb_core::text_index::TextSource;
use bb_core::text_words::SearchWord;
use sea_query::{Alias, BinOper, Expr, ExprTrait as _, Func, Query, SelectStatement, SimpleExpr};

use crate::db::{Dialect, StoredId as _};
use crate::schema::{TextIndexExtra, TextIndexRow};

/// Where a search looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextScope {
    /// A bare `contains`: titles, descriptions, comments and file names; never own fields.
    Everywhere,
    /// `<field> contains`: the rows of that own long-text field only.
    Field(FieldId),
}

/// What to find.
#[derive(Clone, Copy, Debug)]
pub struct TextSearch<'a> {
    /// The prepared words (`bb_core::text_words::search_words`); every one must be in a text. No
    /// words match nothing.
    pub words: &'a [SearchWord],
    /// Where to look.
    pub scope: TextScope,
}

/// The sub-select of a search: for each issue with a matching text, one row with the columns
/// [`TextMatch::ISSUE`] (the issue ID) and [`TextMatch::TITLE_HIT`] (1 when its title matches,
/// else 0).
#[derive(Clone, Debug)]
pub struct TextMatch {
    /// The sub-select. The compiler uses it as a derived table (`FROM (...) AS text_hit`, joined
    /// on `issue.id`), or [`TextMatch::issue_ids`] as the right side of `issue.id IN (...)`.
    pub select: SelectStatement,
}

impl TextMatch {
    /// The name of the issue column of [`TextMatch::select`].
    pub const ISSUE: &'static str = "issue_id";
    /// The name of the title column of [`TextMatch::select`].
    pub const TITLE_HIT: &'static str = "title_hit";
}

/// The sub-select of a search for one database.
#[must_use]
pub fn text_matches(dialect: Dialect, search: &TextSearch<'_>) -> TextMatch {
    let mut select = rows(dialect, search);
    select
        .expr_as(Expr::col(TextIndexRow::Issue), Alias::new(TextMatch::ISSUE))
        .expr_as(
            Func::max(
                Expr::case(
                    Expr::col(TextIndexRow::Source).eq(TextSource::Title.code()),
                    1_i64,
                )
                .finally(0_i64),
            ),
            Alias::new(TextMatch::TITLE_HIT),
        )
        .group_by_col(TextIndexRow::Issue);
    TextMatch { select }
}

/// The issue IDs of a search only, for `issue.id IN (...)` when the compiler does not order by the
/// title.
#[must_use]
pub fn matching_issue_ids(dialect: Dialect, search: &TextSearch<'_>) -> SelectStatement {
    let mut select = rows(dialect, search);
    select.distinct().column(TextIndexRow::Issue);
    select
}

/// `FROM text_index WHERE <the words match> AND <the scope>`, without columns.
fn rows(dialect: Dialect, search: &TextSearch<'_>) -> SelectStatement {
    let mut select = Query::select();
    select
        .from(TextIndexRow::Table)
        .and_where(words_match(dialect, search.words));
    match search.scope {
        TextScope::Everywhere => {
            select.and_where(Expr::col(TextIndexRow::Source).is_in([
                TextSource::Title.code(),
                TextSource::Description.code(),
                TextSource::Comment.code(),
                TextSource::File.code(),
            ]));
        }
        TextScope::Field(field) => {
            select
                .and_where(Expr::col(TextIndexRow::Source).eq(TextSource::Field.code()))
                .and_where(Expr::col(TextIndexRow::SourceRef).eq(field.stored()));
        }
    }
    select
}

/// The condition that the words of the search are in the words of a row, as the database says it.
fn words_match(dialect: Dialect, words: &[SearchWord]) -> SimpleExpr {
    // A word is letters and digits and nothing else, whatever the caller built, so no word can
    // carry the syntax of the query of the database.
    let words: Vec<(String, bool)> = words
        .iter()
        .map(|word| {
            (
                word.text
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect::<String>(),
                word.prefix,
            )
        })
        .filter(|(text, _)| !text.is_empty())
        .collect();
    if words.is_empty() {
        return Expr::val(1).eq(0);
    }
    match dialect {
        Dialect::Sqlite => {
            // FTS5: `"one" "two"*`, phrases of one word side by side mean AND.
            let query = words
                .iter()
                .map(|(text, prefix)| {
                    if *prefix {
                        ["\"", text, "\"*"].concat()
                    } else {
                        ["\"", text, "\""].concat()
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            let mut inner = Query::select();
            inner
                .expr(Expr::col(Alias::new("rowid")))
                .from(TextIndexExtra::Fts)
                .and_where(Expr::col(TextIndexExtra::Fts).binary(BinOper::Custom("MATCH"), query));
            Expr::col(TextIndexRow::Id).in_subquery(inner)
        }
        Dialect::Postgres => {
            // tsquery: `one & two:*`.
            let query = words
                .iter()
                .map(|(text, prefix)| {
                    if *prefix {
                        [text.as_str(), ":*"].concat()
                    } else {
                        text.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(" & ");
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
    use bb_core::text_words::search_words;
    use sea_query::{PostgresQueryBuilder, SqliteQueryBuilder};

    use super::*;

    fn text_of(dialect: Dialect, words: &[SearchWord]) -> String {
        let select = text_matches(
            dialect,
            &TextSearch {
                words,
                scope: TextScope::Everywhere,
            },
        )
        .select;
        match dialect {
            Dialect::Sqlite => select.to_string(SqliteQueryBuilder),
            Dialect::Postgres => select.to_string(PostgresQueryBuilder),
        }
    }

    #[test]
    fn the_words_become_one_query_of_the_database() {
        let words = search_words("crash stor*");
        let sqlite = text_of(Dialect::Sqlite, &words);
        assert!(sqlite.contains(r#"MATCH '"crash" "stor"*'"#), "{sqlite}");
        let postgres = text_of(Dialect::Postgres, &words);
        assert!(
            postgres.contains("to_tsquery(CAST('simple' AS regconfig), 'crash & stor:*')"),
            "{postgres}"
        );
    }

    #[test]
    fn syntax_in_a_word_cannot_reach_the_query() {
        let words = vec![SearchWord {
            text: "a\" OR b; --".into(),
            prefix: false,
        }];
        let text = text_of(Dialect::Sqlite, &words);
        assert!(text.contains(r#"MATCH '"aORb"'"#), "{text}");
    }

    #[test]
    fn no_words_match_nothing() {
        let text = text_of(Dialect::Postgres, &[]);
        assert!(text.contains("1 = 0"), "{text}");
    }
}
