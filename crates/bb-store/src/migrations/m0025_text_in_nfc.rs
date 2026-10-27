//! Migration 25: the words of the full-text search keep their letters (M5-F53, ADR-0016,
//! amendment M5). Until now a text was prepared with its accents removed (*Müller* was stored as
//! `muller`, `ß` as `ss`); now it is lower case and Unicode NFC and nothing else (*Müller* is
//! stored as `müller`). The rows of the index are made again from the stored texts, so an issue
//! written before this migration is found by *müller* after it.
//!
//! The table and its FTS5 or `tsvector` index stay as they are: SQLite's `unicode61` tokenizer is
//! set to `remove_diacritics 0`, and PostgreSQL's `simple` configuration only lower-cases, so
//! both keep a word as it was stored. The triggers of SQLite and the generated column of
//! PostgreSQL follow the rows that this step writes.

use super::Step;

pub(super) fn steps() -> Vec<Step> {
    vec![Step::RebuildTextIndex]
}

#[cfg(test)]
mod tests {
    use bb_core::id::EntityId;
    use bb_core::text_index::TextSource;
    use bb_core::text_words::search_words;
    use bb_testing::{TestResult, each_database};
    use sea_query::{Expr, ExprTrait as _, Query};

    use super::super::Database;
    use crate::AuthStorage;
    use crate::anchor::LogSeal;
    use crate::db::StoredId as _;
    use crate::schema::{IssueRow, TextIndexRow};
    use crate::test_support::{at, ids, world};
    use crate::text_index::{TextScope, TextSearch, text_issues};

    /// The codes of the issues that a search for `input` finds.
    async fn found(storage: &AuthStorage, input: &str) -> Result<Vec<String>, crate::StoreError> {
        let database = storage.database();
        let words = search_words(input);
        let matched = text_issues(
            database.dialect(),
            &TextSearch {
                words: &words,
                scope: TextScope::Everywhere,
            },
        );
        let mut select = Query::select();
        select
            .column(IssueRow::Code)
            .from(IssueRow::Table)
            .and_where(Expr::col(IssueRow::Id).in_subquery(matched));
        database
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| row.get::<String>(IssueRow::Code))
            .collect()
    }

    /// An issue written before the migration has its title in the index with the accent removed
    /// (`muller`); after it the same issue is found by *müller*, and no longer by *muller*.
    #[tokio::test]
    async fn m5_f53_migration_25_makes_an_issue_written_before_it_found_by_its_letters()
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
            let mut rename = Query::update();
            rename
                .table(IssueRow::Table)
                .value(IssueRow::Title, "Frau Müller meldet eine Größe")
                .and_where(Expr::col(IssueRow::Id).eq(first.stored()));
            database.execute(&rename).await?;
            // The index as the program before this migration wrote it.
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
            assert_eq!(found(&storage, "muller").await?, ["1"]);
            assert!(found(&storage, "müller").await?.is_empty());

            assert_eq!(database.migrate(at(1)).await?, [25]);

            assert_eq!(found(&storage, "müller").await?, ["1"]);
            assert_eq!(found(&storage, "Mu\u{308}ller größe").await?, ["1"]);
            assert!(found(&storage, "muller").await?.is_empty());
            assert!(found(&storage, "grosse").await?.is_empty());
            assert_eq!(found(&storage, "issue").await?, ["2"]);
            Ok(())
        })
        .await
    }
}
