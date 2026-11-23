//! The backups part of the in-memory store (OPS-07), and backup files kept in memory.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use bb_core::backup::name::{BackupName, Purpose};
use bb_core::backup::run::{BackupRun, CreatedFile};
use bb_core::security_log::Digest;
use jiff::Timestamp;

use super::fake::{FakeError, FakeStore, Tx};
use crate::backups::{BackupFile, BackupFiles, BackupStore, KeepLease};
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

    async fn lock_backup_asks(_tx: &mut Tx) -> Result<(), FakeError> {
        Ok(())
    }
}

/// How a folder on a mapped network drive of the fake begins: the system would say it lies on
/// another computer.
pub(crate) const A_SHARE_BEHIND_A_DRIVE: &str = "Z:\\";

/// Backup files kept in memory: a name and a size each.
#[derive(Default)]
pub(crate) struct MemoryBackups {
    pub(crate) files: Mutex<Vec<BackupFile>>,
    /// The folders whose leftovers were removed, in order.
    pub(crate) cleaned: Mutex<Vec<Option<String>>>,
    /// Whether a backup fails (the disk is full).
    pub(crate) broken: AtomicBool,
}

impl BackupFiles for MemoryBackups {
    fn on_another_computer(&self, folder: &str) -> bool {
        folder.starts_with(A_SHARE_BEHIND_A_DRIVE)
    }

    fn create<'a>(
        &'a self,
        _folder: Option<String>,
        purpose: Purpose,
        now: Timestamp,
        _keep: Option<KeepLease<'a>>,
    ) -> BoxFuture<'a, std::io::Result<CreatedFile>> {
        if self.broken.load(Ordering::SeqCst) {
            return Box::pin(async { Err(std::io::Error::other("the disk is full")) });
        }
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

    fn remove_leftovers(&self, folder: Option<String>) -> BoxFuture<'_, ()> {
        self.cleaned
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(folder);
        Box::pin(async {})
    }
}
