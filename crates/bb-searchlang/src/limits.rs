//! The limits of a search, named once (ADR-0005, amendment M5, decision 6). The checker enforces
//! the filters, the steps and the sort keys, and the parser the depth and the length of the text;
//! the server and the browser read the same numbers, so they cannot differ.

/// The most filters a search may have. `in (a, b, c)` counts as many as it has values, because
/// each value is one more comparison for the database.
pub const MAX_FILTERS: usize = 50;

/// The most steps `connected to … within N steps` may take.
pub const MAX_STEPS: u32 = 5;

/// The most keys `sort by` may have.
pub const MAX_SORT_KEYS: usize = 5;

/// How deep brackets and `not` may nest. A search with 50 filters needs far less; the limit
/// keeps a hostile text from exhausting the stack of the parser, in the server and in the browser.
pub const MAX_DEPTH: usize = 32;

/// The most bytes the text of a search may have. A search of the most filters, each of them with
/// a field, an operator and a value of a usual length (about 60 bytes), is 3 000 bytes. Nothing
/// is read of a longer text, so the work for a text, the lexer, the parser and the search for
/// close words, is bounded by this number, in the server and in the browser.
pub const MAX_TEXT_BYTES: usize = 4096;
