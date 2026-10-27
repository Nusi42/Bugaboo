//! Migration 22: the index of the full-text search (M5 WP-05, ADR-0016).
//!
//! `text_index` holds one row for each text of an issue (its title, description, each comment,
//! each file name, each own long-text field) with the prepared words of the text. Both databases
//! get the same table; each adds what only it has:
//!
//! - SQLite: an FTS5 table that follows `text_index` by triggers (external content, the numeric
//!   `id` is its row number, which stays when the file is compacted). Only prepared words go in,
//!   so the tokenizer only has to split at spaces.
//! - PostgreSQL: a `tsvector` column made from the words in the `simple` configuration (no
//!   stemming, no stop words), and a GIN index on it.
//!
//! The text of the statements is fixed (no value goes into it); they are written out here because
//! the query builder cannot describe a virtual table or a generated column.

use sea_query::{ColumnDef, ForeignKey, Index, Table};

use super::{Step, id_column};
use crate::db::FixedSql;
use crate::schema::{IssueRow, TextIndexRow};

/// The FTS5 table (SQLite).
const FTS_TABLE: &str = "CREATE VIRTUAL TABLE text_index_fts USING fts5(words, content='text_index', \
     content_rowid='id', prefix='2 3', tokenize='unicode61 remove_diacritics 0')";

/// The triggers that keep the FTS5 table in step with `text_index` (SQLite). A later migration that
/// makes the table again takes these, not a copy.
pub(super) const FTS_TRIGGERS: [&str; 3] = [
    "CREATE TRIGGER text_index_after_insert AFTER INSERT ON text_index BEGIN \
     INSERT INTO text_index_fts(rowid, words) VALUES (new.id, new.words); END",
    "CREATE TRIGGER text_index_after_delete AFTER DELETE ON text_index BEGIN \
     INSERT INTO text_index_fts(text_index_fts, rowid, words) VALUES ('delete', old.id, old.words); END",
    "CREATE TRIGGER text_index_after_update AFTER UPDATE ON text_index BEGIN \
     INSERT INTO text_index_fts(text_index_fts, rowid, words) VALUES ('delete', old.id, old.words); \
     INSERT INTO text_index_fts(rowid, words) VALUES (new.id, new.words); END",
];

/// The statements that take the triggers and the FTS5 table away again (SQLite): the names of
/// [`FTS_TABLE`] and [`FTS_TRIGGERS`] stand here and nowhere else.
pub(super) const DROP_FTS: [&str; 4] = [
    "DROP TRIGGER text_index_after_insert",
    "DROP TRIGGER text_index_after_delete",
    "DROP TRIGGER text_index_after_update",
    "DROP TABLE text_index_fts",
];

/// The generated column (PostgreSQL).
const TSVECTOR_COLUMN: &str = "ALTER TABLE text_index ADD COLUMN words_tsv tsvector \
     GENERATED ALWAYS AS (to_tsvector('simple', words)) STORED";

/// The index on the generated column (PostgreSQL).
pub(super) const TSVECTOR_INDEX: &str =
    "CREATE INDEX text_index_words ON text_index USING GIN (words_tsv)";

/// The statements that take the index and the column away again (PostgreSQL).
pub(super) const DROP_TSVECTOR: [&str; 2] = [
    "DROP INDEX text_index_words",
    "ALTER TABLE text_index DROP COLUMN words_tsv",
];

pub(super) fn steps() -> Vec<Step> {
    vec![
        Step::Table(
            Table::create()
                .table(TextIndexRow::Table)
                .col(
                    ColumnDef::new(TextIndexRow::Id)
                        .big_integer()
                        .not_null()
                        .auto_increment()
                        .primary_key(),
                )
                .col(id_column(TextIndexRow::Issue))
                .col(ColumnDef::new(TextIndexRow::Source).text().not_null())
                .col(id_column(TextIndexRow::SourceRef))
                .col(ColumnDef::new(TextIndexRow::Words).text().not_null())
                .foreign_key(
                    ForeignKey::create()
                        .from(TextIndexRow::Table, TextIndexRow::Issue)
                        .to(IssueRow::Table, IssueRow::Id),
                )
                .to_owned(),
        ),
        // One row for each text; a rebuild of an issue replaces its rows.
        Step::Index(
            Index::create()
                .name("text_index_by_text")
                .table(TextIndexRow::Table)
                .col(TextIndexRow::Issue)
                .col(TextIndexRow::Source)
                .col(TextIndexRow::SourceRef)
                .unique()
                .to_owned(),
        ),
        Step::Sql(FixedSql {
            sqlite: [FTS_TABLE].into_iter().chain(FTS_TRIGGERS).collect(),
            postgres: vec![TSVECTOR_COLUMN, TSVECTOR_INDEX],
        }),
    ]
}
