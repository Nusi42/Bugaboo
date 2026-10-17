//! The port of the bytes of files (ADR-0032, ADR-0001): where uploads are put together, where the
//! files of the store live, and how a preview is made. The adapter is `bb-files` (a second one,
//! S3-compatible storage, can come later, ATT-06); the use cases know only this trait.
//!
//! Nothing here knows who may do what: the use cases judge that first. Everything that touches
//! bytes is asynchronous and may fail with the error of the file system.

use std::io;

use bb_core::files::{LinkTarget, MediaType, PreviewKind, PreviewState};
use bb_core::id::UploadId;
use bb_core::security_log::Digest;

use crate::ports::BoxFuture;

/// What the first bytes of a file say it is (ATT-03): never what the name or the browser claims.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sniffed {
    /// The real type, or `application/octet-stream` when the content says nothing more.
    pub media_type: MediaType,
    /// The extension of that type (`exe`), if the content is of a known type.
    pub extension: Option<String>,
}

/// What a finished upload is, found by reading all of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inspected {
    /// The SHA-256 of the content.
    pub hash: Digest,
    /// The size in bytes.
    pub size: u64,
    /// What the content says it is.
    pub sniffed: Sniffed,
}

/// Why a part was not written.
#[derive(Debug)]
pub enum PartError {
    /// The part does not start where the file stands now; the upload goes on from `received`.
    OutOfOrder {
        /// How many bytes the server has.
        received: u64,
    },
    /// The file system failed.
    Io(io::Error),
}

impl From<io::Error> for PartError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// The content of a file, read piece by piece, so a big file never sits in memory.
pub trait ChunkReader: Send {
    /// The next piece, or `None` at the end.
    fn next_chunk(&mut self) -> BoxFuture<'_, io::Result<Option<Vec<u8>>>>;
}

/// A file that is being sent: its size and its pieces.
pub struct FileBody {
    /// The size in bytes.
    pub size: u64,
    /// The pieces.
    pub reader: Box<dyn ChunkReader>,
}

/// What keeps other calls that work on the same file out: they wait until it is dropped.
pub type FileHold = Box<dyn Send>;

/// The store of the bytes of files.
pub trait FileStore: Send + Sync {
    /// What the first bytes of a file say it is. Reads nothing from disk.
    fn sniff(&self, head: &[u8]) -> Sniffed;

    /// Waits until nobody else holds the file with this hash, and holds it until the answer is
    /// dropped. Whoever puts a file into the store and adds the attachment that points to it, and
    /// whoever deletes the bytes of a file that nothing points to, hold it for the whole of that,
    /// so the one cannot take the bytes away between the other's two steps.
    fn hold(&self, hash: Digest) -> BoxFuture<'_, FileHold>;

    /// Whether the disk has room for `bytes` more and still keeps
    /// [`DISK_KEPT_FREE_BYTES`](bb_core::files::DISK_KEPT_FREE_BYTES) free.
    fn has_room(&self, bytes: u64) -> BoxFuture<'_, io::Result<bool>>;

    /// Starts the place an upload is put together in.
    fn begin_upload(&self, upload: UploadId) -> BoxFuture<'_, io::Result<()>>;

    /// How many bytes of an upload the server has; none for an upload it does not know.
    fn received(&self, upload: UploadId) -> BoxFuture<'_, io::Result<u64>>;

    /// Adds a part that starts at `offset`, which must be the number of bytes the server has;
    /// answers the number it has now. Two parts at the same moment are put one after the other.
    fn write_part(
        &self,
        upload: UploadId,
        offset: u64,
        bytes: Vec<u8>,
    ) -> BoxFuture<'_, Result<u64, PartError>>;

    /// Reads all of an upload: its hash, its size and what its content says it is.
    fn inspect_upload(&self, upload: UploadId) -> BoxFuture<'_, io::Result<Inspected>>;

    /// Moves a finished upload into the store under `hash`; when the store has that file already,
    /// the upload is just removed (a file is stored once, ATT-02). The file is there afterwards,
    /// also if it was missing while the store knew its hash.
    fn keep_upload(&self, upload: UploadId, hash: Digest) -> BoxFuture<'_, io::Result<()>>;

    /// Throws an upload away.
    fn discard_upload(&self, upload: UploadId) -> BoxFuture<'_, io::Result<()>>;

    /// Opens a file or its preview; `None` when it is not there.
    fn open(&self, hash: Digest, target: LinkTarget)
    -> BoxFuture<'_, io::Result<Option<FileBody>>>;

    /// Creates the preview of a stored file in a process of its own, with limits (ADR-0032 §4), and
    /// keeps it: [`PreviewState::Ready`] when there is one, [`PreviewState::Without`] when the file
    /// is too big, takes too long, is broken or crashes the process. Either way the server goes on.
    fn create_preview(
        &self,
        hash: Digest,
        kind: PreviewKind,
    ) -> BoxFuture<'_, io::Result<PreviewState>>;

    /// Deletes a file and its preview. A file that is not there is no error.
    fn delete(&self, hash: Digest) -> BoxFuture<'_, io::Result<()>>;
}

/// The store of a server that has none (the services of a test that never touches files): every
/// call fails.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoFileStore;

fn unavailable<T: Send + 'static>() -> BoxFuture<'static, io::Result<T>> {
    Box::pin(async { Err(io::Error::from(io::ErrorKind::Unsupported)) })
}

impl FileStore for NoFileStore {
    fn sniff(&self, _head: &[u8]) -> Sniffed {
        Sniffed {
            media_type: MediaType::unknown(),
            extension: None,
        }
    }

    fn hold(&self, _hash: Digest) -> BoxFuture<'_, FileHold> {
        Box::pin(async { Box::new(()) as FileHold })
    }

    fn has_room(&self, _bytes: u64) -> BoxFuture<'_, io::Result<bool>> {
        unavailable()
    }

    fn begin_upload(&self, _upload: UploadId) -> BoxFuture<'_, io::Result<()>> {
        unavailable()
    }

    fn received(&self, _upload: UploadId) -> BoxFuture<'_, io::Result<u64>> {
        unavailable()
    }

    fn write_part(
        &self,
        _upload: UploadId,
        _offset: u64,
        _bytes: Vec<u8>,
    ) -> BoxFuture<'_, Result<u64, PartError>> {
        Box::pin(async { Err(PartError::Io(io::Error::from(io::ErrorKind::Unsupported))) })
    }

    fn inspect_upload(&self, _upload: UploadId) -> BoxFuture<'_, io::Result<Inspected>> {
        unavailable()
    }

    fn keep_upload(&self, _upload: UploadId, _hash: Digest) -> BoxFuture<'_, io::Result<()>> {
        unavailable()
    }

    fn discard_upload(&self, _upload: UploadId) -> BoxFuture<'_, io::Result<()>> {
        unavailable()
    }

    fn open(
        &self,
        _hash: Digest,
        _target: LinkTarget,
    ) -> BoxFuture<'_, io::Result<Option<FileBody>>> {
        unavailable()
    }

    fn create_preview(
        &self,
        _hash: Digest,
        _kind: PreviewKind,
    ) -> BoxFuture<'_, io::Result<PreviewState>> {
        unavailable()
    }

    fn delete(&self, _hash: Digest) -> BoxFuture<'_, io::Result<()>> {
        unavailable()
    }
}
