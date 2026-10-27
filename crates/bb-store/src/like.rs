//! Patterns of `LIKE` for a text typed by a person: the characters that `LIKE` gives a meaning to
//! (`%`, `_` and the escape itself) are written as themselves, so one place decides it for every
//! read that looks for the start or the middle of a text.

use sea_query::LikeExpr;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_characters_of_a_pattern_stand_for_themselves() {
        assert_eq!(plain(r"50%_a\b"), r"50\%\_a\\b");
    }
}
