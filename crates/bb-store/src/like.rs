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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_characters_of_a_pattern_stand_for_themselves() {
        assert_eq!(plain(r"50%_a\b"), r"50\%\_a\\b");
    }
}
