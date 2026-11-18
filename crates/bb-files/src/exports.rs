//! The files of *Download my data* on disk (ADR-0044, decision 2): one ZIP for each export in the
//! `exports/` folder of the data folder, named by the ID of its record. A file is written under a
//! name of its own first and gets its real name only when it is whole, so a download never finds a
//! half-written file.

use std::fs::File;
use std::io::{self, Seek, SeekFrom, Write};
use std::path::PathBuf;

use bb_app::files::FileBody;
use bb_app::ports::BoxFuture;
use bb_app::privacy::export::{ExportFiles, ExportSink};
use bb_core::id::DataExportId;

use crate::disk::DiskReader;

/// The ending of a file of an export while it is written.
const WRITING: &str = "part";
/// The ending of a finished file.
const ZIP: &str = "zip";

/// The files of exports in one folder.
#[derive(Clone, Debug)]
pub struct DiskExports {
    root: PathBuf,
}

impl DiskExports {
    /// The files of exports in `root` (the `exports/` folder, which the server prepares owner-only).
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn path(&self, export: DataExportId, ending: &str) -> PathBuf {
        self.root.join(format!("{export}.{ending}"))
    }
}

/// A file being written, and where it goes when it is whole.
struct DiskSink {
    file: File,
    writing: PathBuf,
    done: PathBuf,
}

impl Write for DiskSink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.file.write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

impl Seek for DiskSink {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        self.file.seek(to)
    }
}

impl ExportSink for DiskSink {
    fn keep(self: Box<Self>) -> io::Result<u64> {
        let Self {
            file,
            writing,
            done,
        } = *self;
        file.sync_all()?;
        let size = file.metadata()?.len();
        drop(file);
        std::fs::rename(&writing, &done)?;
        Ok(size)
    }
}

/// Removes a file; a file that is not there is no error.
async fn remove(path: PathBuf) -> io::Result<()> {
    match tokio::fs::remove_file(path).await {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

impl ExportFiles for DiskExports {
    fn create(&self, export: DataExportId) -> io::Result<Box<dyn ExportSink>> {
        std::fs::create_dir_all(&self.root)?;
        let writing = self.path(export, WRITING);
        let file = File::create(&writing)?;
        Ok(Box::new(DiskSink {
            file,
            writing,
            done: self.path(export, ZIP),
        }))
    }

    fn open(&self, export: DataExportId) -> BoxFuture<'_, io::Result<Option<FileBody>>> {
        Box::pin(async move {
            let file = match tokio::fs::File::open(self.path(export, ZIP)).await {
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

    fn delete(&self, export: DataExportId) -> BoxFuture<'_, io::Result<()>> {
        Box::pin(async move {
            remove(self.path(export, WRITING)).await?;
            remove(self.path(export, ZIP)).await
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use bb_core::id::{DataExportId, SequentialIds};

    use super::*;

    #[tokio::test]
    async fn priv_03_a_file_of_an_export_is_found_only_once_it_is_whole() -> Result<(), String> {
        let folder = tempfile::tempdir().map_err(|error| error.to_string())?;
        let exports = DiskExports::new(folder.path().join("exports"));
        let id = DataExportId::generate(&mut SequentialIds::default())
            .map_err(|error| error.to_string())?;
        let mut sink = exports.create(id).map_err(|error| error.to_string())?;
        sink.write_all(b"zip").map_err(|error| error.to_string())?;
        assert!(
            exports
                .open(id)
                .await
                .map_err(|error| error.to_string())?
                .is_none()
        );
        assert_eq!(sink.keep().map_err(|error| error.to_string())?, 3);
        let mut body = exports
            .open(id)
            .await
            .map_err(|error| error.to_string())?
            .ok_or("the file is whole")?;
        assert_eq!(body.size, 3);
        let chunk = body
            .reader
            .next_chunk()
            .await
            .map_err(|error| error.to_string())?;
        assert_eq!(chunk.as_deref(), Some(&b"zip"[..]));
        exports
            .delete(id)
            .await
            .map_err(|error| error.to_string())?;
        assert!(
            exports
                .open(id)
                .await
                .map_err(|error| error.to_string())?
                .is_none()
        );
        // Deleting again is no error.
        exports
            .delete(id)
            .await
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}
