//! The backups part of the in-memory store (OPS-07), and backup files kept in memory.

use std::sync::Mutex;

use bb_core::backup::name::{BackupName, Purpose};
use bb_core::backup::run::{BackupRun, CreatedFile};
use bb_core::security_log::Digest;
use jiff::Timestamp;

use super::fake::{FakeError, FakeStore, Tx};
use crate::backups::{BackupFile, BackupFiles, BackupStore};
use crate::ports::BoxFuture;

impl BackupStore for FakeStore {
    async fn backup_runs(&self, limit: u64) -> Result<Vec<BackupRun>, FakeError> {
        Ok(self.read(|state| {
            let mut runs = state.backup_runs.clone();
            runs.sort_by_key(|run| std::cmp::Reverse(run.at));
            runs.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
            runs
        }))
    }

    async fn insert_backup_run(tx: &mut Tx, run: &BackupRun) -> Result<(), FakeError> {
        tx.state.backup_runs.push(run.clone());
        Ok(())
    }
}

/// Backup files kept in memory: a name and a size each.
#[derive(Default)]
pub(crate) struct MemoryBackups {
    pub(crate) files: Mutex<Vec<BackupFile>>,
}

impl BackupFiles for MemoryBackups {
    fn create(
        &self,
        _folder: Option<String>,
        purpose: Purpose,
        now: Timestamp,
    ) -> BoxFuture<'_, std::io::Result<CreatedFile>> {
        let name = BackupName::new(purpose, now, "0.6.0");
        let mut files = self
            .files
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        files.push(BackupFile {
            name: name.clone(),
            size: 100,
        });
        Box::pin(async move {
            Ok(CreatedFile {
                name,
                size: 100,
                checksum: Digest::new([7; 32]),
            })
        })
    }

    fn list(&self, _folder: Option<String>) -> BoxFuture<'_, std::io::Result<Vec<BackupFile>>> {
        let files = self
            .files
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        Box::pin(async move { Ok(files) })
    }

    fn delete(
        &self,
        _folder: Option<String>,
        name: BackupName,
    ) -> BoxFuture<'_, std::io::Result<()>> {
        self.files
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain(|file| file.name != name);
        Box::pin(async { Ok(()) })
    }
}
