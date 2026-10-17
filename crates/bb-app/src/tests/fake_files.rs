//! The file parts of the in-memory store: the rows of files, attachments and uploads, and an
//! in-memory store of the bytes (`FakeFiles`) that sniffs nothing and previews nothing.

use std::collections::HashMap;
use std::io;
use std::sync::{Arc, Mutex, MutexGuard};

use bb_core::files::{
    Attachment, LinkTarget, MediaType, PreviewKind, PreviewStatus, StoredFile, Upload,
};
use bb_core::id::{AttachmentId, IssueId, UploadId};
use bb_core::security_log::Digest;
use jiff::Timestamp;
use sha2::{Digest as _, Sha256};

use super::fake::{FakeError, FakeStore, Tx};
use crate::files::{
    AttachmentStore, ChunkReader, FileBody, FileHold, FileStore, Inspected, PartError, Sniffed,
};
use crate::ports::BoxFuture;

fn limited<T>(mut found: Vec<T>, limit: u64) -> Vec<T> {
    found.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
    found
}

impl AttachmentStore for FakeStore {
    async fn attachment_in(tx: &mut Tx, id: AttachmentId) -> Result<Option<Attachment>, FakeError> {
        Ok(tx
            .state
            .attachments
            .iter()
            .find(|attachment| attachment.id == id)
            .cloned())
    }

    async fn upload_in(tx: &mut Tx, id: UploadId) -> Result<Option<Upload>, FakeError> {
        Ok(tx
            .state
            .uploads
            .iter()
            .find(|upload| upload.id == id)
            .cloned())
    }

    async fn stored_file_in(tx: &mut Tx, hash: Digest) -> Result<Option<StoredFile>, FakeError> {
        Ok(tx
            .state
            .stored_files
            .iter()
            .find(|file| file.hash == hash)
            .cloned())
    }

    async fn attachments_of_issue(&self, issue: IssueId) -> Result<Vec<Attachment>, FakeError> {
        Ok(self.read(|state| {
            let mut found: Vec<Attachment> = state
                .attachments
                .iter()
                .filter(|attachment| attachment.issue == issue && attachment.deleted_at.is_none())
                .cloned()
                .collect();
            found.sort_by_key(|attachment| attachment.id);
            found
        }))
    }

    async fn attachments_by_ids(&self, ids: &[AttachmentId]) -> Result<Vec<Attachment>, FakeError> {
        Ok(self.read(|state| {
            state
                .attachments
                .iter()
                .filter(|attachment| ids.contains(&attachment.id))
                .cloned()
                .collect()
        }))
    }

    async fn stored_files(&self, hashes: &[Digest]) -> Result<Vec<StoredFile>, FakeError> {
        Ok(self.read(|state| {
            state
                .stored_files
                .iter()
                .filter(|file| hashes.contains(&file.hash))
                .cloned()
                .collect()
        }))
    }

    async fn attachment_count_of_issue(&self, issue: IssueId) -> Result<u64, FakeError> {
        let count = self.read(|state| {
            state
                .attachments
                .iter()
                .filter(|attachment| attachment.issue == issue)
                .count() as u64
        });
        // A real store takes time to answer: whoever else is working gets its turn before the
        // answer is used.
        tokio::task::yield_now().await;
        Ok(count)
    }

    async fn try_insert_upload(
        &self,
        upload: &Upload,
        open_max: u64,
        bytes_max: u64,
    ) -> Result<bool, FakeError> {
        let mut shared = self.lock();
        let (open, declared) = shared
            .state
            .uploads
            .iter()
            .filter(|known| known.person == upload.person)
            .fold((0_u64, 0_u64), |(open, declared), known| {
                (open + 1, declared + u64::try_from(known.size).unwrap_or(0))
            });
        if open >= open_max || declared + u64::try_from(upload.size).unwrap_or(0) > bytes_max {
            return Ok(false);
        }
        shared.state.uploads.push(upload.clone());
        Ok(true)
    }

    async fn uploads_untouched_since(
        &self,
        before: Timestamp,
        limit: u64,
    ) -> Result<Vec<Upload>, FakeError> {
        Ok(self.read(|state| {
            limited(
                state
                    .uploads
                    .iter()
                    .filter(|upload| upload.updated_at < before)
                    .cloned()
                    .collect(),
                limit,
            )
        }))
    }

    async fn touch_upload(&self, id: UploadId, at: Timestamp) -> Result<(), FakeError> {
        for upload in &mut self.lock().state.uploads {
            if upload.id == id {
                upload.updated_at = at;
            }
        }
        Ok(())
    }

    async fn delete_upload_row(&self, id: UploadId) -> Result<bool, FakeError> {
        let mut shared = self.lock();
        let before = shared.state.uploads.len();
        shared.state.uploads.retain(|upload| upload.id != id);
        Ok(shared.state.uploads.len() < before)
    }

    async fn set_preview_status(
        &self,
        hash: Digest,
        state: PreviewStatus,
    ) -> Result<bool, FakeError> {
        let mut shared = self.lock();
        let found = shared
            .state
            .stored_files
            .iter_mut()
            .find(|file| file.hash == hash);
        Ok(found.map(|file| file.preview = state).is_some())
    }

    async fn set_attachment_preview(
        &self,
        id: AttachmentId,
        state: PreviewStatus,
    ) -> Result<bool, FakeError> {
        let mut shared = self.lock();
        let found = shared
            .state
            .attachments
            .iter_mut()
            .find(|attachment| attachment.id == id);
        Ok(found.map(|attachment| attachment.preview = state).is_some())
    }

    async fn purge_trashed_attachments(
        &self,
        before: Timestamp,
        limit: u64,
    ) -> Result<u64, FakeError> {
        let mut shared = self.lock();
        let mut purged = 0_u64;
        shared.state.attachments.retain(|attachment| {
            let due = attachment.deleted_at.is_some_and(|at| at < before) && purged < limit;
            purged += u64::from(due);
            !due
        });
        Ok(purged)
    }

    async fn unreferenced_files(&self, limit: u64) -> Result<Vec<StoredFile>, FakeError> {
        Ok(self.read(|state| {
            limited(
                state
                    .stored_files
                    .iter()
                    .filter(|file| {
                        !state
                            .attachments
                            .iter()
                            .any(|attachment| attachment.file == file.hash)
                    })
                    .cloned()
                    .collect(),
                limit,
            )
        }))
    }

    async fn delete_unreferenced_file(&self, hash: Digest) -> Result<bool, FakeError> {
        let mut shared = self.lock();
        let referenced = shared
            .state
            .attachments
            .iter()
            .any(|attachment| attachment.file == hash);
        let before = shared.state.stored_files.len();
        if !referenced {
            shared.state.stored_files.retain(|file| file.hash != hash);
        }
        Ok(shared.state.stored_files.len() < before)
    }

    async fn lock_issue_files(_tx: &mut Tx, _issue: IssueId) -> Result<(), FakeError> {
        // A change of the in-memory store runs alone.
        Ok(())
    }

    async fn attachment_count_in_change(tx: &mut Tx, issue: IssueId) -> Result<u64, FakeError> {
        Ok(tx
            .state
            .attachments
            .iter()
            .filter(|attachment| attachment.issue == issue)
            .count() as u64)
    }

    async fn delete_upload(tx: &mut Tx, id: UploadId) -> Result<bool, FakeError> {
        let before = tx.state.uploads.len();
        tx.state.uploads.retain(|upload| upload.id != id);
        Ok(tx.state.uploads.len() < before)
    }

    async fn insert_stored_file(tx: &mut Tx, file: &StoredFile) -> Result<bool, FakeError> {
        if tx
            .state
            .stored_files
            .iter()
            .any(|known| known.hash == file.hash)
        {
            return Ok(false);
        }
        tx.state.stored_files.push(file.clone());
        Ok(true)
    }

    async fn insert_attachment(tx: &mut Tx, attachment: &Attachment) -> Result<(), FakeError> {
        tx.state.attachments.push(attachment.clone());
        Ok(())
    }

    async fn set_attachment_deleted(
        tx: &mut Tx,
        id: AttachmentId,
        deleted_at: Option<Timestamp>,
    ) -> Result<bool, FakeError> {
        let found = tx.state.attachments.iter_mut().find(|attachment| {
            attachment.id == id && attachment.deleted_at.is_none() == deleted_at.is_some()
        });
        Ok(found
            .map(|attachment| {
                attachment.deleted_at = deleted_at;
                attachment.version += 1;
            })
            .is_some())
    }
}

/// A stop the tests put into `keep_upload`: the bytes are in the store, and the call waits until the
/// test lets it go, so another call can come in between.
#[derive(Clone, Default)]
pub(crate) struct Gate {
    /// Told when `keep_upload` has put the bytes in the store and waits.
    pub(crate) entered: Arc<tokio::sync::Notify>,
    /// Lets `keep_upload` go on.
    pub(crate) release: Arc<tokio::sync::Notify>,
}

/// What the in-memory file store holds.
#[derive(Default)]
struct Held {
    uploads: HashMap<UploadId, Vec<u8>>,
    files: HashMap<Digest, Vec<u8>>,
    previews: HashMap<Digest, Vec<u8>>,
}

/// The first bytes of every PNG: a file that starts so is a picture.
const PICTURE_START: &[u8] = b"\x89PNG";

/// What a file that starts like a program of Windows says it is (the two letters every one starts
/// with).
const PROGRAM_START: &[u8] = b"MZ";

/// The bytes of files in memory. A file is plain data, except one that starts like a program of
/// Windows, and none gets a preview.
#[derive(Clone, Default)]
pub(crate) struct FakeFiles {
    held: Arc<Mutex<Held>>,
    turn: Arc<tokio::sync::Mutex<()>>,
    full: Arc<std::sync::atomic::AtomicBool>,
    previews_made: Arc<std::sync::atomic::AtomicUsize>,
    gate: Arc<Mutex<Option<Gate>>>,
}

impl FakeFiles {
    fn held(&self) -> MutexGuard<'_, Held> {
        self.held
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// How many previews were created (not reused).
    pub(crate) fn previews_made(&self) -> usize {
        self.previews_made.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Makes the disk full: no part has room any more.
    pub(crate) fn fill_up(&self) {
        self.full.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// How many different files are stored.
    pub(crate) fn file_count(&self) -> usize {
        self.held().files.len()
    }

    /// Makes the next `keep_upload` stop after the bytes are in the store, until the gate lets it
    /// go.
    pub(crate) fn stop_after_keeping(&self) -> Gate {
        let gate = Gate::default();
        *self
            .gate
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(gate.clone());
        gate
    }

    /// How many uploads are on their way.
    pub(crate) fn upload_count(&self) -> usize {
        self.held().uploads.len()
    }
}

struct InMemory(Option<Vec<u8>>);

impl ChunkReader for InMemory {
    fn next_chunk(&mut self) -> BoxFuture<'_, io::Result<Option<Vec<u8>>>> {
        let chunk = self.0.take();
        Box::pin(async move { Ok(chunk) })
    }
}

impl FileStore for FakeFiles {
    fn sniff(&self, head: &[u8]) -> Sniffed {
        if head.starts_with(PROGRAM_START) {
            return Sniffed {
                media_type: MediaType::new("application/vnd.microsoft.portable-executable")
                    .unwrap_or_else(MediaType::unknown),
                extension: Some("exe".to_owned()),
            };
        }
        if head.starts_with(PICTURE_START) {
            return Sniffed {
                media_type: MediaType::new("image/png").unwrap_or_else(MediaType::unknown),
                extension: Some("png".to_owned()),
            };
        }
        Sniffed {
            media_type: MediaType::unknown(),
            extension: None,
        }
    }

    fn hold(&self, _hash: Digest) -> BoxFuture<'_, FileHold> {
        let turn = Arc::clone(&self.turn);
        Box::pin(async move { Box::new(turn.lock_owned().await) as FileHold })
    }

    fn has_room(&self, _bytes: u64) -> BoxFuture<'_, io::Result<bool>> {
        let room = !self.full.load(std::sync::atomic::Ordering::SeqCst);
        Box::pin(async move { Ok(room) })
    }

    fn begin_upload(&self, upload: UploadId) -> BoxFuture<'_, io::Result<()>> {
        self.held().uploads.insert(upload, Vec::new());
        Box::pin(async { Ok(()) })
    }

    fn received(&self, upload: UploadId) -> BoxFuture<'_, io::Result<u64>> {
        let received = self.held().uploads.get(&upload).map_or(0, Vec::len);
        Box::pin(async move { Ok(received as u64) })
    }

    fn write_part(
        &self,
        upload: UploadId,
        offset: u64,
        bytes: Vec<u8>,
    ) -> BoxFuture<'_, Result<u64, PartError>> {
        let mut held = self.held();
        let result = match held.uploads.get_mut(&upload) {
            Some(stored) if stored.len() as u64 == offset => {
                stored.extend_from_slice(&bytes);
                Ok(stored.len() as u64)
            }
            Some(stored) => Err(PartError::OutOfOrder {
                received: stored.len() as u64,
            }),
            None => Err(PartError::Io(io::Error::from(io::ErrorKind::NotFound))),
        };
        Box::pin(async move { result })
    }

    fn inspect_upload(&self, upload: UploadId) -> BoxFuture<'_, io::Result<Inspected>> {
        let result = self
            .held()
            .uploads
            .get(&upload)
            .map(|bytes| Inspected {
                hash: Digest::new(Sha256::digest(bytes).into()),
                size: bytes.len() as u64,
                sniffed: self.sniff(bytes),
            })
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound));
        Box::pin(async move { result })
    }

    fn keep_upload(&self, upload: UploadId, hash: Digest) -> BoxFuture<'_, io::Result<()>> {
        let result = {
            let mut held = self.held();
            match held.uploads.remove(&upload) {
                Some(bytes) => {
                    held.files.entry(hash).or_insert(bytes);
                    Ok(())
                }
                None => Err(io::Error::from(io::ErrorKind::NotFound)),
            }
        };
        let gate = self
            .gate
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        Box::pin(async move {
            if let Some(gate) = gate {
                gate.entered.notify_one();
                gate.release.notified().await;
            }
            result
        })
    }

    fn discard_upload(&self, upload: UploadId) -> BoxFuture<'_, io::Result<()>> {
        self.held().uploads.remove(&upload);
        Box::pin(async { Ok(()) })
    }

    fn open(
        &self,
        hash: Digest,
        target: LinkTarget,
    ) -> BoxFuture<'_, io::Result<Option<FileBody>>> {
        let held = self.held();
        let found = match target {
            LinkTarget::File => held.files.get(&hash),
            LinkTarget::Preview => held.previews.get(&hash),
        };
        let body = found.map(|bytes| FileBody {
            size: bytes.len() as u64,
            reader: Box::new(InMemory(Some(bytes.clone()))),
        });
        Box::pin(async move { Ok(body) })
    }

    fn create_preview(
        &self,
        hash: Digest,
        _kind: PreviewKind,
    ) -> BoxFuture<'_, io::Result<PreviewStatus>> {
        let mut held = self.held();
        let state = if held.files.contains_key(&hash) {
            held.previews.insert(hash, b"a small picture".to_vec());
            self.previews_made
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            PreviewStatus::Ready
        } else {
            PreviewStatus::Without
        };
        Box::pin(async move { Ok(state) })
    }

    fn delete(&self, hash: Digest) -> BoxFuture<'_, io::Result<()>> {
        let mut held = self.held();
        held.files.remove(&hash);
        held.previews.remove(&hash);
        Box::pin(async { Ok(()) })
    }
}
