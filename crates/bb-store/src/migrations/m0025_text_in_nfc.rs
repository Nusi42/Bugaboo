//! Migration 25: the words of the full-text search keep their letters (M5-F53, ADR-0016,
//! amendment M5). Until now a text was prepared with its accents removed (*Müller* was stored as
//! `muller`, `ß` as `ss`); now it is lower case and Unicode NFC and nothing else (*Müller* is
//! stored as `müller`).
//!
//! The migration only empties the index and puts a mark on the place of the reader of the outbox
//! (the largest place there is); it does not prepare a text itself. At a start the server takes
//! the head of the outbox, makes the rows of every issue again from the stored texts, a page and a
//! transaction at a time, with the code of the program that runs and without the migration lock,
//! and puts the place of the reader at that head (`TextIndexStore::rebuild_text_index_if_wanted`).
//! Until it has come to the end the index is empty or partial, so a search finds less; the jobs
//! do not wait for it, and the reader of the outbox reads nothing until it has finished, then
//! reads what came after the head. The names of the table and of the reader and the mark are
//! written out here, not read from the program, so the migration says what a database of
//! migration 24 holds and what the program asks for from now on.
//!
//! The databases only store and compare the words Rust prepared: the FTS5 table is made again with
//! a tokenizer that keeps every letter, number and mark of a word, and the `tsvector` of PostgreSQL
//! is made from the words split at spaces, not by the parser of the text search, which depends on
//! the locale of the database.

use sea_query::{Expr, ExprTrait as _, Query};

use super::Step;
use crate::db::FixedSql;
use crate::schema::{OutboxPosition, TextIndexRow};

/// The name the reader of the index has in the positions of the outbox.
const READER: &str = "text-index";

/// The mark: a place no event is after (`bb_core::text_index::REBUILD_WANTED`).
const REBUILD_WANTED: i64 = i64::MAX;

/// The FTS5 table made again, and the triggers that keep it in step (SQLite). The tokenizer takes
/// letters, numbers, private-use characters and marks as the characters of a word and splits at
/// everything else, so a stored word is never cut inside (`unicode61` alone keeps only the marks
/// U+0300 to U+0331). No accent is removed: the words are what Rust prepared. The text of the
/// statements is the one of migration 22, apart from the tokenizer.
const SQLITE: [&str; 8] = [
    "DROP TRIGGER text_index_after_insert",
    "DROP TRIGGER text_index_after_delete",
    "DROP TRIGGER text_index_after_update",
    "DROP TABLE text_index_fts",
    "CREATE VIRTUAL TABLE text_index_fts USING fts5(words, content='text_index',      content_rowid='id', prefix='2 3',      tokenize=\"unicode61 remove_diacritics 0 categories 'L* N* Co M*'\")",
    "CREATE TRIGGER text_index_after_insert AFTER INSERT ON text_index BEGIN      INSERT INTO text_index_fts(rowid, words) VALUES (new.id, new.words); END",
    "CREATE TRIGGER text_index_after_delete AFTER DELETE ON text_index BEGIN      INSERT INTO text_index_fts(text_index_fts, rowid, words) VALUES ('delete', old.id, old.words); END",
    "CREATE TRIGGER text_index_after_update AFTER UPDATE ON text_index BEGIN \
     INSERT INTO text_index_fts(text_index_fts, rowid, words) VALUES ('delete', old.id, old.words); \
     INSERT INTO text_index_fts(rowid, words) VALUES (new.id, new.words); END",
];

/// The vector made again from the stored words split at spaces, with no parser of the locale
/// (PostgreSQL). Its lexemes are the words as they are stored.
const POSTGRES: [&str; 4] = [
    "DROP INDEX text_index_words",
    "ALTER TABLE text_index DROP COLUMN words_tsv",
    "ALTER TABLE text_index ADD COLUMN words_tsv tsvector GENERATED ALWAYS AS      (array_to_tsvector(array_remove(string_to_array(words, ' '), ''))) STORED",
    "CREATE INDEX text_index_words ON text_index USING GIN (words_tsv)",
];

pub(super) fn steps() -> Vec<Step> {
    vec![
        // The rows go while the triggers of the old table still follow them.
        Step::Delete(Query::delete().from_table(TextIndexRow::Table).to_owned()),
        Step::Sql(FixedSql {
            sqlite: SQLITE.to_vec(),
            postgres: POSTGRES.to_vec(),
        }),
        Step::Update(
            Query::update()
                .table(OutboxPosition::Table)
                .value(OutboxPosition::Position, REBUILD_WANTED)
                .and_where(Expr::col(OutboxPosition::Consumer).eq(READER))
                .to_owned(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use bb_app::ports::ChangeStore;
    use bb_app::text_index::{TextIndexStore as _, catch_up};
    use bb_core::event::{EntityKind, RecordKind};
    use bb_core::id::EntityId;
    use bb_core::text_index::TextSource;
    use bb_testing::{TestResult, each_database};
    use sea_query::{Expr, ExprTrait as _, Query};

    use super::super::Database;
    use crate::AuthStorage;
    use crate::anchor::LogSeal;
    use crate::db::StoredId as _;
    use crate::schema::{IssueRow, TextIndexRow};
    use crate::test_support::{at, event, ids, world};
    use crate::text_index::TextScope;
    use crate::text_index::tests::found;

    /// An issue written before the migration has its title in the index with the accent removed
    /// (`muller`). After the migration the index is empty, and after the first round of the
    /// reader (what a start does) the same issue is found by *müller* and no longer by *muller*.
    /// The title stands for every source: all of them are made again by the one function that the
    /// reader uses (`reader.rs` proves each source).
    #[tokio::test]
    async fn m5_f53_migration_25_makes_an_issue_written_before_it_found_by_its_letters_at_start()
    -> TestResult {
        each_database("migration_25_text_in_nfc", async |db| {
            let database = Database::connect(db.url()).await?;
            database.migrate_until(24, at(0)).await?;
            let folder = tempfile::tempdir()?;
            let keys = std::sync::Arc::new(database.open_keys(&folder.path().join("keys")).await?);
            let seal = LogSeal::new(&keys, folder.path().join("head.log"));
            let storage = AuthStorage::new(database.clone(), keys, seal);
            let mut source = ids();
            let world = world(&storage, &mut source, 2).await;
            let (first, second) = (world.issues[0].id, world.issues[1].id);
            // Each issue has the event of its change that the reader follows (the creation of a
            // real issue writes one); the first also gets its new title.
            for (issue, title) in [
                (first, "Frau Müller meldet eine Größe"),
                (second, "Issue 2"),
            ] {
                let mut about = event(&mut source, 6);
                about.entity.kind = EntityKind::from(RecordKind::Issue);
                about.entity.id = issue.into();
                ChangeStore::record_change(&storage, &about, move |tx| {
                    Box::pin(async move {
                        let mut rename = Query::update();
                        rename
                            .table(IssueRow::Table)
                            .value(IssueRow::Title, title)
                            .and_where(Expr::col(IssueRow::Id).eq(issue.stored()));
                        tx.execute(&rename).await?;
                        Ok(())
                    })
                })
                .await?;
            }
            // The reader has come to the end of the events, as a running server's has, and the
            // index is the one the program before this migration wrote: accents removed.
            catch_up(&storage, at(1)).await?;
            database
                .execute(&Query::delete().from_table(TextIndexRow::Table).to_owned())
                .await?;
            for (issue, words) in [
                (first, "frau muller meldet eine grosse"),
                (second, "issue 2"),
            ] {
                let mut row = Query::insert();
                row.into_table(TextIndexRow::Table)
                    .columns([
                        TextIndexRow::Issue,
                        TextIndexRow::Source,
                        TextIndexRow::SourceRef,
                        TextIndexRow::Words,
                    ])
                    .values([
                        Expr::val(issue.stored()),
                        Expr::val(TextSource::Title.code()),
                        Expr::val(EntityId::from(issue).stored()),
                        Expr::val(words),
                    ])?;
                database.execute(&row).await?;
            }
            let everywhere = TextScope::Everywhere;
            assert_eq!(found(&storage, "muller", everywhere).await?, ["1"]);
            assert!(found(&storage, "müller", everywhere).await?.is_empty());
            assert_eq!(
                catch_up(&storage, at(2)).await?,
                0,
                "nothing is left to read"
            );

            assert_eq!(database.migrate(at(3)).await?, [25]);
            // The migration itself prepares nothing: the index is empty, and the reader reads no
            // event until the index has been made again.
            assert!(found(&storage, "muller", everywhere).await?.is_empty());
            assert_eq!(catch_up(&storage, at(4)).await?, 0);
            assert!(found(&storage, "müller", everywhere).await?.is_empty());

            // A start makes the index again, once, and the reader is at the head afterwards.
            assert_eq!(storage.rebuild_text_index_if_wanted(at(5)).await?, Some(2));
            assert_eq!(found(&storage, "müller", everywhere).await?, ["1"]);
            assert_eq!(
                found(&storage, "Mu\u{308}ller größe", everywhere).await?,
                ["1"]
            );
            assert!(found(&storage, "muller", everywhere).await?.is_empty());
            assert!(found(&storage, "grosse", everywhere).await?.is_empty());
            assert_eq!(found(&storage, "issue", everywhere).await?, ["2"]);
            assert_eq!(storage.rebuild_text_index_if_wanted(at(6)).await?, None);
            assert_eq!(
                catch_up(&storage, at(7)).await?,
                0,
                "the reader is at the head"
            );
            Ok(())
        })
        .await
    }
}
