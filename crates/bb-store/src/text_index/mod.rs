//! The index of the full-text search (ADR-0016, amendment M5): the port that matches words
//! ([`port`]), the rows it reads, and keeping them ([`write`]).

mod adapter;
mod port;
mod write;

pub use port::{TextMatch, TextScope, TextSearch, matching_issue_ids, text_matches};
#[cfg(any(test, feature = "testing"))]
pub use write::index_text;

#[cfg(test)]
mod tests;
