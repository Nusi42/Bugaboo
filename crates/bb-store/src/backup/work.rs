//! The folder a backup works in (ADR-0046), and the blocking thread its file work runs on.
//!
//! A backup copies the database into a folder of its own beside the backups, writes the archive
//! there and moves it into place. The folder goes when the work ends however it ends: the last
//! holder of [`WorkFolder`] deletes it, so a backup that fails or is stopped half way (its future
//! dropped) leaves nothing, also when a blocking thread still finishes a write in it, and a backup
//! that was stopped never puts its archive among the backups ([`WorkFolder::let_go`]). The folder
//! holds a lock on a file of its own ([`HELD`]) for as long as it lives, so no other program
//! deletes the folder of a backup that runs, however long it runs and whatever times the system
//! shows; a folder of a program that was killed is deleted by the next round of the schedule once
//! nothing in it changed for an hour ([`remove_stale`]).

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use bb_core::hex;

use super::io;
use crate::error::StoreError;

/// The start of the name of the folder a backup works in, after its dot.
const LABEL: &str = "backup";

/// The file in a work folder whose lock says that the backup of the folder still runs.
pub(super) const HELD: &str = "held";

/// The folder of the work of one backup, deleted with its last holder.
#[derive(Debug)]
pub(crate) struct WorkFolder {
    path: PathBuf,
    /// The file [`HELD`], locked until the folder goes.
    held: Option<File>,
    /// Whether the backup let go of its folder before it ended ([`OwnHold`]).
    let_go: AtomicBool,
}

impl WorkFolder {
    /// A new folder for the owner only, for the work of one backup, in `near`, with the lock that
    /// keeps it from [`remove_stale`]. Blocking: call it on a blocking thread.
    pub(crate) fn new(near: &Path) -> Result<Arc<Self>, StoreError> {
        let path = new_folder(near, LABEL)?;
        let held = File::create_new(path.join(HELD)).map_err(io)?;
        held.lock().map_err(io)?;
        Ok(Arc::new(Self {
            path,
            held: Some(held),
            let_go: AtomicBool::new(false),
        }))
    }

    /// Where the folder is.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// Whether the backup let go of the folder before it ended (its future was dropped: a request
    /// that ran out of time, a job that lost its lease). A blocking thread that still finishes its
    /// work then puts nothing among the backups, so a backup nobody records never appears there.
    pub(crate) fn let_go(&self) -> bool {
        self.let_go.load(Ordering::Acquire)
    }

    /// Deletes the folder now, on this thread, when no other holder is left; otherwise the last
    /// one deletes it. Blocking: call it on a blocking thread.
    pub(crate) fn remove_now(self: Arc<Self>) {
        if let Ok(mut folder) = Arc::try_unwrap(self) {
            drop(folder.held.take());
            remove(&std::mem::take(&mut folder.path));
        }
    }
}

impl Drop for WorkFolder {
    fn drop(&mut self) {
        // The lock goes first: Windows deletes no folder with a file open in it.
        drop(self.held.take());
        let path = std::mem::take(&mut self.path);
        if path.as_os_str().is_empty() {
            return;
        }
        // A backup stopped on a worker of the runtime does not delete a large copy there.
        match tokio::runtime::Handle::try_current() {
            Ok(runtime) => drop(runtime.spawn_blocking(move || remove(&path))),
            Err(_) => remove(&path),
        }
    }
}

/// The backup's own hold on its work folder, beside the holds of the blocking threads that work in
/// it: when the backup ends or is dropped half way, dropping it says so to them
/// ([`WorkFolder::let_go`]).
pub(crate) struct OwnHold<'a>(&'a WorkFolder);

impl<'a> OwnHold<'a> {
    pub(crate) const fn of(work: &'a WorkFolder) -> Self {
        Self(work)
    }
}

impl Drop for OwnHold<'_> {
    fn drop(&mut self) {
        self.0.let_go.store(true, Ordering::Release);
    }
}

/// How often the deletion of a work folder is tried: a copy that was stopped may still hold its
/// file open for a moment (SQLite finishing `VACUUM INTO`), which Windows does not delete.
const REMOVE_TRIES: u32 = 50;

/// The wait between two tries.
const REMOVE_WAIT: std::time::Duration = std::time::Duration::from_millis(100);

/// Deletes a work folder. Blocking.
fn remove(path: &Path) {
    for tried in 1..=REMOVE_TRIES {
        match std::fs::remove_dir_all(path) {
            Ok(()) => return,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
            Err(error) if tried == REMOVE_TRIES => {
                tracing::warn!(cause = %error, "the work folder of a backup stays; the schedule deletes it later");
            }
            Err(_) => std::thread::sleep(REMOVE_WAIT),
        }
    }
}

/// A new folder for the owner only, for the work of one backup or restore, beside `near`.
pub(super) fn new_folder(near: &Path, label: &str) -> Result<PathBuf, StoreError> {
    let mut random = [0_u8; 8];
    getrandom::fill(&mut random).map_err(|_| StoreError::Corrupt("no random bytes"))?;
    let folder = near.join(format!(".{label}-{}", hex::encode(&random)));
    let builder = std::fs::DirBuilder::new();
    #[cfg(unix)]
    let builder = {
        use std::os::unix::fs::DirBuilderExt as _;
        let mut builder = builder;
        builder.mode(0o700);
        builder
    };
    builder.create(&folder).map_err(io)?;
    Ok(folder)
}

/// Runs the file work of a backup on a blocking thread of the runtime, never on one of its
/// workers, which answer requests meanwhile.
pub(crate) async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, StoreError> + Send + 'static,
) -> Result<T, StoreError> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|error| io(std::io::Error::other(error)))?
}

/// Whether the backup of the work folder `path` still runs: it holds the lock on [`HELD`]. The
/// system lets the lock go with the program that held it, however that program ended.
fn still_held(path: &Path) -> bool {
    let Ok(held) = File::open(path.join(HELD)) else {
        return false;
    };
    matches!(held.try_lock(), Err(std::fs::TryLockError::WouldBlock))
}

/// Deletes from `folder` the work folders of backups that never finished (the program was killed
/// while it copied), once nothing in them changed for `after`. The folder of a backup that runs,
/// in this program or another, is never deleted: it holds its lock ([`still_held`]).
pub(super) fn remove_stale(folder: &Path, after: std::time::Duration) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    let now = std::time::SystemTime::now();
    let prefix = format!(".{LABEL}-");
    let old = |data: &std::fs::Metadata| {
        data.modified()
            .ok()
            .and_then(|changed| now.duration_since(changed).ok())
            .is_some_and(|age| age >= after)
    };
    for entry in entries.flatten() {
        let ours = entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with(&prefix));
        let path = entry.path();
        if !ours
            || !std::fs::symlink_metadata(&path).is_ok_and(|data| data.is_dir())
            || still_held(&path)
        {
            continue;
        }
        let Ok(inside) = std::fs::read_dir(&path) else {
            continue;
        };
        let mut files = inside.flatten().peekable();
        let stale = if files.peek().is_none() {
            std::fs::metadata(&path).is_ok_and(|data| old(&data))
        } else {
            files.all(|file| file.metadata().is_ok_and(|data| old(&data)))
        };
        if stale {
            remove(&path);
        }
    }
}
