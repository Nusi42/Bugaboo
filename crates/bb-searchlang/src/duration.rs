//! Lengths of time in a search: whole numbers each followed by `minutes` or `hours`, like
//! `1 hour 30 minutes` (ADR-0005, decision 10). The units have one fixed length.

use crate::lexer::{Token, TokenKind};
use crate::words::{Keyword, TimeUnit};

/// The minutes that the token at `at` stands for, if it is a word that names a unit of time.
fn unit_at(tokens: &[Token], at: usize) -> Option<u32> {
    match tokens.get(at).map(|token| &token.kind) {
        Some(TokenKind::Word(word)) => Keyword::of(word)
            .and_then(TimeUnit::of)
            .map(TimeUnit::minutes),
        _ => None,
    }
}

/// Whether a length of time starts at `at`: a number and then a unit.
pub(crate) fn starts_duration(tokens: &[Token], at: usize) -> bool {
    matches!(
        tokens.get(at).map(|token| &token.kind),
        Some(TokenKind::Number(_))
    ) && unit_at(tokens, at + 1).is_some()
}

/// The length of time that starts at `at`: its minutes and how many tokens it used. `None` when a
/// count is not a whole number or the sum is too large to count.
pub(crate) fn read_duration(tokens: &[Token], at: usize) -> Option<(u32, usize)> {
    let mut minutes: u32 = 0;
    let mut used = 0;
    while let Some(TokenKind::Number(number)) = tokens.get(at + used).map(|token| &token.kind)
        && let Some(unit) = unit_at(tokens, at + used + 1)
    {
        let count = u32::try_from(number.value)
            .ok()
            .filter(|_| number.scale == 0)?;
        minutes = count
            .checked_mul(unit)
            .and_then(|added| minutes.checked_add(added))?;
        used += 2;
    }
    Some((minutes, used))
}

/// A length of time in words: `2 hours`, `1 hour 30 minutes`, `45 minutes`.
pub(crate) fn duration_text(minutes: u32) -> String {
    let mut left = minutes;
    let mut parts = Vec::new();
    for unit in TimeUnit::LONGEST_FIRST {
        let count = left / unit.minutes();
        left %= unit.minutes();
        if count > 0 {
            parts.push(format!("{count} {}", unit.word(count).text()));
        }
    }
    if parts.is_empty() {
        parts.push(format!("0 {}", TimeUnit::Minute.word(0).text()));
    }
    parts.join(" ")
}
