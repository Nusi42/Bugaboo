//! The search language: parse, print, check and suggest. No I/O; also built for WebAssembly (ADR-0005).
//!
//! One grammar, in Rust, that the server and the browser both use (QRY-03). Simple search edits the
//! tree ([`Search`]); Advanced search edits the text it prints to ([`print`]); both always show the
//! same search because the text that is printed is the one canonical text of the tree. The grammar
//! is written in architecture section 7.
//!
//! The steps, each a module: [`parse`] reads a text into the tree (never panics, on any text),
//! [`print`] writes the canonical text, [`check`] looks at the tree with the [`FieldCatalogue`] of
//! the fields the person may read, and [`suggest`] says what may come next at the cursor.

#![forbid(unsafe_code)]

mod answer;
mod catalogue;
mod checker;
mod json;
mod lexer;
mod limits;
mod literal;
mod messages;
mod mistake;
mod parser;
mod printer;
mod similar;
mod span;
mod suggest;
mod syntax;
pub mod vectors;
mod words;

pub use answer::{Answer, read};
pub use catalogue::{Field, FieldCatalogue, FieldKind, Operator, own_field_word};
pub use checker::{CheckedSearch, check};
pub use json::{Problem, catalogue_from_json, search_from_json};
pub use limits::{MAX_CONDITIONS, MAX_DEPTH, MAX_STEPS};
pub use mistake::Mistake;
pub use printer::print;
pub use span::Span;
pub use suggest::{AskKind, Suggestion, Suggestions, WordKind, suggest};
pub use syntax::{
    Comparison, Date, Decimal, Direction, Expr, FieldRef, Operand, ReadableId, Search, ShortName,
    SortKey, Steps, Test, Text, TimeWord, Unit,
};

/// Reads the text of a search into its tree.
///
/// # Errors
/// The first mistake of the text: where it is and what is wrong, in message keys.
pub fn parse(text: &str) -> Result<Search, Mistake> {
    parser::parse(text)
        .result
        .map_err(|error| Mistake::from(&error))
}
