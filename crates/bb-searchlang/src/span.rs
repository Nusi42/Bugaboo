//! Where a node stands in the text.

use serde::Serialize;

/// A range of bytes in the text of a search, so the browser can mark a word. Two spans are equal
/// when they are the same range. A tree compares its places too; `Search::same_words` compares the
/// words only.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Span {
    /// The first byte.
    pub start: usize,
    /// The byte after the last one.
    pub end: usize,
}

impl Span {
    /// The range from `start` up to `end`.
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// The range from the start of `self` to the end of `last`.
    #[must_use]
    pub const fn to(self, last: Self) -> Self {
        Self {
            start: self.start,
            end: last.end,
        }
    }
}
