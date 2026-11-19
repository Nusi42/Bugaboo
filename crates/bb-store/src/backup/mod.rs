//! Backups (OPS-07, ADR-0046): making one while people work, and restoring one into an empty
//! database of the same kind with the server stopped.
//!
//! What a backup is, is described once in [`bb_core::backup`]. Here is what touches the disk and
//! the database: the copy of the database, one routine per database (`copy.rs`), the archive
//! (`archive.rs`), the records of the backups made and restored (`runs.rs`) and the restore
//! (`restore.rs`).

mod archive;
mod catalog;
mod copy;
mod fill;
mod restore;
mod runs;
mod seal;
mod value;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use bb_core::backup::DatabaseKind;
use bb_core::backup::manifest::{AppliedMigration, FORMAT, LogHead, Manifest};
use bb_core::backup::member::{self, MemberName};
use bb_core::backup::name::{BackupName, EXTENSION, Purpose};
use bb_core::backup::run::CreatedFile;
use bb_core::hex;
use bb_core::security_log::Digest;
use jiff::Timestamp;

use crate::db::{Database, Dialect};
use crate::error::StoreError;
use crate::keys::KeyRing;

pub(crate) use restore::put_back_before_upgrade;
pub use restore::{RestoreFailure, RestoreInput, RestoreReport, RestoreTarget, restore};

/// The version of Bugaboo that makes a backup, as its name and manifest say.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The paths of the stored file with this hash, inside the `files/` folder, with `/` between the
/// parts: the file and its preview. The file adapter knows them; the store does not.
pub type FilePaths = Arc<dyn Fn(&Digest) -> Vec<String> + Send + Sync>;

/// Where a backup goes and what it copies besides the database.
#[derive(Clone)]
pub struct BackupPlaces {
    /// The folder of the backups (`backups/` of the data folder, or the one an administrator
    /// named). It must be the owner's only.
    pub folder: PathBuf,
    /// The `files/` folder of the data folder.
    pub files: PathBuf,
    /// The replay list of the data folder (ADR-0044, decision 8). A list that is not there is an
    /// empty one.
    pub replay_list: PathBuf,
    /// Where each stored file lies inside `files`.
    pub file_paths: FilePaths,
    /// The keys of the installation: a backup carries a seal and the check values of its master
    /// keys, and a restore checks both before anything changes.
    pub keys: Arc<KeyRing>,
}

/// A backup that was created.
#[derive(Clone, Debug)]
pub struct CreatedBackup {
    /// Its name, size and checksum.
    pub file: CreatedFile,
    /// The file.
    pub path: PathBuf,
    /// The newest place and hash of the security log in it.
    pub log_head: Option<(i64, Digest)>,
}

fn io(error: std::io::Error) -> StoreError {
    StoreError::Backup(error)
}

/// The kind of database a dialect is, as a backup names it.
const fn kind_of(dialect: Dialect) -> DatabaseKind {
    match dialect {
        Dialect::Sqlite => DatabaseKind::Sqlite,
        Dialect::Postgres => DatabaseKind::Postgres,
    }
}

/// A new folder for the owner only, for the work of one backup or restore, beside `near`.
fn work_folder(near: &Path, label: &str) -> Result<PathBuf, StoreError> {
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

/// The backups in `folder` whose names this program reads, newest first.
///
/// # Errors
/// Fails when the folder cannot be read.
pub fn backups_in(folder: &Path) -> Result<Vec<(BackupName, PathBuf, u64)>, StoreError> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(folder).map_err(io)? {
        let entry = entry.map_err(io)?;
        let data = entry.metadata().map_err(io)?;
        let name = entry.file_name();
        let parsed = name.to_str().and_then(BackupName::parse);
        // An empty file is a name being taken by a backup that is still copied (`place_new`).
        if let (true, true, Some(parsed)) = (data.is_file(), data.len() > 0, parsed) {
            found.push((parsed, entry.path(), data.len()));
        }
    }
    found.sort_by_key(|(name, _, _)| std::cmp::Reverse(name.moment));
    Ok(found)
}

/// How many seconds back a name is looked for when backups follow each other within a second.
const NAME_TRIES: i64 = 60;

/// Puts the finished archive `staged` into `folder` under the first name that is free at `moment`
/// or a few seconds before: an earlier moment keeps the promise of the name (every change saved
/// before it is in the backup), a later one would not. A backup made at the same moment beside
/// this one (the command line beside the server) that takes a name first makes this one take the
/// next (M9-F17).
fn place_under_free_name(
    staged: &Path,
    folder: &Path,
    purpose: Purpose,
    moment: Timestamp,
) -> Result<(BackupName, PathBuf), StoreError> {
    for back in 0..NAME_TRIES {
        let earlier = moment
            .checked_sub(jiff::SignedDuration::from_secs(back))
            .map_err(|_| StoreError::Corrupt("the moment of a backup"))?;
        let name = BackupName::new(purpose, earlier, VERSION);
        let target = folder.join(name.to_string());
        match place_backup(staged, &target) {
            Ok(()) => return Ok((name, target)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(io(error)),
        }
    }
    Err(io(std::io::ErrorKind::AlreadyExists.into()))
}

/// What a file being copied under a name of its own carries after that name, until it is renamed
/// onto the name ([`place_new`]).
pub const PARTIAL: &str = ".partial";

/// Puts the finished archive `staged` at `target`, which must not be there, through [`place_new`]
/// with a hard link and files for the owner only.
fn place_backup(staged: &Path, target: &Path) -> std::io::Result<()> {
    place_new(
        staged,
        target,
        |from, to| std::fs::hard_link(from, to),
        archive::create_private,
    )
}

/// Puts the file `staged` at `target`, which must not be there, and never shows it half written
/// under that name: the one way Bugaboo places a new file that must not replace one (a backup, a
/// copy of the key files, the setup code).
///
/// `link` is tried first: a hard link fails when the name is taken, so two writers can never
/// take one name. A drive without links (FAT32, exFAT, some network drives) makes it fail for
/// another reason: then the name is taken by an empty file made new with `create` (which must
/// fail with `AlreadyExists` when the name is taken, as the link does), the copy goes into
/// `<target>.partial`, also made with `create`, and that file is renamed over the empty one
/// (M9-F16). Until then the name holds an empty file, which [`backups_in`] does not list. A copy
/// that fails takes both files back.
///
/// # Errors
/// `AlreadyExists` when `target` is there, and the errors of the disk.
pub fn place_new(
    staged: &Path,
    target: &Path,
    link: impl FnOnce(&Path, &Path) -> std::io::Result<()>,
    create: impl Fn(&Path) -> std::io::Result<std::fs::File>,
) -> std::io::Result<()> {
    match link(staged, target) {
        Err(error) if error.kind() != std::io::ErrorKind::AlreadyExists => {
            drop(create(target)?);
            let beside = partial_of(target);
            let copied = copy_whole(staged, &beside, &create)
                .and_then(|()| std::fs::rename(&beside, target));
            if copied.is_err() {
                let _ = std::fs::remove_file(&beside);
                let _ = std::fs::remove_file(target);
            }
            copied
        }
        other => other,
    }
}

/// The file beside `target` that its copy goes into.
fn partial_of(target: &Path) -> PathBuf {
    let mut beside = target.as_os_str().to_owned();
    beside.push(PARTIAL);
    PathBuf::from(beside)
}

/// Copies `staged` into the new file `copy` made by `create`, and waits until it is on disk.
fn copy_whole(
    staged: &Path,
    copy: &Path,
    create: &impl Fn(&Path) -> std::io::Result<std::fs::File>,
) -> std::io::Result<()> {
    let mut file = create(copy)?;
    std::io::copy(&mut std::fs::File::open(staged)?, &mut file)?;
    file.sync_all()
}

/// How long a copy beside a backup's name, or the empty file that holds the name, may stand
/// unchanged before it counts as left behind by a backup that never finished (the program was
/// stopped while it copied). A copy that runs writes its file all the time; an hour is far more
/// than any file system waits before it shows the time of the last write.
const LEFTOVER_AFTER: std::time::Duration = std::time::Duration::from_hours(1);

/// Deletes from `folder` what a backup that never finished left there: a copy `<name>.partial`
/// and the empty file that held its name, once they have not changed for [`LEFTOVER_AFTER`]. An
/// empty name whose copy is still written stays. Nothing else is touched, and a file that cannot
/// be deleted is left for the next time. Only the server's daily round of backups calls it, so a
/// backup made on the command line at the same time never loses its copy to another.
pub fn remove_leftovers(folder: &Path) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    let now = std::time::SystemTime::now();
    let stale = |path: &Path| {
        std::fs::symlink_metadata(path).is_ok_and(|data| {
            data.is_file()
                && data
                    .modified()
                    .ok()
                    .and_then(|changed| now.duration_since(changed).ok())
                    .is_some_and(|age| age >= LEFTOVER_AFTER)
        })
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let copy_of = name
            .strip_suffix(PARTIAL)
            .and_then(BackupName::parse)
            .is_some();
        let empty_name = BackupName::parse(&name).is_some()
            && entry
                .metadata()
                .is_ok_and(|data| data.is_file() && data.len() == 0);
        let left = if copy_of {
            stale(&path)
        } else {
            let copy = partial_of(&path);
            empty_name && stale(&path) && (!copy.exists() || stale(&copy))
        };
        if left && let Err(error) = std::fs::remove_file(&path) {
            tracing::warn!(cause = %error, "a file left by a backup that never finished stays");
        }
    }
}

impl Database {
    /// Creates a backup in `places.folder` while people go on working (ADR-0046, decisions 1 and
    /// 2): the copy of the database, then the files it points to (a file uploaded after the copy
    /// is not in it, so nothing in the backup points to a missing file), and a copy of the replay
    /// list. No key goes in. The file appears whole or not at all.
    ///
    /// # Errors
    /// Fails when the database or a file cannot be read or the backup cannot be written; nothing
    /// is left behind then.
    pub async fn create_backup(
        &self,
        places: &BackupPlaces,
        purpose: Purpose,
        now: Timestamp,
    ) -> Result<CreatedBackup, StoreError> {
        let work = work_folder(&places.folder, "backup")?;
        let made = self.create_in(&work, places, purpose, now).await;
        let _ = std::fs::remove_dir_all(&work);
        made
    }

    async fn create_in(
        &self,
        work: &Path,
        places: &BackupPlaces,
        purpose: Purpose,
        now: Timestamp,
    ) -> Result<CreatedBackup, StoreError> {
        // The name the backup is made for; it is placed under it or a second or two before.
        let name = BackupName::new(purpose, now, VERSION);
        let kind = kind_of(self.dialect());
        let database =
            MemberName::parse(kind.member()).map_err(|_| StoreError::Corrupt("a member"))?;
        let copy = work.join(kind.member());
        let copied = self.copy_into(&copy).await?;
        let replay = work.join(member::REPLAY_LIST);
        match std::fs::copy(&places.replay_list, &replay) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                std::fs::write(&replay, b"").map_err(io)?;
            }
            other => {
                other.map_err(io)?;
            }
        }
        let mut sources = vec![
            archive::Source {
                name: database,
                path: copy,
            },
            archive::Source {
                name: MemberName::parse(member::REPLAY_LIST)
                    .map_err(|_| StoreError::Corrupt("a member"))?,
                path: replay,
            },
        ];
        for hash in &copied.files {
            for path in (places.file_paths)(hash) {
                let source = path
                    .split('/')
                    .fold(places.files.clone(), |folder, part| folder.join(part));
                let member = MemberName::of_file(&path)
                    .map_err(|_| StoreError::Corrupt("the path of a stored file"))?;
                if source.is_file() {
                    sources.push(archive::Source {
                        name: member,
                        path: source,
                    });
                } else {
                    // A file the database points to and the disk does not have (taken away while
                    // the backup ran, or lost before): the backup goes on without it, and says so.
                    tracing::warn!(
                        file = %hex::encode(hash.as_bytes()),
                        "a stored file is missing and is not in the backup"
                    );
                }
            }
        }
        let mut manifest = Manifest {
            format: FORMAT,
            version: name.version.clone(),
            database: kind,
            moment: name.moment,
            migrations: copied
                .migrations
                .iter()
                .map(|record| AppliedMigration {
                    version: record.version,
                    name: record.name.as_str().to_owned(),
                    checksum: hex::encode(record.checksum.as_bytes()),
                })
                .collect(),
            security_log_head: copied.log_head.map(|(seq, hash)| LogHead::new(seq, &hash)),
            keys: seal::key_checks(&places.keys)?,
            members: Vec::new(),
        };
        let partial = work.join(format!("archive{EXTENSION}"));
        let (size, checksum) = archive::write(&partial, &mut manifest, &sources, &places.keys)?;
        let (name, target) = place_under_free_name(&partial, &places.folder, purpose, name.moment)?;
        Ok(CreatedBackup {
            file: CreatedFile {
                name,
                size,
                checksum,
            },
            path: target,
            log_head: copied.log_head,
        })
    }
}

#[cfg(test)]
#[path = "../tests/backup.rs"]
pub(crate) mod tests;
