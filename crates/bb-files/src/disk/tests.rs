//! The files on disk (ATT-01, ATT-02): parts are put together in order, a finished file is stored
//! once under its hash, and a file that is not there is just not there.

use bb_core::id::{SequentialIds, UploadId};
use sha2::{Digest as _, Sha256};

use super::*;

fn files() -> (tempfile::TempDir, DiskFiles) {
    let folder = tempfile::tempdir().unwrap();
    // No preview is made in these tests, so no program is needed.
    let files = DiskFiles::new(folder.path(), PreviewCommand::new("no-such-program", &[]));
    (folder, files)
}

fn upload_ids() -> (UploadId, UploadId) {
    let mut source = SequentialIds::default();
    (
        UploadId::generate(&mut source).unwrap(),
        UploadId::generate(&mut source).unwrap(),
    )
}

fn digest_of(bytes: &[u8]) -> Digest {
    Digest::new(Sha256::digest(bytes).into())
}

async fn read_all(files: &DiskFiles, hash: Digest) -> Option<Vec<u8>> {
    let mut body = files.open(hash, LinkTarget::File).await.unwrap()?;
    let mut all = Vec::new();
    while let Some(chunk) = body.reader.next_chunk().await.unwrap() {
        all.extend(chunk);
    }
    assert_eq!(all.len() as u64, body.size);
    Some(all)
}

#[tokio::test]
async fn att_01_parts_are_put_together_in_order_and_a_part_out_of_order_is_refused() {
    let (_folder, files) = files();
    let (upload, _) = upload_ids();
    files.begin_upload(upload).await.unwrap();
    assert_eq!(files.received(upload).await.unwrap(), 0);
    assert_eq!(
        files
            .write_part(upload, 0, b"Hello, ".to_vec())
            .await
            .unwrap(),
        7
    );
    // The same part again, or one that skips ahead, says where the file stands.
    for offset in [0, 3, 20] {
        match files.write_part(upload, offset, b"x".to_vec()).await {
            Err(PartError::OutOfOrder { received }) => assert_eq!(received, 7),
            other => panic!("{other:?}"),
        }
    }
    assert_eq!(
        files
            .write_part(upload, 7, b"world".to_vec())
            .await
            .unwrap(),
        12
    );
    assert_eq!(files.received(upload).await.unwrap(), 12);
    let inspected = files.inspect_upload(upload).await.unwrap();
    assert_eq!(inspected.size, 12);
    assert_eq!(inspected.hash, digest_of(b"Hello, world"));
}

#[tokio::test]
async fn att_01_two_parts_at_the_same_moment_are_put_one_after_the_other() {
    let (_folder, files) = files();
    let files = Arc::new(files);
    let (upload, _) = upload_ids();
    files.begin_upload(upload).await.unwrap();
    let tries: Vec<_> = (0..8)
        .map(|_| {
            let files = Arc::clone(&files);
            tokio::spawn(async move { files.write_part(upload, 0, b"abcd".to_vec()).await })
        })
        .collect();
    let mut accepted = 0;
    for task in tries {
        accepted += usize::from(task.await.unwrap().is_ok());
    }
    assert_eq!(accepted, 1, "only one of them was at the offset");
    assert_eq!(files.received(upload).await.unwrap(), 4);
}

#[tokio::test]
async fn att_02_a_file_is_stored_once_by_its_content() {
    let (folder, files) = files();
    let (first, second) = upload_ids();
    let bytes = b"The printer jams on page two.".to_vec();
    let hash = digest_of(&bytes);
    for upload in [first, second] {
        files.begin_upload(upload).await.unwrap();
        files.write_part(upload, 0, bytes.clone()).await.unwrap();
        let inspected = files.inspect_upload(upload).await.unwrap();
        assert_eq!(inspected.hash, hash);
        files.keep_upload(upload, hash).await.unwrap();
        // The upload is gone, the file is there.
        assert_eq!(files.received(upload).await.unwrap(), 0);
        assert_eq!(read_all(&files, hash).await.as_deref(), Some(&bytes[..]));
    }
    let stored: Vec<_> = walk(&folder.path().join(CONTENT));
    assert_eq!(stored.len(), 1, "{stored:?}");
    // The name is the hash, in a folder named by its first byte.
    let name = hex::encode(hash.as_bytes());
    assert!(
        stored[0].ends_with(
            format!("{}/{name}", &name[..2]).replace('/', std::path::MAIN_SEPARATOR_STR)
        )
    );
    assert!(walk(&folder.path().join(UPLOADS)).is_empty());
}

#[tokio::test]
async fn att_02_a_copy_that_was_cut_short_is_replaced_and_deleting_removes_the_file() {
    let (folder, files) = files();
    let (upload, _) = upload_ids();
    let bytes = b"whole content".to_vec();
    let hash = digest_of(&bytes);
    // A file of the right name that is shorter than it should be.
    let path = files.content_path(hash);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"whole").unwrap();
    files.begin_upload(upload).await.unwrap();
    files.write_part(upload, 0, bytes.clone()).await.unwrap();
    files.keep_upload(upload, hash).await.unwrap();
    assert_eq!(read_all(&files, hash).await.as_deref(), Some(&bytes[..]));

    files.delete(hash).await.unwrap();
    assert!(read_all(&files, hash).await.is_none());
    // Deleting what is not there is no error.
    files.delete(hash).await.unwrap();
    assert!(walk(&folder.path().join(CONTENT)).is_empty());
}

#[tokio::test]
async fn att_01_a_discarded_upload_leaves_nothing_and_asking_again_is_no_error() {
    let (folder, files) = files();
    let (upload, _) = upload_ids();
    files.begin_upload(upload).await.unwrap();
    files.write_part(upload, 0, b"abc".to_vec()).await.unwrap();
    files.discard_upload(upload).await.unwrap();
    files.discard_upload(upload).await.unwrap();
    assert!(walk(&folder.path().join(UPLOADS)).is_empty());
}

#[tokio::test]
async fn att_04_a_preview_of_a_file_that_is_not_there_is_none_and_no_program_is_started() {
    let (_folder, files) = files();
    let state = files
        .create_preview(Digest::new([1; 32]), PreviewKind::Image)
        .await
        .unwrap();
    assert_eq!(state, PreviewStatus::Without);
}

/// Every file below `folder`.
fn walk(folder: &std::path::Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(folder) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(walk(&path));
        } else {
            found.push(path);
        }
    }
    found
}

#[tokio::test]
async fn att_02_whoever_holds_a_file_keeps_the_others_of_that_file_waiting() {
    let (_folder, files) = files();
    let hash = digest_of(b"held");
    let other = digest_of(b"another");
    let first = files.hold(hash).await;
    // The same file waits; a file of another first byte does not.
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(50), files.hold(hash))
            .await
            .is_err()
    );
    assert_ne!(hash.as_bytes()[0], other.as_bytes()[0]);
    drop(files.hold(other).await);
    drop(first);
    drop(files.hold(hash).await);
}

#[tokio::test]
async fn att_01_a_part_has_room_on_a_disk_with_space_and_none_on_one_that_would_be_full() {
    let (_folder, files) = files();
    assert!(files.has_room(1).await.unwrap(), "a byte fits");
    assert!(
        !files.has_room(u64::MAX).await.unwrap(),
        "more than the disk has does not"
    );
}
