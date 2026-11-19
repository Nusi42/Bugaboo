//! The files of *Download my data* on disk (ADR-0044, decision 2): one ZIP for each export in the
//! `exports/` folder of the data folder, named by the ID of its record. Each try writes a file of its
//! own first (`<export>.<try>.part`, made new, never opened twice), and the file gets its real name
//! only when it is whole, so a download never finds a half-written file and two tries of one export
//! never write into one file.

use std::fs::{File, OpenOptions};
use std::io::{self, Seek, SeekFrom, Write};
use std::path::PathBuf;

use bb_app::files::FileBody;
use bb_app::ports::BoxFuture;
use bb_app::privacy::export::{ExportFiles, ExportSink};
use bb_core::id::DataExportId;

use crate::disk::DiskReader;

/// The ending of a file of an export while a try writes it.
const WRITING: &str = "part";
/// The ending of a finished file.
const ZIP: &str = "zip";

/// The files of exports in one folder.
#[derive(Clone, Debug)]
pub struct DiskExports {
    root: PathBuf,
}

impl DiskExports {
    /// The files of exports in `root`, the `exports/` folder, which the server prepares owner-only
    /// when it starts. A missing folder is an error, never made again here with other rights.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn done(&self, export: DataExportId) -> PathBuf {
        self.root.join(format!("{export}.{ZIP}"))
    }

    /// Whether `name` is a half-written file of `export`, of any try.
    fn is_part_of(name: &str, export: DataExportId) -> bool {
        name.strip_prefix(&format!("{export}."))
            .and_then(|rest| rest.strip_suffix(&format!(".{WRITING}")))
            .is_some_and(|attempt| !attempt.is_empty())
    }
}

/// A file being written by one try, and where it goes when it is whole. `writing` is `None` once
/// the file has its real name.
struct DiskSink {
    file: File,
    writing: Option<PathBuf>,
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
    fn finish(&mut self) -> io::Result<u64> {
        self.file.flush()?;
        self.file.sync_all()?;
        Ok(self.file.metadata()?.len())
    }

    fn keep(mut self: Box<Self>) -> io::Result<()> {
        if let Some(writing) = self.writing.take()
            && let Err(error) = std::fs::rename(&writing, &self.done)
        {
            self.writing = Some(writing);
            return Err(error);
        }
        // A crash after the rename could otherwise lose the file whose record says it is ready.
        crate::sync_folder_of(&self.done)
    }
}

impl Drop for DiskSink {
    /// A try that stops before its file is kept leaves nothing behind.
    fn drop(&mut self) {
        if let Some(writing) = self.writing.take()
            && let Err(error) = std::fs::remove_file(writing)
            && error.kind() != io::ErrorKind::NotFound
        {
            tracing::warn!(cause = %error.kind(), "a half-written file of an export stayed");
        }
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
    fn create(&self, export: DataExportId, attempt: &str) -> io::Result<Box<dyn ExportSink>> {
        // The name of a try holds letters, digits and `_` only, so it never leaves the folder.
        let attempt: String = attempt
            .chars()
            .map(|letter| {
                if letter.is_ascii_alphanumeric() {
                    letter
                } else {
                    '_'
                }
            })
            .collect();
        if attempt.is_empty() {
            return Err(io::Error::from(io::ErrorKind::InvalidInput));
        }
        let writing = self.root.join(format!("{export}.{attempt}.{WRITING}"));
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&writing)?;
        Ok(Box::new(DiskSink {
            file,
            writing: Some(writing),
            done: self.done(export),
        }))
    }

    fn open(&self, export: DataExportId) -> BoxFuture<'_, io::Result<Option<FileBody>>> {
        Box::pin(async move {
            let file = match tokio::fs::File::open(self.done(export)).await {
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
            let mut entries = match tokio::fs::read_dir(&self.root).await {
                Ok(entries) => entries,
                Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
                Err(error) => return Err(error),
            };
            while let Some(entry) = entries.next_entry().await? {
                if entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| Self::is_part_of(name, export))
                {
                    remove(entry.path()).await?;
                }
            }
            remove(self.done(export)).await
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use bb_core::id::{DataExportId, SequentialIds};

    use super::*;

    fn text(error: impl std::fmt::Display) -> String {
        error.to_string()
    }

    /// A folder of exports that exists, as the server prepares it, and an export.
    fn folder() -> Result<(tempfile::TempDir, DiskExports, DataExportId), String> {
        let folder = tempfile::tempdir().map_err(text)?;
        let exports = DiskExports::new(folder.path());
        let id = DataExportId::generate(&mut SequentialIds::default()).map_err(text)?;
        Ok((folder, exports, id))
    }

    /// The names of the files in the folder.
    fn names(folder: &tempfile::TempDir) -> Result<Vec<String>, String> {
        let mut names = std::fs::read_dir(folder.path())
            .map_err(text)?
            .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().into_owned()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(text)?;
        names.sort();
        Ok(names)
    }

    async fn bytes_of(exports: &DiskExports, id: DataExportId) -> Result<Option<Vec<u8>>, String> {
        let Some(mut body) = exports.open(id).await.map_err(text)? else {
            return Ok(None);
        };
        let mut bytes = Vec::new();
        while let Some(chunk) = body.reader.next_chunk().await.map_err(text)? {
            bytes.extend(chunk);
        }
        Ok(Some(bytes))
    }

    #[tokio::test]
    async fn priv_03_a_file_of_an_export_is_found_only_once_it_is_whole() -> Result<(), String> {
        let (_folder, exports, id) = folder()?;
        let mut sink = exports.create(id, "worker.one").map_err(text)?;
        sink.write_all(b"zip").map_err(text)?;
        assert!(bytes_of(&exports, id).await?.is_none());
        assert_eq!(sink.finish().map_err(text)?, 3);
        sink.keep().map_err(text)?;
        assert_eq!(bytes_of(&exports, id).await?.as_deref(), Some(&b"zip"[..]));
        exports.delete(id).await.map_err(text)?;
        assert!(bytes_of(&exports, id).await?.is_none());
        // Deleting again is no error.
        exports.delete(id).await.map_err(text)?;
        Ok(())
    }

    #[tokio::test]
    async fn priv_03_two_tries_of_one_export_write_files_of_their_own() -> Result<(), String> {
        let (folder, exports, id) = folder()?;
        let mut first = exports.create(id, "worker.one").map_err(text)?;
        let mut second = exports.create(id, "worker.two").map_err(text)?;
        first.write_all(b"first").map_err(text)?;
        second.write_all(b"second").map_err(text)?;
        // The same try cannot open its file twice.
        assert!(exports.create(id, "worker.one").is_err());
        assert_eq!(names(&folder)?.len(), 2);
        second.finish().map_err(text)?;
        second.keep().map_err(text)?;
        assert_eq!(
            bytes_of(&exports, id).await?.as_deref(),
            Some(&b"second"[..])
        );
        // A try that stops before its file is kept leaves nothing behind.
        drop(first);
        assert_eq!(names(&folder)?, vec![format!("{id}.zip")]);
        Ok(())
    }

    #[tokio::test]
    async fn priv_03_deleting_an_export_deletes_the_files_of_every_try() -> Result<(), String> {
        let (folder, exports, _) = folder()?;
        let mut ids = SequentialIds::default();
        let id = DataExportId::generate(&mut ids).map_err(text)?;
        let other = DataExportId::generate(&mut ids).map_err(text)?;
        // Files a stopped server left: two tries of the export, and a file of another export.
        std::fs::write(folder.path().join(format!("{id}.worker_one.part")), b"a").map_err(text)?;
        std::fs::write(folder.path().join(format!("{id}.worker_two.part")), b"b").map_err(text)?;
        std::fs::write(folder.path().join(format!("{id}.zip")), b"c").map_err(text)?;
        std::fs::write(folder.path().join(format!("{other}.zip")), b"d").map_err(text)?;
        exports.delete(id).await.map_err(text)?;
        assert_eq!(names(&folder)?, vec![format!("{other}.zip")]);
        Ok(())
    }

    #[test]
    fn priv_03_a_missing_folder_of_exports_is_not_made_again() -> Result<(), String> {
        let (folder, _, id) = folder()?;
        let exports = DiskExports::new(folder.path().join(crate::EXPORTS_FOLDER));
        assert!(exports.create(id, "worker.one").is_err());
        assert!(!folder.path().join(crate::EXPORTS_FOLDER).exists());
        Ok(())
    }
}
