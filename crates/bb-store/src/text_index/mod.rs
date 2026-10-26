//! The index of the full-text search (ADR-0016, amendment M5): the port that matches words
//! ([`port`]), the rows it reads, and keeping them ([`write`]).

mod adapter;
mod port;
mod write;

pub use port::{TextMatch, TextScope, TextSearch, text_issues, text_matches, text_of_issue};
#[cfg(any(test, feature = "testing"))]
pub use write::index_text;
#[cfg(test)]
pub(crate) use write::reindex_issues;

#[cfg(test)]
mod tests;
