//! The ports of backups: the files (the server copies the database and the files and writes the
//! archive) and the records of the backups (the store).

use bb_core::backup::name::{BackupName, Purpose};
use bb_core::backup::run::{BackupRun, CreatedFile};
use std::task::Poll;

use jiff::Timestamp;

use crate::agent_work::Pause;
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

/// What a job asks while a piece of its work may take longer than its lease, so the job keeps its
/// lease (ADR-0017): a copy of a large database or a step that finishes a removal on a large
/// organization takes longer than any fixed lease, and another server must not take the job and
/// do the work twice meanwhile.
pub struct KeepLease<'a> {
    /// How often the lease is renewed: well within its length.
    pub every: std::time::Duration,
    /// Renews the lease; `false` when another worker holds the job now, and the work stops.
    pub renew: Box<dyn Fn() -> BoxFuture<'a, bool> + Send + Sync + 'a>,
}

impl KeepLease<'_> {
    /// Runs `work` while the lease of its job is renewed every [`KeepLease::every`], with time
    /// passing by `pause`; `None` when another worker holds the job now, and `work` was stopped
    /// (dropped).
    pub async fn while_kept<T>(
        self,
        work: impl Future<Output = T>,
        pause: &dyn Pause,
    ) -> Option<T> {
        let mut work = std::pin::pin!(work);
        loop {
            // The lease was taken or renewed just now: the first renewal comes after `every`.
            let mut over = pause.pause(self.every);
            let done = std::future::poll_fn(|context| {
                if let Poll::Ready(done) = work.as_mut().poll(context) {
                    return Poll::Ready(Some(done));
                }
                over.as_mut().poll(context).map(|()| None)
            })
            .await;
            if let Some(done) = done {
                return Some(done);
            }
            if !(self.renew)().await {
                return None;
            }
        }
    }
}

/// The backup files. `folder` is the folder the setting `backups.folder` names, or `None` for the
/// backups folder of the data folder.
pub trait BackupFiles: Send + Sync {
    /// Creates a backup into the folder while people go on working (ADR-0046, decision 2); with
    /// `keep`, the lease of the job that makes it is renewed while it works.
    fn create<'a>(
        &'a self,
        folder: Option<String>,
        purpose: Purpose,
        now: Timestamp,
        keep: Option<KeepLease<'a>>,
    ) -> BoxFuture<'a, std::io::Result<CreatedFile>>;

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
    fn create<'a>(
        &'a self,
        _folder: Option<String>,
        _purpose: Purpose,
        _now: Timestamp,
        _keep: Option<KeepLease<'a>>,
    ) -> BoxFuture<'a, std::io::Result<CreatedFile>> {
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
        /// Holds the asks for a backup until the end of the change: two asks at the same moment
        /// cannot both find no backup asked for that is not made yet.
        fn lock_backup_asks() -> ();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use super::KeepLease;
    use crate::tests::agent_waiting::RuntimePause;

    /// Renews every millisecond and counts the renewals; `held` is what each renewal answers.
    fn keep(renewed: &AtomicUsize, held: bool) -> KeepLease<'_> {
        KeepLease {
            every: Duration::from_millis(1),
            renew: Box::new(move || {
                renewed.fetch_add(1, Ordering::SeqCst);
                Box::pin(async move { held })
            }),
        }
    }

    #[tokio::test]
    async fn adr_0017_work_that_keeps_its_lease_is_renewed_and_stops_when_another_worker_has_it() {
        // While the worker holds the job, the lease is renewed and the work ends.
        let renewed = AtomicUsize::new(0);
        let done = keep(&renewed, true)
            .while_kept(
                async {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    7
                },
                &RuntimePause,
            )
            .await;
        assert_eq!(done, Some(7));
        assert!(renewed.load(Ordering::SeqCst) > 0, "the lease was renewed");
        // Work that ends before the first renewal is not held up.
        let early = AtomicUsize::new(0);
        assert_eq!(
            keep(&early, true)
                .while_kept(async { 8 }, &RuntimePause)
                .await,
            Some(8)
        );
        assert_eq!(early.load(Ordering::SeqCst), 0);
        // Once another worker has it, the work stops at the next renewal.
        let lost = AtomicUsize::new(0);
        let stopped = keep(&lost, false)
            .while_kept(std::future::pending::<()>(), &RuntimePause)
            .await;
        assert_eq!(stopped, None);
        assert_eq!(lost.load(Ordering::SeqCst), 1);
    }
}
