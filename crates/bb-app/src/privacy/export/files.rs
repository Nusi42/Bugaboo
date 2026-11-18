//! The port of the files of *Download my data* (ADR-0044, decision 2): where the ZIP of an export is
//! written, read for its download and deleted with its record. The adapter keeps them in the
//! `exports/` folder of the data folder, which only the account of the server may open; a file is
//! named by the ID of its record and nothing a person typed.

use std::io::{self, Seek, Write};

use bb_core::id::DataExportId;

use crate::files::FileBody;
use crate::ports::BoxFuture;

/// A file of an export being written. Nobody can download it before [`Self::keep`].
pub trait ExportSink: Write + Seek + Send {
    /// Makes the written file the file of its export; answers its size in bytes.
    ///
    /// # Errors
    /// The error of the file system.
    fn keep(self: Box<Self>) -> io::Result<u64>;
}

/// Where the files of exports are kept.
pub trait ExportFiles: Send + Sync {
    /// Starts the file of `export`; a half-written one from an earlier try is replaced.
    ///
    /// # Errors
    /// The error of the file system.
    fn create(&self, export: DataExportId) -> io::Result<Box<dyn ExportSink>>;

    /// Opens the file of `export`; `None` when it is not there.
    fn open(&self, export: DataExportId) -> BoxFuture<'_, io::Result<Option<FileBody>>>;

    /// Deletes the file of `export`, written or half-written. A file that is not there is no error.
    fn delete(&self, export: DataExportId) -> BoxFuture<'_, io::Result<()>>;
}

/// The files of a server that has none (the services of a test that never exports): every call
/// fails.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoExportFiles;

impl ExportFiles for NoExportFiles {
    fn create(&self, _export: DataExportId) -> io::Result<Box<dyn ExportSink>> {
        Err(io::Error::from(io::ErrorKind::Unsupported))
    }

    fn open(&self, _export: DataExportId) -> BoxFuture<'_, io::Result<Option<FileBody>>> {
        Box::pin(async { Err(io::Error::from(io::ErrorKind::Unsupported)) })
    }

    fn delete(&self, _export: DataExportId) -> BoxFuture<'_, io::Result<()>> {
        Box::pin(async { Err(io::Error::from(io::ErrorKind::Unsupported)) })
    }
}
