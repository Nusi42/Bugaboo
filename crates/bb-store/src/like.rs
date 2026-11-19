//! Patterns of `LIKE` for a text typed by a person: the characters that `LIKE` gives a meaning to
//! (`%`, `_` and the escape itself) are written as themselves, so one place decides it for every
//! read that looks for the start or the middle of a text.

use sea_query::{BinOper, Expr, ExprTrait as _, Func, LikeExpr};

/// The character that makes the next one in a pattern mean itself.
const ESCAPE: char = '\\';

/// `text` with the characters that a pattern gives a meaning to made plain.
fn plain(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    for character in text.chars() {
        if matches!(character, '%' | '_' | ESCAPE) {
            plain.push(ESCAPE);
        }
        plain.push(character);
    }
    plain
}

/// Texts that hold `text` anywhere.
pub(crate) fn containing(text: &str) -> LikeExpr {
    LikeExpr::new(format!("%{}%", plain(text))).escape(ESCAPE)
}

/// Texts that begin with `text`.
pub(crate) fn starting_with(text: &str) -> LikeExpr {
    LikeExpr::new(format!("{}%", plain(text))).escape(ESCAPE)
}

/// `column` matches `pattern`, with capitals not counting the way the database folds them: both
/// sides go through the database's own `lower`, so the rule is one rule on both sides of the
/// comparison. `lower` folds only the letters of ASCII on SQLite (and on PostgreSQL without a
/// locale), so it is only for texts that are ASCII anyway (an address). A name is looked for in
/// its stored key (`bb_core::names::name_key`, made in Rust), with [`starting_with`] and
/// [`containing`].
fn any_case(column: impl Into<Expr>, pattern: String) -> Expr {
    let like = Expr::Binary(
        Box::new(Func::lower(Expr::val(pattern)).into()),
        BinOper::Escape,
        Box::new(Expr::Constant(ESCAPE.into())),
    );
    Expr::expr(Func::lower(column.into())).binary(BinOper::Like, like)
}

/// `column` holds `text` anywhere, capitals not counting (see [`any_case`]).
pub(crate) fn containing_any_case(column: impl Into<Expr>, text: &str) -> Expr {
    any_case(column, format!("%{}%", plain(text)))
}

/// The fewest characters a pattern of [`may_hold_folded`] keeps: with fewer it would pick nearly
/// every row, so the caller reads them all instead.
pub(crate) const FOLDED_KEPT_MIN: usize = 3;

/// The rows of `column` that may hold `folded`, a text folded in Rust (`bb_core::names::name_key`):
/// a pattern that keeps only the ASCII letters and digits the database folds the way Rust does,
/// with `%` for every run of other characters. `lower` folds only ASCII on SQLite (and on
/// PostgreSQL without a locale), so a letter outside it (`Ö`, `Ł`, Cyrillic) is any run; so are
/// `i` and `k`, which the dotted capital `İ` and the Kelvin sign fold to in Rust but not in the
/// database; and so are spaces, quotes and the escapes of JSON. The rows are only candidates: the
/// caller decides in Rust. `None` when fewer than [`FOLDED_KEPT_MIN`] characters are kept.
pub(crate) fn may_hold_folded(column: impl Into<Expr>, folded: &str) -> Option<Expr> {
    let mut pattern = String::from('%');
    let mut kept = 0;
    for character in folded.chars() {
        if character.is_ascii_alphanumeric() && !matches!(character, 'i' | 'k') {
            pattern.push(character.to_ascii_lowercase());
            kept += 1;
        } else if !pattern.ends_with('%') {
            pattern.push('%');
        }
    }
    if !pattern.ends_with('%') {
        pattern.push('%');
    }
    (kept >= FOLDED_KEPT_MIN).then(|| any_case(column, pattern))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_query::{Alias, Query, SqliteQueryBuilder};

    fn pattern_of(folded: &str) -> Option<String> {
        may_hold_folded(Expr::col(Alias::new("t")), folded)
            .map(|expr| Query::select().expr(expr).to_string(SqliteQueryBuilder))
    }

    #[test]
    fn priv_04_a_folded_text_keeps_only_what_the_database_folds_alike() {
        let found = pattern_of("özlem çelik").unwrap_or_default();
        assert!(found.contains("'%zlem%el%'"), "{found}");
        assert!(pattern_of("юлия").is_none());
        assert!(pattern_of("kiki li").is_none());
    }

    #[test]
    fn the_characters_of_a_pattern_stand_for_themselves() {
        assert_eq!(plain(r"50%_a\b"), r"50\%\_a\\b");
    }
}
