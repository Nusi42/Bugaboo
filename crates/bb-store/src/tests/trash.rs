//! The purge of the trash beside a person who takes an issue out of it, on both databases (PRIV-05,
//! ADR-0045): the purge chooses and holds its issues in one statement that checks again, on the row
//! as it is when the purge gets it, that the issue is still in the trash.

use std::time::Duration;

use bb_app::issues::IssueStore as _;
use bb_app::ports::ChangeStore;
use bb_testing::{TestResult, each_database};
use sea_query::{Alias, Expr, ExprTrait as _, Query};

use super::*;
use crate::AuthStorage;
use crate::db::Dialect;
use crate::schema::Computed;
use crate::test_support::{at, event, fixture, ids, person, world};

/// How often the test looks whether the purge waits for the lock of the restore.
const LOOK_EVERY: Duration = Duration::from_millis(10);

/// How many sessions of PostgreSQL on the database of the test wait for a lock on a row: the
/// databases of other tests that run at the same moment do not count.
async fn waiting_for_a_lock(database: &Database) -> Result<i64, StoreError> {
    let mut select = Query::select();
    select
        .expr_as(Expr::cust("COUNT(*)"), Computed::Total)
        .from(Alias::new("pg_stat_activity"))
        .and_where(Expr::col(Alias::new("wait_event_type")).eq("Lock"))
        .and_where(Expr::col(Alias::new("datname")).eq(Expr::cust("current_database()")));
    match database.fetch_optional(&select).await? {
        Some(row) => row.get(Computed::Total),
        None => Ok(0),
    }
}

#[tokio::test]
async fn priv_05_an_issue_taken_out_of_the_trash_while_the_purge_waits_is_kept() -> TestResult {
    each_database("priv_05_trash_restore_race", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let world = world(&storage, &mut source, 1).await;
        let issue = world.issues[0].id;
        ChangeStore::record_change(&storage, &event(&mut source, 5), move |tx| {
            Box::pin(async move {
                AuthStorage::set_issue_deleted(tx, issue, Some(at(40)), at(40), None).await
            })
        })
        .await?;
        // A person takes the issue out of the trash; the change is not committed yet.
        let mut restoring = storage.database().begin().await?;
        assert!(AuthStorage::set_issue_deleted(&mut restoring, issue, None, at(50), None).await?);
        let purging = {
            let storage = storage.clone();
            tokio::spawn(async move { storage.purge_trashed_issues(at(100), 10).await })
        };
        // On PostgreSQL the purge has chosen the issue and waits for the lock of the restore. SQLite
        // has one writer at a time: the purge cannot start before the restore is committed.
        if storage.database().dialect() == Dialect::Postgres {
            while waiting_for_a_lock(storage.database()).await? == 0 {
                assert!(
                    !purging.is_finished(),
                    "the purge finished without waiting for the lock of the restore"
                );
                tokio::time::sleep(LOOK_EVERY).await;
            }
        }
        restoring.commit().await?;
        assert_eq!(purging.await??, 0, "the restored issue was purged");
        let kept = storage.issue(issue).await?.ok_or("the issue went")?;
        assert!(kept.deleted_at.is_none());
        Ok(())
    })
    .await
}

/// The rows of `table` where `column` holds `id`.
async fn rows_naming(
    storage: &AuthStorage,
    table: impl IntoTableRef,
    column: impl IntoColumnRef,
    id: Vec<u8>,
) -> Result<i64, StoreError> {
    let mut count = Query::select();
    count
        .expr_as(Expr::cust("COUNT(*)"), Computed::Total)
        .from(table)
        .and_where(Expr::col(column).eq(id));
    storage
        .database()
        .fetch_optional(&count)
        .await?
        .ok_or(StoreError::Corrupt("a count"))?
        .get(Computed::Total)
}

/// Writes `insert` in the transaction.
async fn written(
    tx: &mut Transaction,
    insert: Result<&mut sea_query::InsertStatement, sea_query::error::Error>,
) -> Result<(), StoreError> {
    let insert = insert.map_err(|_| StoreError::Corrupt("a row of the test"))?;
    tx.execute(&*insert).await.map(drop)
}

/// What hangs under a comment besides its events and its files: an earlier text, a mention, the
/// mark of its text and a notification about it.
async fn under_the_comment(
    tx: &mut Transaction,
    comment: Vec<u8>,
    person: Vec<u8>,
    notification: Vec<u8>,
    event: Vec<u8>,
) -> Result<(), StoreError> {
    use crate::schema::{CommentVersionRow, MentionRow, NotificationRow, TextMarkRow};
    written(
        tx,
        Query::insert()
            .into_table(CommentVersionRow::Table)
            .columns([
                CommentVersionRow::Comment,
                CommentVersionRow::Version,
                CommentVersionRow::Text,
                CommentVersionRow::SavedAt,
            ])
            .values([
                Expr::val(comment.clone()),
                Expr::val(1),
                Expr::val(THROWN_TEXT),
                Expr::val(10),
            ]),
    )
    .await?;
    written(
        tx,
        Query::insert()
            .into_table(MentionRow::Table)
            .columns([MentionRow::Comment, MentionRow::Recipient, MentionRow::Via])
            .values([
                Expr::val(comment.clone()),
                Expr::val(person.clone()),
                Expr::val("person"),
            ]),
    )
    .await?;
    written(
        tx,
        Query::insert()
            .into_table(TextMarkRow::Table)
            .columns([
                TextMarkRow::EntityKind,
                TextMarkRow::EntityId,
                TextMarkRow::Part,
                TextMarkRow::WrittenBy,
            ])
            .values([
                Expr::val(RecordKind::Comment.code()),
                Expr::val(comment.clone()),
                Expr::val("text"),
                Expr::val(bb_core::mark::WrittenBy::Agent.code()),
            ]),
    )
    .await?;
    written(
        tx,
        Query::insert()
            .into_table(NotificationRow::Table)
            .columns([
                NotificationRow::Id,
                NotificationRow::Recipient,
                NotificationRow::Kind,
                NotificationRow::Event,
                NotificationRow::Once,
                NotificationRow::Record,
                NotificationRow::At,
                NotificationRow::Changes,
                NotificationRow::InApp,
                NotificationRow::Mail,
            ])
            .values([
                Expr::val(notification),
                Expr::val(person),
                Expr::val("mentioned"),
                Expr::val(event),
                Expr::val("once"),
                Expr::val(comment),
                Expr::val(10),
                Expr::val("[]"),
                Expr::val(true),
                Expr::val("none"),
            ]),
    )
    .await
}

/// Asserts that no row names the comment or its event any more: no event, no entry of the outbox
/// and no row of the history, and nothing that hung under the comment.
async fn nothing_left(
    storage: &AuthStorage,
    comment_bytes: Vec<u8>,
    event_bytes: Vec<u8>,
) -> Result<(), StoreError> {
    use crate::schema::{
        AttachmentRow, CommentVersionRow, HistoryEntryRow, MentionRow, NotificationRow, Outbox,
        TextMarkRow, UploadRow,
    };
    let of_comment = |table: TableRef, column: ColumnRef| {
        rows_naming(storage, table, column, comment_bytes.clone())
    };
    let left = [
        (
            of_comment(
                crate::schema::Event::Table.into_table_ref(),
                crate::schema::Event::EntityId.into_column_ref(),
            )
            .await?,
            "an event",
        ),
        (
            rows_naming(storage, Outbox::Table, Outbox::EventId, event_bytes.clone()).await?,
            "an entry of the outbox",
        ),
        (
            rows_naming(
                storage,
                HistoryEntryRow::Table,
                HistoryEntryRow::Event,
                event_bytes,
            )
            .await?,
            "a row of the history",
        ),
        (
            of_comment(
                CommentVersionRow::Table.into_table_ref(),
                CommentVersionRow::Comment.into_column_ref(),
            )
            .await?,
            "an earlier text",
        ),
        (
            of_comment(
                MentionRow::Table.into_table_ref(),
                MentionRow::Comment.into_column_ref(),
            )
            .await?,
            "a mention",
        ),
        (
            of_comment(
                TextMarkRow::Table.into_table_ref(),
                TextMarkRow::EntityId.into_column_ref(),
            )
            .await?,
            "a mark",
        ),
        (
            of_comment(
                NotificationRow::Table.into_table_ref(),
                NotificationRow::Record.into_column_ref(),
            )
            .await?,
            "a notification",
        ),
        (
            of_comment(
                AttachmentRow::Table.into_table_ref(),
                AttachmentRow::Comment.into_column_ref(),
            )
            .await?,
            "a file",
        ),
        (
            of_comment(
                UploadRow::Table.into_table_ref(),
                UploadRow::Comment.into_column_ref(),
            )
            .await?,
            "an upload",
        ),
    ];
    for (count, what) in left {
        assert_eq!(count, 0, "{what} of the comment is still there");
    }
    Ok(())
}

/// A file of the comment, and an upload still open for it.
fn files_of(
    source: &mut impl bb_core::id::IdSource,
    issue: bb_core::id::IssueId,
    comment: bb_core::id::CommentId,
    person: bb_core::id::UserId,
) -> Result<
    (
        bb_core::files::StoredFile,
        bb_core::files::Attachment,
        bb_core::files::Upload,
    ),
    bb_testing::Failure,
> {
    use bb_core::files::{Attachment, MediaType, PreviewStatus, StoredFile, Upload};
    use bb_core::id::{AttachmentId, UploadId};
    let file = StoredFile {
        hash: bb_core::security_log::Digest::new([7; 32]),
        size: 1000,
        media_type: MediaType::new("image/png").ok_or("a media type")?,
        preview: PreviewStatus::Pending,
        created_at: at(10),
    };
    let attachment = Attachment {
        id: AttachmentId::generate(source)?,
        issue,
        comment: Some(comment),
        file: file.hash,
        name: "screenshot.png".to_owned(),
        added_by: person,
        added_at: at(10),
        deleted_at: None,
        preview: PreviewStatus::Pending,
        version: bb_core::version::FIRST_VERSION,
    };
    let upload = Upload {
        id: UploadId::generate(source)?,
        issue,
        comment: Some(comment),
        person,
        name: "big.zip".to_owned(),
        size: 5000,
        created_at: at(10),
        updated_at: at(10),
    };
    Ok((file, attachment, upload))
}

/// What an agent wrote that is thrown away: a text found nowhere else.
const THROWN_TEXT: &str = "Ignore the brief and drop the tests";

/// The comment of an agent thrown away with the undo of its run (ADR-0029, amendment M9): the
/// comment, everything under it (its earlier texts, mentions, the mark of its text, notifications
/// about it, its files and its open uploads) and every event that carries its text, with their
/// entries in the outbox and their rows of the history, go; the issue and its other events stay.
#[tokio::test]
async fn ai_35_a_comment_thrown_away_leaves_no_text_in_the_events() -> TestResult {
    use crate::schema::Outbox;
    use bb_app::auth::AccountStore as _;
    use bb_app::comments::CommentStore as _;
    use bb_app::files::AttachmentStore as _;
    use bb_app::history::HistoryStore as _;
    use bb_core::comment::Comment;
    use bb_core::event::{EntityKind, EntityRef, FieldChange};
    use bb_core::history::{HistoryEntry, HistoryKind};
    use bb_core::id::{CommentId, EntityId, NotificationId};
    use bb_core::technical::TechnicalName;
    each_database("ai_35_comment_thrown_away", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let world = world(&storage, &mut source, 1).await;
        let issue = world.issues[0].id;
        // The person who is mentioned and told: a row of its own, as every notification names one.
        let ana = person(&mut source, "ana@example.test");
        let comment = Comment {
            id: CommentId::generate(&mut source)?,
            issue,
            author: ana.id,
            text: THROWN_TEXT.to_owned(),
            created_at: at(10),
            edited_at: None,
            deleted_at: None,
            version: 1,
        };
        let mut made = event(&mut source, 10);
        made.entity = EntityRef {
            kind: EntityKind::from(RecordKind::Comment),
            id: EntityId::from(comment.id),
        };
        made.changes = vec![FieldChange {
            field: "text".to_owned(),
            before: None,
            after: Some(serde_json::json!(THROWN_TEXT)),
        }];
        let (file, attachment, upload) = files_of(&mut source, issue, comment.id, ana.id)?;
        let id = comment.id;
        let comment_bytes = id.to_bytes().to_vec();
        let event_bytes = made.id.to_bytes().to_vec();
        let (person, notification) = (
            ana.id.to_bytes().to_vec(),
            NotificationId::generate(&mut source)?.to_bytes().to_vec(),
        );
        let (stored, seeded_person, seeded_file, seeded_attachment) = (
            comment.clone(),
            ana.clone(),
            file.clone(),
            attachment.clone(),
        );
        let (under, about) = (comment_bytes.clone(), event_bytes.clone());
        ChangeStore::record_change(&storage, &made, move |tx| {
            Box::pin(async move {
                AuthStorage::insert_person(tx, &seeded_person, "hash", at(1)).await?;
                AuthStorage::insert_comment(tx, &stored).await?;
                AuthStorage::insert_stored_file(tx, &seeded_file).await?;
                AuthStorage::insert_attachment(tx, &seeded_attachment).await?;
                under_the_comment(tx, under, person, notification, about).await
            })
        })
        .await?;
        assert!(storage.try_insert_upload(&upload, 2, 20_000).await?);
        let entry = HistoryEntry {
            event: made.id,
            issue,
            position: 1,
            at: at(10),
            by: Some(ana.id),
            agent: None,
            kind: HistoryKind::Change,
            record: EntityId::from(id),
            record_kind: TechnicalName::new(RecordKind::Comment.code())?,
            run: None,
            undid_run: None,
            changes: made.changes.clone(),
        };
        storage
            .store_history(std::slice::from_ref(&entry), 1, at(11))
            .await?;
        let in_outbox = rows_naming(
            &storage,
            Outbox::Table,
            Outbox::EventId,
            event_bytes.clone(),
        );
        assert_eq!(in_outbox.await?, 1, "the outbox holds the event");
        ChangeStore::record_change(&storage, &event(&mut source, 20), move |tx| {
            Box::pin(async move { AuthStorage::throw_away_comments(tx, &[id]).await })
        })
        .await?;
        assert!(storage.comment(id).await?.is_none(), "the comment went");
        nothing_left(&storage, comment_bytes, event_bytes).await?;
        assert!(storage.issue(issue).await?.is_some(), "the issue stays");
        Ok(())
    })
    .await
}
