//! The limits of a search, named once (ADR-0005, amendment M5, decision 6). The checker enforces
//! the first two and the parser the third; the server and the browser read the same numbers, so
//! they cannot differ.

/// The most conditions a search may have. `in (a, b, c)` is one condition.
pub const MAX_CONDITIONS: usize = 50;

/// The most steps `connected to … within N steps` may take.
pub const MAX_STEPS: u32 = 5;

/// How deep brackets and `not` may nest. A search with 50 conditions needs far less; the limit
/// keeps a hostile text from exhausting the stack of the parser, in the server and in the browser.
pub const MAX_DEPTH: usize = 32;
