//! Files, attachments and uploads on both databases (ATT-01, ATT-02): a file is one row whatever
//! the number of attachments, the row of a file that an attachment points to is never deleted, the
//! trash keeps an attachment until its days are over, and old uploads are found by their last part.

use bb_app::files::AttachmentStore as _;
use bb_app::ports::{BoxFuture, ChangeStore};
use bb_core::files::{
    ATTACHMENTS_PER_ISSUE_MAX, Attachment, MediaType, PreviewStatus, StoredFile, TRASH_KEPT,
    UNFINISHED_KEPT, Upload,
};
use bb_core::id::{AttachmentId, IdSource, IssueId, UploadId};
use bb_core::security_log::Digest;
use bb_core::version::FIRST_VERSION;
use bb_testing::{TestResult, each_database};

use super::*;
use crate::test_support::{World, at, event, fixture, ids, world};

async fn changed<T: Send>(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    write: impl for<'t> FnOnce(&'t mut Transaction) -> BoxFuture<'t, Result<T, StoreError>> + Send,
) -> Result<T, StoreError> {
    ChangeStore::record_change(storage, &event(source, 5), write).await
}

fn file(byte: u8) -> StoredFile {
    StoredFile {
        hash: Digest::new([byte; 32]),
        size: 1000,
        media_type: MediaType::new("image/png").unwrap(),
        preview: PreviewStatus::Pending,
        created_at: at(1),
    }
}

fn attachment(
    source: &mut impl IdSource,
    world: &World,
    at_issue: usize,
    file: &StoredFile,
) -> Attachment {
    Attachment {
        id: AttachmentId::generate(source).unwrap(),
        issue: world.issues[at_issue].id,
        comment: None,
        file: file.hash,
        name: "screenshot.png".to_owned(),
        added_by: world.person,
        added_at: at(2),
        deleted_at: None,
        preview: PreviewStatus::Pending,
        version: FIRST_VERSION,
    }
}

async fn add(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    file: &StoredFile,
    attachment: &Attachment,
) -> Result<bool, StoreError> {
    let (file, attachment) = (file.clone(), attachment.clone());
    changed(storage, source, move |tx| {
        Box::pin(async move {
            let new = AuthStorage::insert_stored_file(tx, &file).await?;
            AuthStorage::insert_attachment(tx, &attachment).await?;
            Ok(new)
        })
    })
    .await
}

async fn trash(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    id: AttachmentId,
    at: Option<Timestamp>,
) -> Result<bool, StoreError> {
    changed(storage, source, move |tx| {
        Box::pin(async move { AuthStorage::set_attachment_deleted(tx, id, at).await })
    })
    .await
}

#[tokio::test]
async fn att_02_the_store_keeps_one_row_for_a_file_and_deletes_it_only_when_nothing_points_to_it()
-> TestResult {
    each_database("att_02_one_row_per_file", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let world = world(storage, &mut source, 2).await;
        let png = file(7);
        let first = attachment(&mut source, &world, 0, &png);
        let second = attachment(&mut source, &world, 1, &png);

        assert!(
            add(storage, &mut source, &png, &first).await?,
            "the first is new"
        );
        assert!(
            !add(storage, &mut source, &png, &second).await?,
            "the second is the same file"
        );
        assert_eq!(storage.stored_files(&[png.hash]).await?, vec![png.clone()]);
        assert!(storage.unreferenced_files(10).await?.is_empty());
        // The row of a file that something points to is not deleted.
        assert!(!storage.delete_unreferenced_file(png.hash).await?);

        // In the trash an attachment still points to its file, and is not listed.
        assert!(trash(storage, &mut source, first.id, Some(at(10))).await?);
        assert!(
            !trash(storage, &mut source, first.id, Some(at(11))).await?,
            "only once"
        );
        assert_eq!(
            storage.attachments_of_issue(world.issues[0].id).await?,
            vec![]
        );
        // It counts toward the files of the issue until it is purged.
        assert_eq!(
            storage
                .attachment_count_of_issue(world.issues[0].id)
                .await?,
            1
        );
        assert!(storage.unreferenced_files(10).await?.is_empty());
        // *Undo* puts it back, with the version counted up twice.
        assert!(trash(storage, &mut source, first.id, None).await?);
        let back = storage.attachments_of_issue(world.issues[0].id).await?;
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].version, FIRST_VERSION + 2);

        // Both go into the trash; only an attachment that stayed there for its days is purged.
        assert!(trash(storage, &mut source, first.id, Some(at(10))).await?);
        assert!(trash(storage, &mut source, second.id, Some(at(10_000_000))).await?);
        let enough = at(10).checked_add(TRASH_KEPT)?;
        assert_eq!(storage.purge_trashed_attachments(enough, 10).await?, 1);
        assert!(storage.attachment(first.id).await?.is_none());
        assert!(
            storage.unreferenced_files(10).await?.is_empty(),
            "the second still points"
        );
        let later = at(10_000_000).checked_add(TRASH_KEPT)?;
        assert_eq!(storage.purge_trashed_attachments(later, 10).await?, 1);

        // Now nothing points to the file.
        assert_eq!(storage.unreferenced_files(10).await?, vec![png.clone()]);
        assert!(storage.delete_unreferenced_file(png.hash).await?);
        assert!(storage.stored_files(&[png.hash]).await?.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn att_04_the_state_of_a_preview_is_kept_with_the_file() -> TestResult {
    each_database("att_04_preview_state", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let world = world(storage, &mut source, 1).await;
        let png = file(8);
        let row = attachment(&mut source, &world, 0, &png);
        add(storage, &mut source, &png, &row).await?;
        assert!(
            storage
                .set_preview_status(png.hash, PreviewStatus::Ready)
                .await?
        );
        let stored = storage.stored_files(&[png.hash]).await?;
        assert_eq!(stored[0].preview, PreviewStatus::Ready);
        assert!(
            !storage
                .set_preview_status(Digest::new([9; 32]), PreviewStatus::Ready)
                .await?
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn att_01_open_uploads_are_counted_and_old_ones_are_found_by_their_last_part() -> TestResult {
    each_database("att_01_uploads", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let world = world(storage, &mut source, 1).await;
        let mut upload = || Upload {
            id: UploadId::generate(&mut source).unwrap(),
            issue: world.issues[0].id,
            comment: None,
            person: world.person,
            name: "big.zip".to_owned(),
            size: 5000,
            created_at: at(0),
            updated_at: at(0),
        };
        let (old, fresh) = (upload(), upload());
        assert!(storage.try_insert_upload(&old, 2, 20_000).await?);
        assert!(storage.try_insert_upload(&fresh, 2, 20_000).await?);
        // The third is refused by the number, and a fourth that is small by the sum of the sizes.
        let third = upload();
        assert!(!storage.try_insert_upload(&third, 2, 20_000).await?);
        assert!(!storage.try_insert_upload(&third, 3, 14_999).await?);
        assert!(storage.try_insert_upload(&third, 3, 15_000).await?);
        assert!(storage.delete_upload_row(third.id).await?);
        let day_later = at(0)
            .checked_add(UNFINISHED_KEPT)?
            .checked_add(jiff::SignedDuration::from_secs(1))?;
        // A part of the fresh one came after the day started.
        storage.touch_upload(fresh.id, day_later).await?;
        let stale = storage.uploads_untouched_since(day_later, 10).await?;
        assert_eq!(
            stale.iter().map(|upload| upload.id).collect::<Vec<_>>(),
            vec![old.id]
        );
        assert!(storage.delete_upload_row(old.id).await?);
        assert!(!storage.delete_upload_row(old.id).await?);
        let left = storage.uploads_untouched_since(day_later, 10).await?;
        assert!(left.is_empty());
        assert!(storage.try_insert_upload(&upload(), 2, 20_000).await?);
        Ok(())
    })
    .await
}

/// The finish of an upload as the change of the store sees it: takes the lock of the issue, counts,
/// and adds the file only if there is room. Answers whether it did.
fn finishing(
    issue: IssueId,
    new: Attachment,
) -> impl for<'t> FnOnce(&'t mut Transaction) -> BoxFuture<'t, Result<bool, StoreError>> + Send {
    move |tx| {
        Box::pin(async move {
            AuthStorage::lock_issue_files(tx, issue).await?;
            let held = AuthStorage::attachment_count_in_change(tx, issue).await?;
            if held >= ATTACHMENTS_PER_ISSUE_MAX as u64 {
                return Ok(false);
            }
            AuthStorage::insert_attachment(tx, &new).await?;
            Ok(true)
        })
    }
}

#[tokio::test]
async fn att_03_two_finishes_at_the_last_place_of_an_issue_are_put_one_after_the_other()
-> TestResult {
    each_database("att_03_last_place", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let world = world(storage, &mut source, 1).await;
        let png = file(7);
        let issue = world.issues[0].id;
        // The issue holds one file less than it may.
        let held: Vec<Attachment> = (0..ATTACHMENTS_PER_ISSUE_MAX - 1)
            .map(|_| attachment(&mut source, &world, 0, &png))
            .collect();
        let stored = png.clone();
        changed(storage, &mut source, move |tx| {
            Box::pin(async move {
                AuthStorage::insert_stored_file(tx, &stored).await?;
                for each in &held {
                    AuthStorage::insert_attachment(tx, each).await?;
                }
                Ok(())
            })
        })
        .await?;

        // Two finishes at the same moment: each takes the lock of the issue, counts, and adds its
        // file only if there is room.
        let (first_event, second_event) = (event(&mut source, 5), event(&mut source, 5));
        let (first, second) = (
            attachment(&mut source, &world, 0, &png),
            attachment(&mut source, &world, 0, &png),
        );
        let (one, other) = tokio::join!(
            ChangeStore::record_change(storage, &first_event, finishing(issue, first)),
            ChangeStore::record_change(storage, &second_event, finishing(issue, second)),
        );
        assert_eq!([one?, other?].iter().filter(|took| **took).count(), 1);
        assert_eq!(
            storage.attachment_count_of_issue(issue).await?,
            ATTACHMENTS_PER_ISSUE_MAX as u64
        );
        Ok(())
    })
    .await
}
