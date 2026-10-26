//! The table of the full-text search (ADR-0016, amendment M5).

use super::Iden;

/// One text of an issue that a search may look into, with its prepared words. On SQLite a table
/// `text_index_fts` (FTS5) follows it, on PostgreSQL the column `words_tsv` does; both are made
/// by the migration and read only by the search port (`text_index::port`).
#[derive(Clone, Copy, Iden)]
pub(crate) enum TextIndexRow {
    #[iden = "text_index"]
    Table,
    Id,
    Issue,
    Source,
    SourceRef,
    Words,
}

/// The extras of the full-text search that exist on one database only.
#[derive(Clone, Copy, Iden)]
pub(crate) enum TextIndexExtra {
    /// The FTS5 table that follows `text_index` (SQLite).
    #[iden = "text_index_fts"]
    Fts,
    /// The `tsvector` column made from `words` (PostgreSQL).
    #[iden = "words_tsv"]
    WordsTsv,
}
