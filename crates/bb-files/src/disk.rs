//! The bytes of files on disk (ADR-0032 §1), in the `files/` folder of the data folder:
//!
//! - `uploads/<upload>`: a file on its way, put together by appending the parts;
//! - `content/<aa>/<hash>`: a finished file, named by the SHA-256 of its content (`aa` is the first
//!   byte, so no folder holds millions of files);
//! - `previews/<aa>/<hash>.png`: the preview of a file;
//! - `previews/work/`: where the child writes a preview before the server accepts it.
//!
//! No name here comes from a person: the folders hold IDs and hashes only.

use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use bb_app::files::{ChunkReader, FileBody, FileHold, FileStore, Inspected, PartError, Sniffed};
use bb_app::ports::BoxFuture;
use bb_core::files::{DISK_KEPT_FREE_BYTES, LinkTarget, PreviewKind, PreviewState};
use bb_core::hex;
use bb_core::id::UploadId;
use bb_core::security_log::Digest;
use sha2::{Digest as _, Sha256};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

use crate::preview::{self, PreviewCommand, PreviewOutcome};
use crate::sniff::{HEAD_BYTES, sniff};

const UPLOADS: &str = "uploads";
const CONTENT: &str = "content";
const PREVIEWS: &str = "previews";
const PREVIEW_WORK: &str = "work";
const PREVIEW_EXTENSION: &str = "png";

/// How many locks hold files apart: a hash is told by its first byte.
const HOLDS: usize = 256;

/// How many bytes a piece of a file holds when it is read.
const CHUNK_BYTES: usize = 64 * 1024;

/// The files of an installation, on disk.
pub struct DiskFiles {
    root: PathBuf,
    command: PreviewCommand,
    /// One lock for each upload that gets parts, so two parts of one upload are put one after the
    /// other.
    locks: Mutex<HashMap<UploadId, Arc<tokio::sync::Mutex<()>>>>,
    /// The locks of [`FileStore::hold`]: one for each first byte of a hash, so there are never
    /// more than [`HOLDS`] of them and files with the same first byte only wait for each other
    /// for the moment a clean-up or a finished upload takes.
    holds: Vec<Arc<tokio::sync::Mutex<()>>>,
}

impl DiskFiles {
    /// The files in `root` (the `files/` folder of the data folder); previews are made by
    /// `command`.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>, command: PreviewCommand) -> Self {
        Self {
            root: root.into(),
            command,
            locks: Mutex::default(),
            holds: (0..HOLDS).map(|_| Arc::default()).collect(),
        }
    }

    fn upload_path(&self, upload: UploadId) -> PathBuf {
        self.root.join(UPLOADS).join(upload.to_string())
    }

    fn content_path(&self, hash: Digest) -> PathBuf {
        let name = hex::encode(hash.as_bytes());
        self.root.join(CONTENT).join(&name[..2]).join(name)
    }

    fn preview_path(&self, hash: Digest) -> PathBuf {
        let name = hex::encode(hash.as_bytes());
        self.root
            .join(PREVIEWS)
            .join(&name[..2])
            .join(format!("{name}.{PREVIEW_EXTENSION}"))
    }

    fn lock_of(&self, upload: UploadId) -> Arc<tokio::sync::Mutex<()>> {
        let mut locks = self.locks.lock().unwrap_or_else(PoisonError::into_inner);
        Arc::clone(locks.entry(upload).or_default())
    }

    fn forget_lock(&self, upload: UploadId) {
        self.locks
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&upload);
    }
}

/// Makes the folder that holds `path`.
async fn create_parent(path: &std::path::Path) -> io::Result<()> {
    match path.parent() {
        Some(parent) => tokio::fs::create_dir_all(parent).await,
        None => Ok(()),
    }
}

/// Removes a file; a file that is not there is no error.
async fn remove(path: &std::path::Path) -> io::Result<()> {
    match tokio::fs::remove_file(path).await {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

/// The hash, the size and the first bytes of the file at `path`, read in pieces.
fn read_through(path: &std::path::Path) -> io::Result<(Digest, u64, Vec<u8>)> {
    use std::io::Read as _;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut head = Vec::with_capacity(HEAD_BYTES);
    let mut buffer = vec![0_u8; CHUNK_BYTES];
    let mut size = 0_u64;
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let piece = &buffer[..read];
        hasher.update(piece);
        size += read as u64;
        if head.len() < HEAD_BYTES {
            let room = HEAD_BYTES - head.len();
            head.extend_from_slice(&piece[..piece.len().min(room)]);
        }
    }
    Ok((Digest::new(hasher.finalize().into()), size, head))
}

struct DiskReader(tokio::fs::File);

impl ChunkReader for DiskReader {
    fn next_chunk(&mut self) -> BoxFuture<'_, io::Result<Option<Vec<u8>>>> {
        Box::pin(async move {
            let mut buffer = vec![0_u8; CHUNK_BYTES];
            let read = self.0.read(&mut buffer).await?;
            buffer.truncate(read);
            Ok((read > 0).then_some(buffer))
        })
    }
}

impl FileStore for DiskFiles {
    fn sniff(&self, head: &[u8]) -> Sniffed {
        sniff(head)
    }

    fn hold(&self, hash: Digest) -> BoxFuture<'_, FileHold> {
        let lock = Arc::clone(&self.holds[usize::from(hash.as_bytes()[0])]);
        Box::pin(async move { Box::new(lock.lock_owned().await) as FileHold })
    }

    fn has_room(&self, bytes: u64) -> BoxFuture<'_, io::Result<bool>> {
        Box::pin(async move {
            tokio::fs::create_dir_all(&self.root).await?;
            let root = self.root.clone();
            let free = tokio::task::spawn_blocking(move || fs4::available_space(root))
                .await
                .map_err(io::Error::other)??;
            Ok(free
                .checked_sub(DISK_KEPT_FREE_BYTES)
                .is_some_and(|spare| spare >= bytes))
        })
    }

    fn begin_upload(&self, upload: UploadId) -> BoxFuture<'_, io::Result<()>> {
        Box::pin(async move {
            let path = self.upload_path(upload);
            create_parent(&path).await?;
            tokio::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .await
                .map(|_| ())
        })
    }

    fn received(&self, upload: UploadId) -> BoxFuture<'_, io::Result<u64>> {
        Box::pin(async move {
            match tokio::fs::metadata(self.upload_path(upload)).await {
                Ok(metadata) => Ok(metadata.len()),
                Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(0),
                Err(error) => Err(error),
            }
        })
    }

    fn write_part(
        &self,
        upload: UploadId,
        offset: u64,
        bytes: Vec<u8>,
    ) -> BoxFuture<'_, Result<u64, PartError>> {
        Box::pin(async move {
            let lock = self.lock_of(upload);
            let _turn = lock.lock().await;
            let mut file = tokio::fs::OpenOptions::new()
                .append(true)
                .open(self.upload_path(upload))
                .await?;
            let length = file.metadata().await?.len();
            if length != offset {
                return Err(PartError::OutOfOrder { received: length });
            }
            file.write_all(&bytes).await?;
            // The part is on disk before the client is told, so a crash never leaves the client
            // ahead of the server.
            file.sync_data().await?;
            Ok(length + bytes.len() as u64)
        })
    }

    fn inspect_upload(&self, upload: UploadId) -> BoxFuture<'_, io::Result<Inspected>> {
        Box::pin(async move {
            let path = self.upload_path(upload);
            let (hash, size, head) = tokio::task::spawn_blocking(move || read_through(&path))
                .await
                .map_err(io::Error::other)??;
            Ok(Inspected {
                hash,
                size,
                sniffed: sniff(&head),
            })
        })
    }

    fn keep_upload(&self, upload: UploadId, hash: Digest) -> BoxFuture<'_, io::Result<()>> {
        Box::pin(async move {
            let from = self.upload_path(upload);
            let to = self.content_path(hash);
            create_parent(&to).await?;
            let size = tokio::fs::metadata(&from).await?.len();
            let stored = tokio::fs::metadata(&to).await.ok().map(|found| found.len());
            if stored == Some(size) {
                // The same content is stored already: a file is stored once.
                remove(&from).await?;
            } else {
                // Not there, or not whole (a copy that was cut short): put this one in its place.
                remove(&to).await?;
                tokio::fs::rename(&from, &to).await?;
            }
            self.forget_lock(upload);
            Ok(())
        })
    }

    fn discard_upload(&self, upload: UploadId) -> BoxFuture<'_, io::Result<()>> {
        Box::pin(async move {
            self.forget_lock(upload);
            remove(&self.upload_path(upload)).await
        })
    }

    fn open(
        &self,
        hash: Digest,
        target: LinkTarget,
    ) -> BoxFuture<'_, io::Result<Option<FileBody>>> {
        Box::pin(async move {
            let path = match target {
                LinkTarget::File => self.content_path(hash),
                LinkTarget::Preview => self.preview_path(hash),
            };
            let file = match tokio::fs::File::open(path).await {
                Ok(file) => file,
                Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
                Err(error) => return Err(error),
            };
            let size = file.metadata().await?.len();
            Ok(Some(FileBody {
                size,
                reader: Box::new(DiskReader(file)),
            }))
        })
    }

    fn create_preview(
        &self,
        hash: Digest,
        kind: PreviewKind,
    ) -> BoxFuture<'_, io::Result<PreviewState>> {
        Box::pin(async move {
            let input = self.content_path(hash);
            if tokio::fs::metadata(&input).await.is_err() {
                return Ok(PreviewState::Without);
            }
            let name = hex::encode(hash.as_bytes());
            let work = self
                .root
                .join(PREVIEWS)
                .join(PREVIEW_WORK)
                .join(format!("{name}.{PREVIEW_EXTENSION}"));
            create_parent(&work).await?;
            let outcome = preview::create(&self.command, kind, &input, &work).await?;
            if outcome != PreviewOutcome::Created {
                tracing::info!(?outcome, kind = kind.code(), "a file got no preview");
                return Ok(PreviewState::Without);
            }
            let to = self.preview_path(hash);
            create_parent(&to).await?;
            remove(&to).await?;
            tokio::fs::rename(&work, &to).await?;
            Ok(PreviewState::Ready)
        })
    }

    fn delete(&self, hash: Digest) -> BoxFuture<'_, io::Result<()>> {
        Box::pin(async move {
            remove(&self.content_path(hash)).await?;
            remove(&self.preview_path(hash)).await
        })
    }
}

#[cfg(test)]
#[path = "disk/tests.rs"]
mod tests;
