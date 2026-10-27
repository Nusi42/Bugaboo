//! The index of the full-text search (ADR-0016, amendment M5): the port that matches words
//! ([`port`]), the rows it reads, and keeping them ([`write`]).

mod adapter;
mod port;
mod write;

pub use port::{TextScope, TextSearch, text_issues, text_of_issue};
#[cfg(any(test, feature = "testing"))]
pub use write::index_text;
pub(crate) use write::reindex_all;
#[cfg(test)]
pub(crate) use write::reindex_issues;

#[cfg(test)]
mod tests;
