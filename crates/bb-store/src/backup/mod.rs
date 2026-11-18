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
        if let (true, Some(parsed)) = (data.is_file(), parsed) {
            found.push((parsed, entry.path(), data.len()));
        }
    }
    found.sort_by_key(|(name, _, _)| std::cmp::Reverse(name.moment));
    Ok(found)
}

/// How many seconds back a name is looked for when backups follow each other within a second.
const NAME_TRIES: i64 = 60;

/// The first name that is free in `folder`, at `now` or a few seconds before: an earlier moment
/// keeps the promise of the name (every change saved before it is in the backup), a later one
/// would not.
fn free_name(
    folder: &Path,
    purpose: Purpose,
    now: Timestamp,
) -> Result<(BackupName, PathBuf), StoreError> {
    for back in 0..NAME_TRIES {
        let moment = now
            .checked_sub(jiff::SignedDuration::from_secs(back))
            .map_err(|_| StoreError::Corrupt("the moment of a backup"))?;
        let name = BackupName::new(purpose, moment, VERSION);
        let target = folder.join(name.to_string());
        if !target.exists() {
            return Ok((name, target));
        }
    }
    Err(io(std::io::ErrorKind::AlreadyExists.into()))
}

/// Puts the finished archive `staged` at `target`, which must not be there: a link, which fails
/// when the name is taken, so two backups can never take one name, even made at the same moment
/// by the server and the command line. A drive without links gets a copy into a file that is made
/// new under the name, which fails the same way.
fn place_new(staged: &Path, target: &Path) -> std::io::Result<()> {
    match std::fs::hard_link(staged, target) {
        Err(error) if error.kind() != std::io::ErrorKind::AlreadyExists => {
            let mut copy = archive::create_private(target)?;
            let copied = std::io::copy(&mut std::fs::File::open(staged)?, &mut copy)
                .and_then(|_| copy.sync_all());
            if copied.is_err() {
                drop(copy);
                let _ = std::fs::remove_file(target);
            }
            copied
        }
        other => other,
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
        let (name, target) = free_name(&places.folder, purpose, now)?;
        let work = work_folder(&places.folder, "backup")?;
        let made = self.create_in(&work, places, &name, &target).await;
        let _ = std::fs::remove_dir_all(&work);
        made
    }

    async fn create_in(
        &self,
        work: &Path,
        places: &BackupPlaces,
        name: &BackupName,
        target: &Path,
    ) -> Result<CreatedBackup, StoreError> {
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
        place_new(&partial, target).map_err(io)?;
        Ok(CreatedBackup {
            file: CreatedFile {
                name: name.clone(),
                size,
                checksum,
            },
            path: target.to_path_buf(),
            log_head: copied.log_head,
        })
    }
}

#[cfg(test)]
#[path = "../tests/backup.rs"]
mod tests;
