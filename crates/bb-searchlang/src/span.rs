//! Where a node stands in the text.

use serde::Serialize;

/// A range of bytes in the text of a search, so the browser can mark a word. Comparing two spans
/// always says they are equal: two trees are the same tree when their words are the same, wherever
/// the words stood (`parse(print(tree)) == tree` holds although the printer moves everything).
#[derive(Debug, Clone, Copy, Default, Eq, Serialize)]
pub struct Span {
    /// The first byte.
    pub start: usize,
    /// The byte after the last one.
    pub end: usize,
}

impl PartialEq for Span {
    fn eq(&self, _: &Self) -> bool {
        true
    }
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
