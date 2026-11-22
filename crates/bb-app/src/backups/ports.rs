//! The ports of backups: the files (the server copies the database and the files and writes the
//! archive) and the records of the backups (the store).

use bb_core::backup::name::{BackupName, Purpose};
use bb_core::backup::run::{BackupRun, CreatedFile};
use jiff::Timestamp;

use crate::ports::{BoxFuture, ChangeStore};
use crate::store_macros::{reads, writes};

/// A backup file in the folder of the backups.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackupFile {
    /// Its name.
    pub name: BackupName,
    /// Its size in bytes.
    pub size: u64,
}

/// The backup files. `folder` is the folder the setting `backups.folder` names, or `None` for the
/// backups folder of the data folder.
pub trait BackupFiles: Send + Sync {
    /// Creates a backup into the folder while people go on working (ADR-0046, decision 2).
    fn create(
        &self,
        folder: Option<String>,
        purpose: Purpose,
        now: Timestamp,
    ) -> BoxFuture<'_, std::io::Result<CreatedFile>>;

    /// The backups in the folder whose names Bugaboo reads, newest first.
    fn list(&self, folder: Option<String>) -> BoxFuture<'_, std::io::Result<Vec<BackupFile>>>;

    /// Deletes a backup of the folder.
    fn delete(
        &self,
        folder: Option<String>,
        name: BackupName,
    ) -> BoxFuture<'_, std::io::Result<()>>;

    /// Deletes from the folder what a backup that never finished left behind (the program was
    /// stopped while it copied): the copy beside a name and the empty file that held the name.
    /// What cannot be deleted stays for the next time.
    fn remove_leftovers(&self, folder: Option<String>) -> BoxFuture<'_, ()>;

    /// Whether the system says that `folder` lies on a share of another computer (a mapped
    /// network drive, a mounted share). `false` when it cannot tell.
    fn on_another_computer(&self, folder: &str) -> bool {
        let _ = folder;
        false
    }
}

/// No backup files: every call fails. What the services have until the server gives them the
/// real ones, and what the tests of other parts run with.
pub struct NoBackupFiles;

fn none<T>() -> std::io::Result<T> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "this server keeps no backups",
    ))
}

impl BackupFiles for NoBackupFiles {
    fn create(
        &self,
        _folder: Option<String>,
        _purpose: Purpose,
        _now: Timestamp,
    ) -> BoxFuture<'_, std::io::Result<CreatedFile>> {
        Box::pin(async { none() })
    }

    fn list(&self, _folder: Option<String>) -> BoxFuture<'_, std::io::Result<Vec<BackupFile>>> {
        Box::pin(async { none() })
    }

    fn delete(
        &self,
        _folder: Option<String>,
        _name: BackupName,
    ) -> BoxFuture<'_, std::io::Result<()>> {
        Box::pin(async { none() })
    }

    fn remove_leftovers(&self, _folder: Option<String>) -> BoxFuture<'_, ()> {
        Box::pin(async {})
    }
}

/// What the use cases of backups need from storage: the records of the backups (migration 54).
pub trait BackupStore: ChangeStore + 'static {
    reads! {
        /// The newest `limit` records of backups created and restored, newest first.
        fn backup_runs(limit: u64) -> Vec<BackupRun>;
    }

    writes! {
        /// Adds the record of a backup.
        fn insert_backup_run(run: &BackupRun) -> ();
    }
}
