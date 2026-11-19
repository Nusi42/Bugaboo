//! The port of the files of *Download my data* (ADR-0044, decision 2): where the ZIP of an export is
//! written, read for its download and deleted with its record. The adapter keeps them in the
//! `exports/` folder of the data folder, which only the account of the server may open; a file is
//! named by the ID of its record and nothing a person typed.

use std::io::{self, Seek, Write};

use bb_core::id::DataExportId;

use crate::files::FileBody;
use crate::ports::BoxFuture;

/// A file of an export being written by one try. Nobody can download it before [`Self::keep`], and
/// one that is dropped before is deleted.
pub trait ExportSink: Write + Seek + Send {
    /// Writes what is still buffered to the disk; answers the size of the file in bytes. Blocks.
    ///
    /// # Errors
    /// The error of the file system.
    fn finish(&mut self) -> io::Result<u64>;

    /// Makes the finished file the file of its export, in place of one an earlier try left. Blocks.
    ///
    /// # Errors
    /// The error of the file system.
    fn keep(self: Box<Self>) -> io::Result<()>;
}

/// Where the files of exports are kept.
pub trait ExportFiles: Send + Sync {
    /// Starts a file of `export` for the try `attempt` under a name of its own, so two tries of one
    /// export never write into one file. Blocks.
    ///
    /// # Errors
    /// The error of the file system, also when the folder of the exports is missing.
    fn create(&self, export: DataExportId, attempt: &str) -> io::Result<Box<dyn ExportSink>>;

    /// Opens the file of `export`; `None` when it is not there.
    fn open(&self, export: DataExportId) -> BoxFuture<'_, io::Result<Option<FileBody>>>;

    /// Deletes the file of `export` and every half-written one of its tries. A file that is not
    /// there is no error.
    fn delete(&self, export: DataExportId) -> BoxFuture<'_, io::Result<()>>;
}

/// The files of a server that has none (the services of a test that never exports): every call
/// fails.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoExportFiles;

impl ExportFiles for NoExportFiles {
    fn create(&self, _export: DataExportId, _attempt: &str) -> io::Result<Box<dyn ExportSink>> {
        Err(io::Error::from(io::ErrorKind::Unsupported))
    }

    fn open(&self, _export: DataExportId) -> BoxFuture<'_, io::Result<Option<FileBody>>> {
        Box::pin(async { Err(io::Error::from(io::ErrorKind::Unsupported)) })
    }

    fn delete(&self, _export: DataExportId) -> BoxFuture<'_, io::Result<()>> {
        Box::pin(async { Err(io::Error::from(io::ErrorKind::Unsupported)) })
    }
}
