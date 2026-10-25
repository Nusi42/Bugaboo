//! The port and the index on both databases: the same words find the same issues in the same
//! order (ARCH-DB-PARITY).

use bb_core::id::{EntityId, FieldId, IssueId};
use bb_core::text_index::TextSource;
use bb_core::text_words::search_words;
use bb_testing::{TestResult, each_database};
use sea_query::{Expr, ExprTrait as _, Order, Query};

use super::{TextMatch, TextScope, TextSearch, index_text, text_matches};
use crate::schema::IssueRow;
use crate::test_support::{fixture, ids, world};

/// An expected hit: the code of the issue and whether its title matched.
fn hit(number: i64, title: i64) -> (String, i64) {
    (number.to_string(), title)
}

/// The issues that a search finds, newest first within the same title hit (the order the compiler
/// of the search will ask for), as their numbers.
async fn found(
    storage: &crate::AuthStorage,
    input: &str,
    scope: TextScope,
) -> Result<Vec<(String, i64)>, Box<dyn std::error::Error + Send + Sync>> {
    let database = storage.database();
    let words = search_words(input);
    let matched = text_matches(
        database.dialect(),
        &TextSearch {
            words: &words,
            scope,
        },
    );
    let mut select = Query::select();
    select
        .column((IssueRow::Table, IssueRow::Code))
        .expr(Expr::col(("text_hit", TextMatch::TITLE_HIT)))
        .from(IssueRow::Table)
        .join_subquery(
            sea_query::JoinType::InnerJoin,
            matched.select,
            "text_hit",
            Expr::col((IssueRow::Table, IssueRow::Id)).equals(("text_hit", TextMatch::ISSUE)),
        )
        .order_by(("text_hit", TextMatch::TITLE_HIT), Order::Desc)
        .order_by((IssueRow::Table, IssueRow::CreatedAt), Order::Desc)
        .order_by((IssueRow::Table, IssueRow::Id), Order::Desc);
    let rows = database.fetch_all(&select).await?;
    rows.iter()
        .map(|row| {
            Ok((
                row.get::<String>(IssueRow::Code)?,
                row.get::<i64>(TextMatch::TITLE_HIT)?,
            ))
        })
        .collect()
}

#[tokio::test]
async fn iss_13_the_port_finds_each_source_and_orders_title_hits_first() -> TestResult {
    each_database("iss_13_port_sources", async |db| {
        let fixture = fixture(db).await?;
        let mut source = ids();
        let world = world(&fixture.storage, &mut source, 5).await;
        let issue: Vec<IssueId> = world.issues.iter().map(|made| made.id).collect();
        let mut tx = fixture.storage.database().begin().await?;
        let comment = EntityId::from(IssueId::generate(&mut source)?);
        index_text(
            &mut tx,
            issue[0],
            TextSource::Title,
            issue[0].into(),
            "Crash on save",
        )
        .await?;
        index_text(
            &mut tx,
            issue[1],
            TextSource::Comment,
            comment,
            "It CRASHES, then crash on SAVE",
        )
        .await?;
        index_text(
            &mut tx,
            issue[2],
            TextSource::Description,
            issue[2].into(),
            "Absturz beim Speichern: Größe",
        )
        .await?;
        index_text(
            &mut tx,
            issue[3],
            TextSource::File,
            comment,
            "crash-log.txt",
        )
        .await?;
        index_text(
            &mut tx,
            issue[4],
            TextSource::Field,
            comment,
            "crash on save in issue[0] field",
        )
        .await?;
        tx.commit().await?;

        let storage = &fixture.storage;
        // Title hits first, then the others, newest first.
        assert_eq!(
            found(storage, "crash on save", TextScope::Everywhere).await?,
            [hit(1, 1), hit(2, 0)]
        );
        // `crash` is not `crashes`, but `crash*` finds both.
        assert_eq!(
            found(storage, "crashes", TextScope::Everywhere).await?,
            [hit(2, 0)]
        );
        assert_eq!(
            found(storage, "crash*", TextScope::Everywhere).await?,
            [hit(1, 1), hit(4, 0), hit(2, 0)]
        );
        // Accents and case do not matter; ß is ss.
        assert_eq!(
            found(storage, "GROSSE größe", TextScope::Everywhere).await?,
            [hit(3, 0)]
        );
        // A bare search never reads an own field.
        assert!(
            found(storage, "field", TextScope::Everywhere)
                .await?
                .is_empty()
        );
        // No words find nothing.
        assert!(
            found(storage, " ... ", TextScope::Everywhere)
                .await?
                .is_empty()
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_13_a_search_in_one_field_reads_the_rows_of_that_field_only() -> TestResult {
    each_database("iss_13_port_field", async |db| {
        let fixture = fixture(db).await?;
        let mut source = ids();
        let world = world(&fixture.storage, &mut source, 3).await;
        let issue: Vec<IssueId> = world.issues.iter().map(|made| made.id).collect();
        let (wanted, other) = (
            FieldId::generate(&mut source)?,
            FieldId::generate(&mut source)?,
        );
        let mut tx = fixture.storage.database().begin().await?;
        index_text(
            &mut tx,
            issue[0],
            TextSource::Field,
            wanted.into(),
            "steps to reproduce",
        )
        .await?;
        index_text(
            &mut tx,
            issue[1],
            TextSource::Field,
            other.into(),
            "steps to reproduce",
        )
        .await?;
        index_text(
            &mut tx,
            issue[2],
            TextSource::Title,
            issue[2].into(),
            "steps to reproduce",
        )
        .await?;
        tx.commit().await?;
        assert_eq!(
            found(
                &fixture.storage,
                "steps reproduce",
                TextScope::Field(wanted)
            )
            .await?,
            [hit(1, 0)]
        );
        Ok(())
    })
    .await
}

mod reader {
    //! The reader of the outbox that keeps the index, with the real tables.

    use bb_app::comments::CommentStore as _;
    use bb_app::fields::FieldStore as _;
    use bb_app::files::AttachmentStore as _;
    use bb_app::issues::IssueStore as _;
    use bb_app::ports::{BoxFuture, ChangeStore};
    use bb_app::text_index::{TextIndexStore as _, Touched, catch_up};
    use bb_core::comment::Comment;
    use bb_core::event::{EntityKind, RecordKind};
    use bb_core::files::{Attachment, MediaType, PreviewStatus, StoredFile};
    use bb_core::id::{AttachmentId, CommentId, UserId};
    use bb_core::own_fields::FieldType;
    use bb_core::own_fields_value::FieldValue;
    use bb_core::security_log::Digest;
    use bb_core::version::FIRST_VERSION;

    use super::*;
    use crate::db::Transaction;
    use crate::schema::TextIndexRow;
    use crate::test_support::{at, definition, event, fixture, ids, world};
    use crate::{AuthStorage, StoreError};

    /// Runs `write` as one change whose event is about `issue`, the way a use case of an issue
    /// does, so the reader knows which issue to read again.
    async fn changed_issue(
        storage: &AuthStorage,
        source: &mut impl bb_core::id::IdSource,
        issue: IssueId,
        write: impl for<'t> FnOnce(&'t mut Transaction) -> BoxFuture<'t, Result<(), StoreError>> + Send,
    ) -> Result<(), StoreError> {
        let mut about = event(source, 6);
        about.entity.kind = EntityKind::from(RecordKind::Issue);
        about.entity.id = EntityId::from(issue);
        ChangeStore::record_change(storage, &about, write).await
    }

    async fn rows(storage: &AuthStorage) -> Result<Vec<(String, String)>, StoreError> {
        let mut select = Query::select();
        select
            .columns([TextIndexRow::Source, TextIndexRow::Words])
            .from(TextIndexRow::Table)
            .order_by(TextIndexRow::Id, Order::Asc);
        storage
            .database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| {
                Ok((
                    row.get(TextIndexRow::Source)?,
                    row.get(TextIndexRow::Words)?,
                ))
            })
            .collect()
    }

    fn comment(source: &mut impl bb_core::id::IdSource, issue: IssueId, text: &str) -> Comment {
        Comment {
            id: CommentId::generate(source).unwrap(),
            issue,
            author: UserId::generate(source),
            text: text.to_owned(),
            created_at: at(2),
            edited_at: None,
            deleted_at: None,
            version: FIRST_VERSION,
        }
    }

    fn file() -> StoredFile {
        StoredFile {
            hash: Digest::new([7; 32]),
            size: 10,
            media_type: MediaType::new("image/png").unwrap(),
            preview: PreviewStatus::Pending,
            created_at: at(1),
        }
    }

    fn attachment(
        source: &mut impl bb_core::id::IdSource,
        issue: IssueId,
        name: &str,
    ) -> Attachment {
        Attachment {
            id: AttachmentId::generate(source).unwrap(),
            issue,
            comment: None,
            file: file().hash,
            name: name.to_owned(),
            added_by: UserId::generate(source),
            added_at: at(2),
            deleted_at: None,
            preview: PreviewStatus::Pending,
            version: FIRST_VERSION,
        }
    }

    #[tokio::test]
    async fn iss_13_a_change_while_the_reader_was_stopped_is_in_the_index_after_a_start()
    -> TestResult {
        each_database("iss_13_reader_catches_up", async |db| {
            let fixture = fixture(db).await?;
            let storage = &fixture.storage;
            let mut source = ids();
            let world = world(storage, &mut source, 2).await;
            let mut issue = world.issues[0].clone();
            let id = issue.id;
            issue.title = "Absturz beim Speichern".to_owned();
            issue.description = "Steps: open the Größe dialog".to_owned();
            let (said, kept) = (comment(&mut source, id, "It CRASHES at once"), file());
            let (named, field) = (
                attachment(&mut source, id, "Protokoll-Übersicht.pdf"),
                definition(&mut source, FieldType::LongText, 1),
            );
            let field_id = field.field.id;
            let cells = FieldValue::Text("hidden-notes about printers".to_owned()).to_cells();
            // The server is stopped: only the changes are written, no reader runs.
            changed_issue(storage, &mut source, id, move |tx| {
                Box::pin(async move {
                    AuthStorage::update_issue(tx, &issue, FIRST_VERSION).await?;
                    AuthStorage::insert_comment(tx, &said).await?;
                    AuthStorage::insert_stored_file(tx, &kept).await?;
                    AuthStorage::insert_attachment(tx, &named).await?;
                    AuthStorage::insert_field(tx, &field).await?;
                    AuthStorage::put_field_value(
                        tx,
                        EntityKind::from(RecordKind::Issue),
                        EntityId::from(id),
                        field_id,
                        &cells,
                    )
                    .await?;
                    Ok(())
                })
            })
            .await?;
            assert!(rows(storage).await?.is_empty());
            let everywhere = TextScope::Everywhere;
            assert!(found(storage, "crashes", everywhere).await?.is_empty());

            // A start: the reader reads what it has not read.
            assert!(catch_up(storage, at(9)).await? > 0);
            for words in ["absturz", "STEPS grosse", "crashes", "protokoll ubersicht*"] {
                assert_eq!(
                    found(storage, words, everywhere).await?,
                    [hit(1, i64::from(words == "absturz"))],
                    "{words}"
                );
            }
            // A bare search never reads a long-text field; a search in that field does.
            assert!(found(storage, "printers", everywhere).await?.is_empty());
            assert_eq!(
                found(storage, "printers", TextScope::Field(field_id)).await?,
                [hit(1, 0)]
            );
            assert_eq!(catch_up(storage, at(10)).await?, 0);
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn iss_13_reading_an_event_twice_changes_nothing() -> TestResult {
        each_database("iss_13_reader_twice", async |db| {
            let fixture = fixture(db).await?;
            let storage = &fixture.storage;
            let mut source = ids();
            let world = world(storage, &mut source, 1).await;
            let id = world.issues[0].id;
            let said = comment(&mut source, id, "twice is the same");
            let comment_id = said.id;
            changed_issue(storage, &mut source, id, move |tx| {
                Box::pin(async move { AuthStorage::insert_comment(tx, &said).await })
            })
            .await?;
            catch_up(storage, at(9)).await?;
            let once = rows(storage).await?;
            assert!(!once.is_empty());
            // The same events again, as after a restart that lost the place, and a comment named
            // on its own.
            let again = [Touched::Issue(id), Touched::Comment(comment_id)];
            storage.store_text_index(&again, 0, at(10)).await?;
            storage.store_text_index(&again, 0, at(11)).await?;
            assert_eq!(rows(storage).await?, once);
            assert_eq!(
                found(storage, "twice same", TextScope::Everywhere).await?,
                [hit(1, 0)]
            );
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn iss_13_trashed_texts_leave_the_index_and_a_restored_one_comes_back() -> TestResult {
        each_database("iss_13_reader_trash", async |db| {
            let fixture = fixture(db).await?;
            let storage = &fixture.storage;
            let mut source = ids();
            let world = world(storage, &mut source, 1).await;
            let id = world.issues[0].id;
            let said = comment(&mut source, id, "gone soon");
            let comment_id = said.id;
            let (kept, named) = (file(), attachment(&mut source, id, "evidence.txt"));
            let file_id = named.id;
            changed_issue(storage, &mut source, id, move |tx| {
                Box::pin(async move {
                    AuthStorage::insert_comment(tx, &said).await?;
                    AuthStorage::insert_stored_file(tx, &kept).await?;
                    AuthStorage::insert_attachment(tx, &named).await?;
                    Ok(())
                })
            })
            .await?;
            catch_up(storage, at(9)).await?;
            let everywhere = TextScope::Everywhere;
            assert_eq!(found(storage, "soon", everywhere).await?.len(), 1);
            assert_eq!(found(storage, "evidence", everywhere).await?.len(), 1);

            changed_issue(storage, &mut source, id, move |tx| {
                Box::pin(async move {
                    AuthStorage::set_comment_deleted(tx, comment_id, Some(at(10))).await?;
                    AuthStorage::set_attachment_deleted(tx, file_id, Some(at(10))).await?;
                    Ok(())
                })
            })
            .await?;
            catch_up(storage, at(11)).await?;
            assert!(found(storage, "soon", everywhere).await?.is_empty());
            assert!(found(storage, "evidence", everywhere).await?.is_empty());

            changed_issue(storage, &mut source, id, move |tx| {
                Box::pin(async move {
                    AuthStorage::set_comment_deleted(tx, comment_id, None).await?;
                    Ok(())
                })
            })
            .await?;
            catch_up(storage, at(12)).await?;
            assert_eq!(found(storage, "soon", everywhere).await?.len(), 1);

            // The whole issue in the trash leaves the index with everything it had.
            changed_issue(storage, &mut source, id, move |tx| {
                Box::pin(async move {
                    AuthStorage::set_issue_deleted(tx, id, Some(at(13)), at(13)).await?;
                    Ok(())
                })
            })
            .await?;
            catch_up(storage, at(14)).await?;
            assert!(rows(storage).await?.is_empty());
            changed_issue(storage, &mut source, id, move |tx| {
                Box::pin(async move {
                    AuthStorage::set_issue_deleted(tx, id, None, at(15)).await?;
                    Ok(())
                })
            })
            .await?;
            catch_up(storage, at(16)).await?;
            assert_eq!(found(storage, "soon", everywhere).await?.len(), 1);
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn iss_13_a_rebuild_makes_the_index_from_the_stored_texts() -> TestResult {
        each_database("iss_13_rebuild", async |db| {
            let fixture = fixture(db).await?;
            let storage = &fixture.storage;
            let mut source = ids();
            let world = world(storage, &mut source, 3).await;
            // Nothing was ever indexed (a database from before the index): the rebuild reads
            // every issue, and the title of each is found.
            assert!(rows(storage).await?.is_empty());
            assert_eq!(storage.rebuild_text_index().await?, 3);
            assert_eq!(
                found(storage, "issue", TextScope::Everywhere).await?.len(),
                world.issues.len()
            );
            Ok(())
        })
        .await
    }
}
