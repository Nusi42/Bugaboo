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
//! reads what came after the head. The name of the reader and the mark are the ones of `bb_core`,
//! and so is the number of this migration (`RULE_MIGRATION`, which the list of migrations takes as
//! its number), so a tool that keeps an index of its own can compare its marker with it.
//!
//! The databases only store and compare the words Rust prepared: the FTS5 table is made again with
//! the `ascii` tokenizer, which splits at the spaces between the words and nowhere else (a word
//! holds only letters, numbers and marks, and no byte of a letter that is not ASCII is a
//! separator or is folded: no accent is removed, `ς` stays `ς`), and the `tsvector` of PostgreSQL
//! is made from the words split at spaces, not by the parser of the text search, which depends on
//! the locale of the database.
//!
//! The triggers and the FTS5 table (SQLite), the index and the column (PostgreSQL) are dropped
//! before the rows are deleted and made again after, so the delete does not follow each of the
//! rows into the search structure while it holds the lock of the migration (7 seconds for 300 000
//! rows through the triggers, 0.2 seconds without). The statements of the triggers and of the
//! index are the ones of migration 22.

use bb_core::text_index::{CONSUMER, REBUILD_WANTED};
use sea_query::{Expr, ExprTrait as _, Query};

use super::Step;
use super::m0022_text_index::{DROP_FTS, DROP_TSVECTOR, FTS_TRIGGERS, TSVECTOR_INDEX};
use crate::db::FixedSql;
use crate::schema::{OutboxPosition, TextIndexRow};

/// The FTS5 table made again (SQLite): external content, the tokenizer that splits at spaces.
const FTS_TABLE: &str = "CREATE VIRTUAL TABLE text_index_fts USING fts5(words, content='text_index', \
     content_rowid='id', prefix='2 3', tokenize='ascii')";

/// The vector made from the stored words split at spaces, with no parser of the locale
/// (PostgreSQL). Its lexemes are the words as they are stored.
const TSVECTOR_COLUMN: &str = "ALTER TABLE text_index ADD COLUMN words_tsv tsvector GENERATED ALWAYS AS \
     (array_to_tsvector(array_remove(string_to_array(words, ' '), ''))) STORED";

pub(super) fn steps() -> Vec<Step> {
    vec![
        Step::Sql(FixedSql {
            sqlite: DROP_FTS.to_vec(),
            postgres: DROP_TSVECTOR.to_vec(),
        }),
        Step::Delete(Query::delete().from_table(TextIndexRow::Table).to_owned()),
        Step::Sql(FixedSql {
            sqlite: [FTS_TABLE].into_iter().chain(FTS_TRIGGERS).collect(),
            postgres: vec![TSVECTOR_COLUMN, TSVECTOR_INDEX],
        }),
        Step::Update(
            Query::update()
                .table(OutboxPosition::Table)
                .value(OutboxPosition::Position, REBUILD_WANTED)
                .and_where(Expr::col(OutboxPosition::Consumer).eq(CONSUMER))
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
    use bb_core::text_index::{RULE_MIGRATION, TextSource};
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
            database.migrate_until(RULE_MIGRATION - 1, at(0)).await?;
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

            assert_eq!(database.migrate(at(3)).await?, [RULE_MIGRATION]);
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
