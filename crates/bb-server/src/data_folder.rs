//! The data folder and its layout (architecture §6): everything Bugaboo stores lives here.
//!
//! ```text
//! bugaboo.db   the built-in database (made by the store)
//! files/       attachments
//! keys/        the master key and other secrets
//! backups/     backups
//! tls/         the certificate and key made by Bugaboo
//! logs/        files the security log keeps outside the database (its anchor file), and the
//!              log of a server that runs as a service
//! setup-code.txt  the setup code of the first start, for a server without a console
//! ```
//!
//! Folders this program creates are readable by their owner only: on Linux mode `0700`, on
//! Windows an access list that removes inherited rights and grants only the current account
//! (found by its SID, which no language or domain setting can change). `keys/`, `tls/` and `logs/`
//! hold secrets or proof, so on every start they are checked and, if someone loosened them, made private
//! again. The rest of a folder that exists is only looked at: what other accounts can open is
//! reported and never changed (`rights.rs`).

use std::fmt;
use std::path::{Path, PathBuf};

mod rights;

use bb_core::message;

/// The flag of `bugaboo service install` that takes over a folder with the person's own
/// certificate in `tls/`; the messages name it.
pub(crate) const TAKE_OVER_FLAG: &str = "--take-over-data-folder";

const DATABASE_FILE: &str = bb_store::DATABASE_FILE;
const FILES: &str = "files";
const KEYS: &str = bb_store::KEYS_FOLDER;
const BACKUPS: &str = "backups";
const TLS: &str = "tls";
const LOGS: &str = bb_store::LOGS_FOLDER;
const ANCHOR_FILE: &str = bb_store::SECURITY_LOG_ANCHOR_FILE;
#[cfg(windows)]
const SERVER_LOG_FILE: &str = "server.log";
const SETUP_CODE_FILE: &str = "setup-code.txt";
/// The folders `prepare` makes inside the data folder.
pub(crate) const SUB_FOLDERS: [&str; 5] = [FILES, KEYS, BACKUPS, TLS, LOGS];
/// What the built-in database (SQLite) keeps next to its file.
const DATABASE_COMPANIONS: [&str; 3] = ["-wal", "-shm", "-journal"];

/// The prepared data folder.
#[derive(Debug)]
pub(crate) struct DataFolder {
    root: PathBuf,
}

/// Why the data folder could not be prepared.
#[derive(Debug)]
pub(crate) enum DataFolderError {
    Create {
        path: PathBuf,
        reason: String,
    },
    Protect {
        path: PathBuf,
        reason: String,
    },
    /// A folder that has other things in it and does not look like a Bugaboo data folder.
    NotADataFolder {
        path: PathBuf,
    },
    /// A folder whose only content is files of the person's own in `tls/` (a certificate made
    /// before the first start): taken over only when the person says so.
    OwnCertificate {
        path: PathBuf,
    },
}

impl DataFolderError {
    pub(crate) fn message(&self) -> String {
        match self {
            Self::Create { path, reason } => {
                message::data_folder_create_failed(path.display(), reason).render()
            }
            Self::Protect { path, reason } => {
                message::data_folder_protect_failed(path.display(), reason).render()
            }
            Self::NotADataFolder { path } => {
                message::data_folder_not_a_data_folder(path.display()).render()
            }
            Self::OwnCertificate { path } => {
                message::data_folder_own_certificate(path.display(), TAKE_OVER_FLAG).render()
            }
        }
    }
}

impl fmt::Display for DataFolderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message())
    }
}

impl std::error::Error for DataFolderError {}

impl DataFolder {
    /// Creates the folder and its sub-folders where they are missing, and makes sure the
    /// folders with secrets are private to their owner.
    pub(crate) fn prepare(root: &Path) -> Result<Self, DataFolderError> {
        create_private(root)?;
        for name in SUB_FOLDERS {
            create_private(&root.join(name))?;
        }
        for name in [KEYS, TLS, LOGS] {
            let path = root.join(name);
            ensure_private(&path).map_err(|reason| DataFolderError::Protect { path, reason })?;
        }
        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    /// The folder of an installation that exists, or `None` when `root` is no folder. Commands
    /// that only look at stored data use this, so a mistyped path does not make an empty data
    /// folder.
    pub(crate) fn existing(root: &Path) -> Option<Self> {
        root.is_dir().then(|| Self {
            root: root.to_path_buf(),
        })
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn tls(&self) -> PathBuf {
        self.root.join(TLS)
    }

    pub(crate) fn keys(&self) -> PathBuf {
        self.root.join(KEYS)
    }

    /// Where the security log writes down its end (SEC-15).
    pub(crate) fn security_log_anchor(&self) -> PathBuf {
        self.root.join(LOGS).join(ANCHOR_FILE)
    }

    /// The log of a server that has no console to write to (a service).
    #[cfg(windows)]
    pub(crate) fn server_log(&self) -> PathBuf {
        self.root.join(LOGS).join(SERVER_LOG_FILE)
    }

    /// Where the setup code of the first start is saved when nobody can read the console.
    pub(crate) fn setup_code_file(&self) -> PathBuf {
        self.root.join(SETUP_CODE_FILE)
    }

    /// The built-in database.
    pub(crate) fn database_file(&self) -> PathBuf {
        self.root.join(DATABASE_FILE)
    }

    /// The files directly in the folder that Bugaboo itself makes and that exist now: the
    /// built-in database with its companions, and the setup code. A service takes over these and
    /// the sub-folders, and nothing else that happens to lie in the folder.
    #[cfg(any(unix, test))]
    pub(crate) fn known_files(&self) -> Vec<PathBuf> {
        let database = self.database_file();
        let mut files = vec![database.clone()];
        for suffix in DATABASE_COMPANIONS {
            let mut name = database.clone().into_os_string();
            name.push(suffix);
            files.push(PathBuf::from(name));
        }
        files.push(self.setup_code_file());
        // A link is not Bugaboo's file, whatever it points to: `symlink_metadata` does not follow.
        files.retain(|file| std::fs::symlink_metadata(file).is_ok_and(|data| data.is_file()));
        files
    }
}

/// Whether a service may take over `root`: a folder that is not there yet or is empty, or one
/// that only Bugaboo makes: it holds the master key or the built-in database, or it is a data
/// folder that has not run yet (all of its sub-folders, all empty). A folder with other things
/// in it, such as documents or a `keys` folder of something else, is refused, so that installing
/// never changes the owner of somebody's files.
///
/// One more folder is taken over, but only when the person asks for it (`take_over`, the flag
/// `--take-over-data-folder`): a folder whose only content is files of their own in `tls/`, such
/// as a certificate put there before Bugaboo ever ran (follow-up M1-F66). The refusal names the
/// flag when it would help.
pub(crate) fn check_for_service(root: &Path, take_over: bool) -> Result<(), DataFolderError> {
    let empty = std::fs::read_dir(root).is_ok_and(|mut entries| entries.next().is_none());
    // A link where a sub-folder belongs would send the changes of rights somewhere else.
    let linked = SUB_FOLDERS.iter().any(|name| {
        std::fs::symlink_metadata(root.join(name)).is_ok_and(|data| data.file_type().is_symlink())
    });
    if linked {
        return Err(DataFolderError::NotADataFolder {
            path: root.to_path_buf(),
        });
    }
    if !root.is_dir() || empty || made_by_bugaboo(root) {
        return Ok(());
    }
    if holds_only_own_certificate(root) {
        return if take_over {
            Ok(())
        } else {
            Err(DataFolderError::OwnCertificate {
                path: root.to_path_buf(),
            })
        };
    }
    Err(DataFolderError::NotADataFolder {
        path: root.to_path_buf(),
    })
}

/// Whether the only content of `root` is files in its `tls` folder: the other sub-folders may be
/// there but are empty, nothing else is in the folder, and `tls` holds plain files (no folders,
/// no links, which could lead anywhere, and on Linux no file with more than one name, which may
/// be somebody else's file under another name). An entry that cannot be read refuses the folder:
/// what cannot be seen cannot be called the person's own.
fn holds_only_own_certificate(root: &Path) -> bool {
    let Some(entries) = std::fs::read_dir(root).ok().and_then(all_ok) else {
        return false;
    };
    let mut certificate_files = 0_usize;
    for entry in entries {
        let name = entry.file_name();
        let path = entry.path();
        let is_folder = std::fs::symlink_metadata(&path).is_ok_and(|data| data.is_dir());
        if !is_folder || !SUB_FOLDERS.iter().any(|known| name == *known) {
            return false;
        }
        let Some(inside) = std::fs::read_dir(&path).ok().and_then(all_ok) else {
            return false;
        };
        for item in inside {
            if name != TLS || !is_plain_file(&item.path()) {
                return false;
            }
            certificate_files += 1;
        }
    }
    certificate_files > 0
}

/// All the items, or `None` when one of them could not be read.
fn all_ok<T, E>(items: impl Iterator<Item = Result<T, E>>) -> Option<Vec<T>> {
    items.collect::<Result<Vec<_>, _>>().ok()
}

/// A regular file that is no link and, on Linux, has one name only.
fn is_plain_file(path: &Path) -> bool {
    let Ok(data) = std::fs::symlink_metadata(path) else {
        return false;
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        data.is_file() && data.nlink() == 1
    }
    #[cfg(not(unix))]
    {
        data.is_file()
    }
}

/// Whether `root` holds something that only Bugaboo makes (see `check_for_service`).
fn made_by_bugaboo(root: &Path) -> bool {
    let empty_folder = |name: &str| {
        std::fs::read_dir(root.join(name)).is_ok_and(|mut entries| entries.next().is_none())
    };
    root.join(KEYS).join(bb_store::MASTER_KEY_FILE).is_file()
        || root.join(DATABASE_FILE).is_file()
        || SUB_FOLDERS.iter().all(|name| empty_folder(name))
}

/// Creates `path` (and missing parents) for its owner only. A folder that already exists is
/// left as its owner set it up.
pub(crate) fn create_private(path: &Path) -> Result<(), DataFolderError> {
    if path.is_dir() {
        return Ok(());
    }
    create_folder(path).map_err(|error| DataFolderError::Create {
        path: path.to_path_buf(),
        reason: error.to_string(),
    })?;
    // On Linux the folder was created with mode 0700 already.
    #[cfg(windows)]
    restrict(path).map_err(|reason| DataFolderError::Protect {
        path: path.to_path_buf(),
        reason,
    })?;
    Ok(())
}

#[cfg(unix)]
fn create_folder(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt as _;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
}

#[cfg(not(unix))]
fn create_folder(path: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(path)
}

/// Linux: the folder gets mode 0700 and the files directly in it mode 0600, if they had more.
#[cfg(unix)]
fn ensure_private(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt as _;
    let tighten = |path: &Path, mode: u32| -> Result<(), String> {
        let current = std::fs::metadata(path)
            .map_err(|error| error.to_string())?
            .permissions()
            .mode();
        if current & 0o077 != 0 {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    };
    tighten(path, 0o700)?;
    for entry in std::fs::read_dir(path).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        if entry.file_type().is_ok_and(|kind| kind.is_file()) {
            tighten(&entry.path(), 0o600)?;
        }
    }
    Ok(())
}

/// Windows: the access list is set again; doing so twice changes nothing.
#[cfg(windows)]
fn ensure_private(path: &Path) -> Result<(), String> {
    restrict(path)
}

/// Windows: drop every entry that was added by hand (`/reset`), remove inherited rights and
/// grant the current account full control, also for everything created inside later (`OI`
/// objects, `CI` containers).
#[cfg(windows)]
fn restrict(path: &Path) -> Result<(), String> {
    let sid = current_sid()?;
    restrict_to(path, &[format!("*{sid}:(OI)(CI)F")])
}

/// The well-known SID of the group of administrators of the computer (fixed, while the name of
/// the group changes with the language).
#[cfg(any(windows, test))]
pub(crate) const ADMINISTRATORS_SID: &str = "S-1-5-32-544";

/// Windows: the same for one file, which the account that runs Bugaboo may change and the
/// administrators of the computer may read (they can take over the file anyway).
#[cfg(windows)]
fn restrict_file(path: &Path) -> Result<(), String> {
    let sid = current_sid()?;
    restrict_to(
        path,
        &[format!("*{sid}:F"), format!("*{ADMINISTRATORS_SID}:R")],
    )
}

/// Windows: removes every entry of the access list and inherited rights, then grants `grants`.
#[cfg(windows)]
fn restrict_to(path: &Path, grants: &[String]) -> Result<(), String> {
    let mut grant = vec!["/inheritance:r".to_owned()];
    for entry in grants {
        grant.push("/grant:r".to_owned());
        grant.push(entry.clone());
    }
    for arguments in [vec!["/reset".to_owned()], grant] {
        let output = std::process::Command::new(system_tool("icacls.exe"))
            .arg(path)
            .args(arguments)
            .output()
            .map_err(|error| error.to_string())?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stdout).trim().to_owned());
        }
    }
    Ok(())
}

#[cfg(windows)]
fn system_tool(name: &str) -> PathBuf {
    let system = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_owned());
    Path::new(&system).join("System32").join(name)
}

/// On Linux: the name of the account that owns `path`, when it is not the account that runs this
/// program. A command that changes a data folder from another account would leave files that the
/// account of the service cannot read, so the commands refuse (root included). `None` when the
/// owner is the current account, and on systems where this cannot be found out.
#[cfg(target_os = "linux")]
pub(crate) fn other_owner(path: &Path) -> Option<String> {
    use std::os::unix::fs::MetadataExt as _;
    let owner = std::fs::metadata(path).ok()?.uid();
    // The owner of /proc/self is the account the program runs as.
    let current = std::fs::metadata("/proc/self").ok()?.uid();
    (owner != current).then(|| {
        std::fs::read_to_string("/etc/passwd")
            .ok()
            .and_then(|passwd| name_in_passwd(&passwd, owner))
            .unwrap_or_else(|| owner.to_string())
    })
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn other_owner(_path: &Path) -> Option<String> {
    None
}

/// The account name for a user number, from the text of `/etc/passwd` (`name:x:uid:...`).
#[cfg(target_os = "linux")]
fn name_in_passwd(passwd: &str, uid: u32) -> Option<String> {
    passwd.lines().find_map(|line| {
        let mut fields = line.split(':');
        let name = fields.next()?;
        (fields.nth(1)?.parse::<u32>().ok()? == uid).then(|| name.to_owned())
    })
}

/// What `write_private` does with a file that is there already.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Existing {
    /// Replaces it, whatever rights it had.
    Replace,
    /// Fails with `AlreadyExists`, and changes nothing.
    Refuse,
}

/// Saves `contents` in a file that only the account that runs Bugaboo can read (and, on Windows,
/// the administrators of the computer): a secret that has to be readable by a person, such as the
/// setup code of a server without a console, or a copy of the key files. The folder of the file
/// must be there already.
///
/// The file is never visible half-made and never visible to others: it is made in a folder of its
/// own that only the owner can enter (made private before anything goes into it), made private
/// itself, filled, and only then moved into place. A write that fails leaves nothing behind.
pub(crate) fn write_private(
    path: &Path,
    contents: &[u8],
    existing: Existing,
) -> std::io::Result<()> {
    use std::io::Write as _;
    write_private_with(path, existing, |file| file.write_all(contents))
}

/// `write_private` with the writing left to `write`, so a failure can be tried.
fn write_private_with(
    path: &Path,
    existing: Existing,
    write: impl FnOnce(&mut std::fs::File) -> std::io::Result<()>,
) -> std::io::Result<()> {
    let not_there = || std::io::Error::from(std::io::ErrorKind::NotFound);
    if !path.parent().is_some_and(Path::is_dir) {
        return Err(not_there());
    }
    let name = path.file_name().ok_or_else(not_there)?;
    let staging = create_staging_folder(path)?;
    let staged = staging.join(name);
    let placed = open_private(&staged)
        .and_then(|mut file| {
            write(&mut file)?;
            std::io::Write::flush(&mut file)
        })
        .and_then(|()| {
            place(&staged, path, existing, |from, to| {
                std::fs::hard_link(from, to)
            })
        });
    // Whatever happened, nothing stays in the folder of the file but the file itself.
    let _ = std::fs::remove_file(&staged);
    let _ = std::fs::remove_dir(&staging);
    placed
}

/// Moves the staged file to `target`. Replacing is a rename. Never replacing is a hard link,
/// which fails when the target is there. A drive without hard links (FAT32, exFAT, some network
/// drives) makes `link` fail for another reason: then the file is made new in the target with
/// `create_new`, which fails when the target is there as well. Such drives have no access lists
/// to keep, so nothing of the privacy of the file is lost (on Windows it is made private where the
/// drive allows).
fn place(
    staged: &Path,
    target: &Path,
    existing: Existing,
    link: impl FnOnce(&Path, &Path) -> std::io::Result<()>,
) -> std::io::Result<()> {
    match existing {
        Existing::Replace => std::fs::rename(staged, target),
        Existing::Refuse => match link(staged, target) {
            Err(error) if error.kind() != std::io::ErrorKind::AlreadyExists => {
                copy_new(staged, target)
            }
            other => other,
        },
    }
}

/// Copies `from` to a file that is new at `to`, and takes it back if the copy fails.
fn copy_new(from: &Path, to: &Path) -> std::io::Result<()> {
    let mut source = std::fs::File::open(from)?;
    let mut target = open_new_where_allowed(to)?;
    let copied =
        std::io::copy(&mut source, &mut target).and_then(|_| std::io::Write::flush(&mut target));
    drop(target);
    if copied.is_err() {
        let _ = std::fs::remove_file(to);
    }
    copied
}

/// How many names are tried for the staging folder before giving up.
const STAGING_TRIES: usize = 5;

/// Makes the folder in which a secret file is made before it is moved to `path`.
fn create_staging_folder(path: &Path) -> std::io::Result<PathBuf> {
    create_staging_folder_with(|| staging_name(path))
}

/// Creates a new folder under the first name that is free. A name that is taken (by somebody who
/// guessed it, or by a call that is running) is never used: the folder is made with a plain
/// `create_dir`, which fails when the name exists, whoever made it and whatever it is (a link
/// included), and the next name is tried. The new folder is for its owner only from the moment it
/// exists (Linux), or at once after (Windows, where nothing can be in it yet).
fn create_staging_folder_with(
    mut name: impl FnMut() -> std::io::Result<PathBuf>,
) -> std::io::Result<PathBuf> {
    for _ in 0..STAGING_TRIES {
        let folder = name()?;
        match create_owner_only_folder(&folder) {
            Ok(()) => return Ok(folder),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists))
}

/// A name for the staging folder beside `path` that cannot be guessed: it ends in random bytes.
fn staging_name(path: &Path) -> std::io::Result<PathBuf> {
    let mut random = [0_u8; 8];
    getrandom::fill(&mut random).map_err(std::io::Error::other)?;
    let tail = bb_core::hex::encode(&random);
    let name = path
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
    Ok(path.with_file_name(format!(".{name}.staging-{tail}")))
}

/// Makes one new folder, not its parents, that only its owner can enter. Fails with
/// `AlreadyExists` when the name is taken.
#[cfg(unix)]
fn create_owner_only_folder(folder: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt as _;
    std::fs::DirBuilder::new().mode(0o700).create(folder)
}

#[cfg(windows)]
fn create_owner_only_folder(folder: &Path) -> std::io::Result<()> {
    std::fs::create_dir(folder)?;
    // Nothing is in it yet. Its access list is set before anything goes in, and what is in it
    // after that was put there by somebody who could enter before: such a folder is not used.
    let made = restrict(folder)
        .map_err(std::io::Error::other)
        .and_then(|()| {
            if std::fs::read_dir(folder)?.next().is_some() {
                Err(std::io::Error::other("the new folder was entered at once"))
            } else {
                Ok(())
            }
        });
    if made.is_err() {
        let _ = std::fs::remove_dir(folder);
    }
    made
}

/// A new file in the target of an export on a drive without hard links: for the owner only where
/// the drive can say so (on Windows a drive without access lists, such as FAT32, cannot).
#[cfg(unix)]
fn open_new_where_allowed(path: &Path) -> std::io::Result<std::fs::File> {
    open_private(path)
}

#[cfg(windows)]
fn open_new_where_allowed(path: &Path) -> std::io::Result<std::fs::File> {
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    let _ = restrict_file(path);
    Ok(file)
}

/// A new file for the owner only, in a folder that is for the owner only.
#[cfg(unix)]
fn open_private(path: &Path) -> std::io::Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt as _;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}

#[cfg(windows)]
fn open_private(path: &Path) -> std::io::Result<std::fs::File> {
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    // The folder it is in grants the owner only. The access list that stays with the file when it
    // is moved is the one set here: the owner, and the administrators for reading.
    restrict_file(path).map_err(std::io::Error::other)?;
    Ok(file)
}

/// Whether anybody besides the owner can open the folder (Linux: a group or world bit is set).
/// On Windows every file written with `write_private` carries an access list of its own, so
/// the folder is not asked.
#[cfg(unix)]
pub(crate) fn open_to_others(folder: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::metadata(folder).is_ok_and(|data| data.permissions().mode() & 0o077 != 0)
}

#[cfg(not(unix))]
pub(crate) fn open_to_others(_folder: &Path) -> bool {
    false
}

/// The SID of the account that runs Bugaboo. Names can be translated, renamed or ambiguous;
/// the SID is not.
#[cfg(windows)]
fn current_sid() -> Result<String, String> {
    let output = std::process::Command::new(system_tool("whoami.exe"))
        .args(["/user", "/fo", "csv", "/nh"])
        .output()
        .map_err(|error| error.to_string())?;
    parse_sid(&String::from_utf8_lossy(&output.stdout))
        .ok_or_else(|| message::Message::from(message::DATA_FOLDER_ACCOUNT_UNKNOWN).render())
}

/// The SID in the output `"DOMAIN\name","S-1-5-21-…"` of `whoami /user /fo csv /nh`.
#[cfg(windows)]
fn parse_sid(output: &str) -> Option<String> {
    let last = output.trim().rsplit(',').next()?;
    let sid = last.trim().trim_matches('"');
    (sid.starts_with("S-1-") && sid.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
        .then(|| sid.to_owned())
}

#[cfg(test)]
mod tests;
