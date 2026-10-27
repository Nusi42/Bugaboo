//! The reader of the outbox that keeps the index, with the real tables and the events the use
//! cases write.

use bb_app::comments::CommentStore as _;
use bb_app::fields::FieldStore as _;
use bb_app::files::AttachmentStore as _;
use bb_app::issues::IssueStore as _;
use bb_app::ports::{BoxFuture, ChangeStore};
use bb_app::text_index::{TextIndexStore as _, Touched, catch_up};
use bb_core::comment::Comment;
use bb_core::event::{EntityKind, RecordKind};
use bb_core::files::{Attachment, MediaType, PreviewStatus, StoredFile};
use bb_core::id::{AttachmentId, CommentId, IdSource, UserId};
use bb_core::own_fields::FieldType;
use bb_core::own_fields_value::FieldValue;
use bb_core::security_log::Digest;
use bb_core::text_index::{CONSUMER, REBUILD_WANTED};
use bb_core::version::FIRST_VERSION;
use bb_core::version::Version;
use std::time::Duration;

use super::*;
use crate::db::{Dialect, Transaction};
use crate::schema::TextIndexRow;
use crate::test_support::{at, definition, event, fixture, ids, world};
use crate::text_index::adapter::move_reader_to;
use crate::text_index::write::reindex_issues;
use crate::{AuthStorage, StoreError};

/// Runs `write` as one change whose event is about the record `id` of this kind, the way a use
/// case does.
async fn changed(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    kind: RecordKind,
    id: EntityId,
    write: impl for<'t> FnOnce(&'t mut Transaction) -> BoxFuture<'t, Result<(), StoreError>> + Send,
) -> Result<(), StoreError> {
    let mut about = event(source, 6);
    about.entity.kind = EntityKind::from(kind);
    about.entity.id = id;
    ChangeStore::record_change(storage, &about, write).await
}

/// The same for an issue.
async fn changed_issue(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    issue: IssueId,
    write: impl for<'t> FnOnce(&'t mut Transaction) -> BoxFuture<'t, Result<(), StoreError>> + Send,
) -> Result<(), StoreError> {
    changed(storage, source, RecordKind::Issue, issue.into(), write).await
}

async fn rows(storage: &AuthStorage) -> Result<Vec<(String, String)>, StoreError> {
    let mut select = Query::select();
    select
        .columns([TextIndexRow::Source, TextIndexRow::Words])
        .from(TextIndexRow::Table)
        .order_by(TextIndexRow::Source, Order::Asc)
        .order_by(TextIndexRow::Words, Order::Asc);
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

/// Sets the value of a long-text field of an issue, as a change of the issue.
async fn set_text(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    issue: IssueId,
    field: FieldId,
    text: &str,
) -> Result<(), StoreError> {
    let cells = FieldValue::Text(text.to_owned()).to_cells();
    changed_issue(storage, source, issue, move |tx| {
        Box::pin(async move {
            AuthStorage::put_field_value(
                tx,
                EntityKind::from(RecordKind::Issue),
                EntityId::from(issue),
                field,
                &cells,
            )
            .await
        })
    })
    .await
}

fn comment(source: &mut impl IdSource, issue: IssueId, text: &str) -> Comment {
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

fn attachment(source: &mut impl IdSource, issue: IssueId, name: &str) -> Attachment {
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

/// A word that is far longer than a real word (a line of Chinese text without a stop, a hash)
/// is not stored, and neither stops the reader nor fails a search (PostgreSQL refuses a lexeme of
/// more than 2046 bytes): the other words of the text are found, and the long word is found
/// nowhere, on both databases.
#[tokio::test]
async fn iss_13_a_word_too_long_for_the_index_does_not_stop_the_reader_or_a_search() -> TestResult {
    each_database("iss_13_long_word", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let world = world(storage, &mut source, 2).await;
        let (first, second) = (world.issues[0].id, world.issues[1].id);
        // 683 characters of three bytes are 2049 bytes; the hex dump is 2047 bytes.
        let chinese = "\u{4e2d}".repeat(683);
        let dump = "ab".repeat(1023) + "c";
        assert_eq!((chinese.len(), dump.len()), (2049, 2047));
        let said = comment(&mut source, first, &format!("kept {chinese} words"));
        changed_issue(storage, &mut source, first, move |tx| {
            Box::pin(async move { AuthStorage::insert_comment(tx, &said).await })
        })
        .await?;
        let named = attachment(&mut source, second, &format!("{dump} report.txt"));
        changed_issue(storage, &mut source, second, move |tx| {
            let file = file();
            Box::pin(async move {
                AuthStorage::insert_stored_file(tx, &file).await?;
                AuthStorage::insert_attachment(tx, &named).await
            })
        })
        .await?;

        // The reader goes through both texts and the move of its place is committed.
        assert!(catch_up(storage, at(9)).await? > 0);
        assert_eq!(catch_up(storage, at(10)).await?, 0);
        let everywhere = TextScope::Everywhere;
        assert_eq!(found(storage, "kept words", everywhere).await?, [hit(1)]);
        assert_eq!(found(storage, "report txt", everywhere).await?, [hit(2)]);
        // The long words are in no row, and a search for one is a clean answer: nothing.
        let left = rows(storage).await?;
        assert!(left.iter().all(|(_, words)| words.len() < 1000), "{left:?}");
        for typed in [
            chinese.as_str(),
            dump.as_str(),
            &format!("{dump}*"),
            &format!("kept {chinese}"),
        ] {
            assert!(found(storage, typed, everywhere).await?.is_empty());
        }
        Ok(())
    })
    .await
}

/// The rebuild takes the head of the outbox first and puts the reader there: a change written after
/// it is read as an event, and one written before it is already in the rows.
#[tokio::test]
async fn iss_13_a_change_after_a_rebuild_is_read_by_the_reader() -> TestResult {
    each_database("iss_13_change_after_rebuild", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let world = world(storage, &mut source, 2).await;
        let (first, second) = (world.issues[0].id, world.issues[1].id);
        let before = comment(&mut source, first, "written before the rebuild");
        changed_issue(storage, &mut source, first, move |tx| {
            Box::pin(async move { AuthStorage::insert_comment(tx, &before).await })
        })
        .await?;
        assert_eq!(storage.rebuild_text_index(at(5)).await?, 2);
        let everywhere = TextScope::Everywhere;
        assert_eq!(found(storage, "rebuild", everywhere).await?, [hit(1)]);
        // The events up to the head are not read again.
        assert_eq!(catch_up(storage, at(6)).await?, 0);

        let after = comment(&mut source, second, "written after the rebuild");
        changed_issue(storage, &mut source, second, move |tx| {
            Box::pin(async move { AuthStorage::insert_comment(tx, &after).await })
        })
        .await?;
        assert_eq!(catch_up(storage, at(7)).await?, 1);
        assert_eq!(
            found(storage, "rebuild", everywhere).await?,
            [hit(2), hit(1)]
        );
        Ok(())
    })
    .await
}

/// The place of the reader only moves on: the mark goes, and a reader that has gone past the head
/// by itself stays where it is.
#[tokio::test]
async fn iss_13_the_reader_is_put_at_the_head_only_forward() -> TestResult {
    each_database("iss_13_reader_forward", async |db| {
        let fixture = fixture(db).await?;
        let database = fixture.storage.database();
        let reader = crate::events::consumer_name(CONSUMER)?;
        let position = async || database.outbox_position(&reader).await;

        // A reader that never read anything gets a place.
        move_reader_to(database, 5, at(1)).await?;
        assert_eq!(position().await?, 5);
        // The mark is cleared whatever the head is.
        database.outbox_set(CONSUMER, REBUILD_WANTED, at(2)).await?;
        move_reader_to(database, 5, at(3)).await?;
        assert_eq!(position().await?, 5);
        // A reader that moved on by itself is left alone, and one behind the head is moved.
        database.outbox_set(CONSUMER, 8, at(4)).await?;
        move_reader_to(database, 5, at(5)).await?;
        assert_eq!(position().await?, 8);
        move_reader_to(database, 11, at(6)).await?;
        assert_eq!(position().await?, 11);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_13_a_change_while_the_reader_was_stopped_is_in_the_index_after_a_start() -> TestResult
{
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
                AuthStorage::update_issue(tx, &issue, Version::new(FIRST_VERSION)).await?;
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
        for words in ["absturz", "STEPS größe", "crashes", "protokoll übersicht*"] {
            assert_eq!(
                found(storage, words, everywhere).await?,
                [hit(1)],
                "{words}"
            );
        }
        // A bare search never reads a long-text field; a search in that field does.
        assert!(found(storage, "printers", everywhere).await?.is_empty());
        assert_eq!(
            found(storage, "printers", TextScope::Field(field_id)).await?,
            [hit(1)]
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
            [hit(1)]
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_13_an_edited_comment_replaces_its_words() -> TestResult {
    each_database("iss_13_reader_comment_edit", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let world = world(storage, &mut source, 1).await;
        let id = world.issues[0].id;
        let said = comment(&mut source, id, "first version of the words");
        let comment_id = said.id;
        changed_issue(storage, &mut source, id, move |tx| {
            Box::pin(async move { AuthStorage::insert_comment(tx, &said).await })
        })
        .await?;
        catch_up(storage, at(9)).await?;
        let everywhere = TextScope::Everywhere;
        assert_eq!(found(storage, "first", everywhere).await?.len(), 1);
        // The event of an edit is about the comment and does not name the issue.
        changed(
            storage,
            &mut source,
            RecordKind::Comment,
            comment_id.into(),
            move |tx| {
                Box::pin(async move {
                    AuthStorage::update_comment_text(
                        tx,
                        comment_id,
                        "second version",
                        at(10),
                        Version::new(FIRST_VERSION),
                    )
                    .await?;
                    Ok(())
                })
            },
        )
        .await?;
        catch_up(storage, at(11)).await?;
        assert!(found(storage, "first", everywhere).await?.is_empty());
        assert_eq!(found(storage, "second", everywhere).await?.len(), 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_13_a_changed_long_text_field_replaces_its_words() -> TestResult {
    each_database("iss_13_reader_field_change", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let world = world(storage, &mut source, 1).await;
        let id = world.issues[0].id;
        let field = definition(&mut source, FieldType::LongText, 1);
        let field_id = field.field.id;
        changed_issue(storage, &mut source, id, move |tx| {
            Box::pin(async move { AuthStorage::insert_field(tx, &field).await })
        })
        .await?;
        set_text(storage, &mut source, id, field_id, "old notes").await?;
        catch_up(storage, at(9)).await?;
        let scope = TextScope::Field(field_id);
        assert_eq!(found(storage, "old", scope).await?.len(), 1);
        set_text(storage, &mut source, id, field_id, "new notes").await?;
        catch_up(storage, at(10)).await?;
        assert!(found(storage, "old", scope).await?.is_empty());
        assert_eq!(found(storage, "new", scope).await?.len(), 1);
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

        // The events of the trash are about the comment and the file.
        changed(
            storage,
            &mut source,
            RecordKind::Comment,
            comment_id.into(),
            move |tx| {
                Box::pin(async move {
                    AuthStorage::set_comment_deleted(tx, comment_id, Some(at(10))).await?;
                    Ok(())
                })
            },
        )
        .await?;
        changed(
            storage,
            &mut source,
            RecordKind::Attachment,
            file_id.into(),
            move |tx| {
                Box::pin(async move {
                    AuthStorage::set_attachment_deleted(tx, file_id, Some(at(10))).await?;
                    Ok(())
                })
            },
        )
        .await?;
        catch_up(storage, at(11)).await?;
        assert!(found(storage, "soon", everywhere).await?.is_empty());
        assert!(found(storage, "evidence", everywhere).await?.is_empty());

        changed(
            storage,
            &mut source,
            RecordKind::Comment,
            comment_id.into(),
            move |tx| {
                Box::pin(async move {
                    AuthStorage::set_comment_deleted(tx, comment_id, None).await?;
                    Ok(())
                })
            },
        )
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
async fn iss_13_a_batch_of_comment_edits_costs_the_same_statements_whatever_its_size() -> TestResult
{
    each_database("iss_13_reader_statements", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let world = world(storage, &mut source, 1).await;
        let id = world.issues[0].id;
        let mut counts = Vec::new();
        for edits in [3_usize, 30] {
            // Comments the index knows, then one edit event for each, then one run of the reader.
            let made: Vec<Comment> = (0..edits)
                .map(|n| comment(&mut source, id, &format!("before {n}")))
                .collect();
            let written = made.clone();
            changed_issue(storage, &mut source, id, move |tx| {
                Box::pin(async move {
                    for one in &written {
                        AuthStorage::insert_comment(tx, one).await?;
                    }
                    Ok(())
                })
            })
            .await?;
            catch_up(storage, at(9)).await?;
            for one in &made {
                let comment_id = one.id;
                changed(
                    storage,
                    &mut source,
                    RecordKind::Comment,
                    comment_id.into(),
                    move |tx| {
                        Box::pin(async move {
                            AuthStorage::update_comment_text(
                                tx,
                                comment_id,
                                "after",
                                at(10),
                                Version::new(FIRST_VERSION),
                            )
                            .await?;
                            Ok(())
                        })
                    },
                )
                .await?;
            }
            let before = storage.database().statements_run();
            catch_up(storage, at(11)).await?;
            counts.push(storage.database().statements_run() - before);
        }
        assert_eq!(counts[0], counts[1], "statements for 3 and for 30 edits");
        assert_eq!(
            found(storage, "after", TextScope::Everywhere).await?.len(),
            1
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_13_readers_at_the_same_moment_leave_the_index_as_the_stored_texts_say() -> TestResult {
    each_database("iss_13_reader_interleaved", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let world = world(storage, &mut source, 3).await;
        let issues: Vec<IssueId> = world.issues.iter().map(|made| made.id).collect();
        for round in 0..6_i64 {
            // A change after the reading of an older event: the newest text must win, however
            // the readers interleave (two consumers, and the rebuild).
            let texts: Vec<Comment> = issues
                .iter()
                .map(|issue| comment(&mut source, *issue, &format!("round {round} kept")))
                .collect();
            for one in texts {
                let issue = one.issue;
                changed_issue(storage, &mut source, issue, move |tx| {
                    Box::pin(async move { AuthStorage::insert_comment(tx, &one).await })
                })
                .await?;
            }
            let (first, second, third) = tokio::join!(
                catch_up(storage, at(20 + round)),
                catch_up(storage, at(20 + round)),
                storage.rebuild_text_index(at(0))
            );
            // One of two writers may be refused by a rule of the database; the next round of the
            // reader makes it good, so only the end state counts.
            let _ = (first, second, third);
            // Trash every comment of the round, then let the readers go on.
            let trash: Vec<CommentId> = {
                let mut select = Query::select();
                select
                    .column(crate::schema::CommentRow::Id)
                    .from(crate::schema::CommentRow::Table);
                let ids = storage.database().fetch_all(&select).await?;
                ids.iter()
                    .map(|row| row.id::<CommentId>(crate::schema::CommentRow::Id))
                    .collect::<Result<_, _>>()?
            };
            for one in trash {
                changed(
                    storage,
                    &mut source,
                    RecordKind::Comment,
                    one.into(),
                    move |tx| {
                        Box::pin(async move {
                            AuthStorage::set_comment_deleted(tx, one, Some(at(30))).await?;
                            Ok(())
                        })
                    },
                )
                .await?;
            }
            let (first, second) = tokio::join!(
                catch_up(storage, at(40 + round)),
                storage.rebuild_text_index(at(0))
            );
            let _ = (first, second);
            catch_up(storage, at(50 + round)).await?;
        }
        // Everything is in the trash: no word of a comment is left.
        assert!(
            found(storage, "kept", TextScope::Everywhere)
                .await?
                .is_empty()
        );
        let left = rows(storage).await?;
        assert!(left.iter().all(|(source, _)| source == "title"), "{left:?}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_13_a_second_writer_waits_for_the_first_and_then_reads_the_newest_texts() -> TestResult
{
    each_database("iss_13_writer_waits", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        // SQLite has one writer at a time: the second waits for the whole transaction there,
        // and the change in the middle of this test could not be made. PostgreSQL has many.
        if storage.database().dialect() != Dialect::Postgres {
            return Ok(());
        }
        let mut source = ids();
        let world = world(storage, &mut source, 1).await;
        let id = world.issues[0].id;
        let said = comment(&mut source, id, "kept in the index until the end");
        let comment_id = said.id;
        changed_issue(storage, &mut source, id, move |tx| {
            Box::pin(async move { AuthStorage::insert_comment(tx, &said).await })
        })
        .await?;

        // A reads the texts (the comment is there) and holds the lock until it commits.
        let mut first = storage.database().begin().await?;
        reindex_issues(&mut first, &[id]).await?;
        // The comment goes to the trash after A has read it.
        changed(
            storage,
            &mut source,
            RecordKind::Comment,
            comment_id.into(),
            move |tx| {
                Box::pin(async move {
                    AuthStorage::set_comment_deleted(tx, comment_id, Some(at(10))).await?;
                    Ok(())
                })
            },
        )
        .await?;
        // B must wait for A at the lock. Without it B would read the old texts now, and its rows
        // would clash with A's or leave A's comment behind; the checks after the commit tell.
        let touched = [Touched::Issue(id)];
        let mut second = Box::pin(storage.store_text_index(&touched, 0, at(11)));
        let waited = tokio::time::timeout(Duration::from_millis(500), second.as_mut()).await;
        assert!(
            waited.is_err(),
            "the second writer did not wait for the first"
        );
        first.commit().await?;
        second.await?;
        // What B read after A committed is what the index holds: without the comment.
        assert!(
            found(storage, "kept", TextScope::Everywhere)
                .await?
                .is_empty()
        );
        let left = rows(storage).await?;
        assert!(left.iter().all(|(source, _)| source == "title"), "{left:?}");
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
        assert_eq!(storage.rebuild_text_index(at(0)).await?, 3);
        assert_eq!(
            found(storage, "issue", TextScope::Everywhere).await?.len(),
            world.issues.len()
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_13_a_word_with_a_combining_mark_is_split_alike_on_both_databases() -> TestResult {
    each_database("iss_13_combining_mark", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let world = world(storage, &mut source, 1).await;
        let id = world.issues[0].id;
        // A Hindi word whose vowel signs are combining marks, an Arabic word with a mark, and a
        // plain word after them.
        let mut tx = storage.database().begin().await?;
        index_text(&mut tx, id, TextSource::Title, id.into(), "हिन्दी كِتاب done").await?;
        tx.commit().await?;
        let everywhere = TextScope::Everywhere;
        for words in ["हिन्दी", "हि*", "كِتاب", "done"] {
            assert_eq!(
                found(storage, words, everywhere).await?,
                [hit(1)],
                "{words}"
            );
        }
        Ok(())
    })
    .await
}
