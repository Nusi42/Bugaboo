//! The search language: parse, print, check and suggest. No I/O; also built for WebAssembly (ADR-0005).
//!
//! One grammar, in Rust, that the server and the browser both use (QRY-03). Simple search edits the
//! tree ([`Search`]); Advanced search edits the text it prints to ([`print`]); both always show the
//! same search because the text that is printed is the one canonical text of the tree. The grammar
//! is written in architecture section 7.
//!
//! The steps, each a module: [`parse`] reads a text into the tree (never panics, on any text),
//! [`print`] writes the canonical text, [`check`] looks at the tree with the [`ReadableFields`] of
//! the fields the person may read, and [`suggest`] says what may come next at the cursor.

#![forbid(unsafe_code)]

#[macro_use]
mod every;
mod duration;

mod answer;
#[cfg(feature = "browser")]
mod browser;
mod checker;
mod fields;
mod lexer;
mod limits;
mod literal;
pub mod messages;
mod mistake;
mod parser;
mod printer;
mod similar;
mod situation;
mod span;
mod suggest;
mod syntax;
pub mod vectors;
mod words;

pub use answer::{Answer, read};
#[cfg(feature = "browser")]
pub use browser::{Problem, print_from_json, readable_fields_from_json, search_from_json};
pub use checker::{CheckedSearch, check};
pub use fields::{Field, FieldKind, IssueField, NO, Operator, ReadableFields, YES, own_field_word};
pub use limits::{
    MAX_CONTAINS_WORDS, MAX_DEPTH, MAX_FILTERS, MAX_SORT_KEYS, MAX_STEPS, MAX_TEXT_BYTES,
    MIN_PREFIX_CHARS,
};
pub use mistake::Mistake;
pub use printer::print;
pub use span::Span;
pub use suggest::{AskKind, Suggestion, Suggestions, WordKind, suggest};
pub use syntax::{
    Comparison, Date, Decimal, Direction, Expr, FieldRef, IssueRef, Operand, ProjectRef, Search,
    SortKey, Steps, Test, Text, TimeWord, Unit,
};

// Switches on the feature `browser` for the tests of this crate (`unused_crate_dependencies`).
#[cfg(test)]
use bb_searchlang as _;

/// Reads the text of a search into its tree. A word that is written like an Issue ID is read as one
/// wherever a value stands, because without the fields nothing says what the field holds; see
/// [`parse_with`].
///
/// # Errors
/// The first mistake of the text: where it is and what is wrong, in message keys.
pub fn parse(text: &str) -> Result<Search, Mistake> {
    parser::parse(text)
        .result
        .map_err(|error| Mistake::from(&error))
}

/// Reads the text of a search into its tree, and reads a value by its field: an Issue ID is read
/// only where the field holds issues (`parent is web-142`), and a word that is written like one
/// stays the text it is where it does not (`tag is re-test`). The server and the browser read a
/// search this way before they check it.
///
/// # Errors
/// The first mistake of the text: where it is and what is wrong, in message keys.
pub fn parse_with(text: &str, fields: &ReadableFields) -> Result<Search, Mistake> {
    parser::parse_for(text, Some(fields))
        .result
        .map_err(|error| Mistake::from(&error))
}
