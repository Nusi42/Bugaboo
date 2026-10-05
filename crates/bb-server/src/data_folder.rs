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
//! again.

use std::fmt;
use std::path::{Path, PathBuf};

use bb_core::message;

const DATABASE_FILE: &str = "bugaboo.db";
const FILES: &str = "files";
const KEYS: &str = "keys";
const BACKUPS: &str = "backups";
const TLS: &str = "tls";
const LOGS: &str = "logs";
const ANCHOR_FILE: &str = "security-head.log";
#[cfg(windows)]
const SERVER_LOG_FILE: &str = "server.log";
const SETUP_CODE_FILE: &str = "setup-code.txt";
/// The folders `prepare` makes inside the data folder.
pub(crate) const SUB_FOLDERS: [&str; 5] = [FILES, KEYS, BACKUPS, TLS, LOGS];
/// What the built-in database (SQLite) keeps next to its file.
#[cfg(any(unix, test))]
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
pub(crate) fn check_for_service(root: &Path) -> Result<(), DataFolderError> {
    let empty = std::fs::read_dir(root).is_ok_and(|mut entries| entries.next().is_none());
    // A link where a sub-folder belongs would send the changes of rights somewhere else.
    let linked = SUB_FOLDERS.iter().any(|name| {
        std::fs::symlink_metadata(root.join(name)).is_ok_and(|data| data.file_type().is_symlink())
    });
    if !linked && (!root.is_dir() || empty || made_by_bugaboo(root)) {
        Ok(())
    } else {
        Err(DataFolderError::NotADataFolder {
            path: root.to_path_buf(),
        })
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
    make_folder(path).map_err(|error| DataFolderError::Create {
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
fn make_folder(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt as _;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
}

#[cfg(not(unix))]
fn make_folder(path: &Path) -> std::io::Result<()> {
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

/// The well-known SID of the group of administrators of the computer.
#[cfg(windows)]
const ADMINISTRATORS_SID: &str = "S-1-5-32-544";

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
    let staging = make_staging_folder(path)?;
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
fn make_staging_folder(path: &Path) -> std::io::Result<PathBuf> {
    make_staging_folder_with(|| staging_name(path))
}

/// Makes a new folder under the first name that is free. A name that is taken (by somebody who
/// guessed it, or by a call that is running) is never used: the folder is made with a plain
/// `create_dir`, which fails when the name exists, whoever made it and whatever it is (a link
/// included), and the next name is tried. The new folder is for its owner only from the moment it
/// exists (Linux), or at once after (Windows, where nothing can be in it yet).
fn make_staging_folder_with(
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
    use std::fmt::Write as _;
    let mut random = [0_u8; 8];
    getrandom::fill(&mut random).map_err(std::io::Error::other)?;
    let tail = random.iter().fold(String::new(), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");
        hex
    });
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
mod tests {
    use super::*;

    #[test]
    fn dist_01_the_layout_is_created() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("data");
        let folder = DataFolder::prepare(&root).unwrap();
        for name in [FILES, KEYS, BACKUPS, TLS, LOGS] {
            assert!(root.join(name).is_dir(), "{name}");
        }
        assert_eq!(folder.tls(), root.join("tls"));
        assert_eq!(folder.root(), root);
        // Preparing again changes nothing and does not fail.
        DataFolder::prepare(&root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn sec_16_folders_are_for_the_owner_only_and_repaired_on_linux() {
        use std::os::unix::fs::PermissionsExt as _;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("data");
        DataFolder::prepare(&root).unwrap();
        let mode = |name: &str| {
            std::fs::metadata(root.join(name))
                .unwrap()
                .permissions()
                .mode()
                & 0o777
        };
        for name in ["", KEYS, TLS, LOGS] {
            assert_eq!(mode(name), 0o700, "{name}");
        }
        // Someone loosens the secrets; the next start makes them private again.
        let secret = root.join(KEYS).join("master.key");
        std::fs::write(&secret, "x").unwrap();
        std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o644)).unwrap();
        std::fs::set_permissions(root.join(KEYS), std::fs::Permissions::from_mode(0o755)).unwrap();
        let anchor = root.join(LOGS).join("security-head.log");
        std::fs::write(&anchor, "x").unwrap();
        std::fs::set_permissions(&anchor, std::fs::Permissions::from_mode(0o644)).unwrap();
        std::fs::set_permissions(root.join(LOGS), std::fs::Permissions::from_mode(0o755)).unwrap();
        DataFolder::prepare(&root).unwrap();
        assert_eq!(mode(KEYS), 0o700);
        assert_eq!(mode(LOGS), 0o700);
        assert_eq!(mode("logs/security-head.log"), 0o600);
        assert_eq!(mode("keys/master.key"), 0o600);
    }

    #[cfg(windows)]
    fn entries(path: &Path) -> usize {
        let output = std::process::Command::new("icacls")
            .arg(path)
            .output()
            .unwrap();
        // Names of groups are translated, so count the entries instead: only one account (the
        // owner) may have rights.
        String::from_utf8_lossy(&output.stdout)
            .matches(":(")
            .count()
    }

    #[cfg(windows)]
    #[test]
    fn sec_16_folders_are_for_the_owner_only_and_repaired_on_windows() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("data");
        DataFolder::prepare(&root).unwrap();
        for name in [KEYS, TLS, LOGS] {
            assert_eq!(entries(&root.join(name)), 1, "{name}");
        }
        // Someone opens the keys and logs folders again; the next start closes them.
        let logs = root.join(LOGS);
        let keys = root.join(KEYS);
        let sid = current_sid().unwrap();
        let reset = std::process::Command::new("icacls")
            .arg(&keys)
            .args(["/inheritance:e", "/grant", "*S-1-5-32-545:(OI)(CI)R"])
            .output()
            .unwrap();
        assert!(reset.status.success());
        let reset_logs = std::process::Command::new("icacls")
            .arg(&logs)
            .args(["/inheritance:e", "/grant", "*S-1-5-32-545:(OI)(CI)R"])
            .output()
            .unwrap();
        assert!(reset_logs.status.success());
        assert!(entries(&keys) > 1, "the test could not open the folder");
        assert!(
            entries(&logs) > 1,
            "the test could not open the logs folder"
        );
        DataFolder::prepare(&root).unwrap();
        assert_eq!(entries(&keys), 1, "owner {sid} only");
        assert_eq!(entries(&logs), 1, "owner {sid} only");
    }

    #[cfg(windows)]
    #[test]
    fn sec_16_the_account_is_found_by_sid() {
        assert_eq!(
            parse_sid("\"EXAMPLE-PC\\ana\",\"S-1-5-21-1-2-3-1001\"\r\n").as_deref(),
            Some("S-1-5-21-1-2-3-1001")
        );
        assert_eq!(parse_sid("nonsense"), None);
        assert_eq!(parse_sid("\"a\",\"S-1-5-21; calc\""), None);
        assert!(current_sid().unwrap().starts_with("S-1-"));
    }

    #[test]
    fn sec_16_a_secret_file_holds_its_text_and_replaces_an_older_file() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("code.txt");
        std::fs::write(&file, "an older and longer text").unwrap();
        write_private(&file, b"new\n", Existing::Replace).unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "new\n");
        assert_eq!(names(temp.path()), ["code.txt"], "a half-made file stayed");
    }

    /// What is in a folder, by name.
    fn names(folder: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn sec_16_a_failed_write_leaves_no_file_and_no_leftover() {
        use std::io::Write as _;
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("code.txt");
        let failing = |file: &mut std::fs::File| -> std::io::Result<()> {
            file.write_all(b"half of a secret")?;
            Err(std::io::Error::other("the disk is full"))
        };
        // New file: nothing is left of the half-written secret, in the folder or beside it.
        let error = write_private_with(&target, Existing::Refuse, failing).unwrap_err();
        assert_eq!(error.to_string(), "the disk is full");
        assert!(names(temp.path()).is_empty(), "{:?}", names(temp.path()));
        // Replacing: the old file stays as it was.
        std::fs::write(&target, "the old file").unwrap();
        write_private_with(&target, Existing::Replace, failing).unwrap_err();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "the old file");
        assert_eq!(names(temp.path()), ["code.txt"]);
        // A file that is there is not the failed write's to remove.
        write_private_with(&target, Existing::Refuse, failing).unwrap_err();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "the old file");
    }

    #[test]
    fn sec_16_a_secret_is_not_written_into_a_folder_that_is_not_there() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("not-there").join("code.txt");
        let error = write_private(&target, b"secret", Existing::Replace).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
        assert!(
            !temp.path().join("not-there").exists(),
            "the folder was made"
        );
    }

    #[test]
    fn sec_16_a_staging_folder_that_somebody_made_first_is_never_used() {
        let temp = tempfile::tempdir().unwrap();
        let planted = temp.path().join("planted");
        let fresh = temp.path().join("fresh");
        std::fs::create_dir(&planted).unwrap();
        std::fs::write(planted.join("theirs.txt"), "x").unwrap();
        // The first name is taken by somebody else: the next one is used, and nothing is ever
        // written into the folder that was there.
        let mut offered = vec![planted.clone(), fresh.clone()].into_iter();
        let made = make_staging_folder_with(|| Ok(offered.next().unwrap())).unwrap();
        assert_eq!(made, fresh);
        assert_eq!(names(&planted), ["theirs.txt"]);
        // Names that are all taken give up after a few tries, with an error.
        let taken = temp.path().join("taken");
        std::fs::create_dir(&taken).unwrap();
        let mut tries = 0;
        let error = make_staging_folder_with(|| {
            tries += 1;
            Ok(taken.clone())
        })
        .unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(tries, STAGING_TRIES);
        assert!(names(&taken).is_empty(), "something was written into it");
    }

    #[test]
    fn sec_16_a_secret_goes_to_the_new_folder_and_not_to_a_planted_one() {
        // The names are random: a planted folder with a name that was guessed does not exist,
        // and two calls never agree on a name.
        let path = Path::new("a/code.txt");
        let first = staging_name(path).unwrap();
        let second = staging_name(path).unwrap();
        assert_ne!(first, second);
        assert!(
            first.to_string_lossy().contains(".code.txt.staging-"),
            "{first:?}"
        );
    }

    #[cfg(windows)]
    #[test]
    fn sec_16_a_secret_is_made_in_a_folder_only_its_owner_can_enter() {
        // The file is first written in a folder of its own that is for the owner only, and moved
        // into place after: nobody who can read the target folder can catch it half-made.
        let temp = tempfile::tempdir().unwrap();
        let staging = make_staging_folder(&temp.path().join("code.txt")).unwrap();
        let output = std::process::Command::new("icacls")
            .arg(&staging)
            .output()
            .unwrap();
        let listing = String::from_utf8_lossy(&output.stdout).into_owned();
        assert_eq!(listing.matches(":(").count(), 1, "{listing}");
        assert!(!listing.contains("(I)"), "{listing}");
    }

    #[test]
    fn sec_16_without_hard_links_a_new_file_is_made_in_the_target() {
        let temp = tempfile::tempdir().unwrap();
        let staged = temp.path().join("staged.txt");
        std::fs::write(&staged, "the secret").unwrap();
        let target = temp.path().join("target.txt");
        // A drive without hard links (FAT32, exFAT, some network drives) says "unsupported".
        let unsupported =
            |_: &Path, _: &Path| Err(std::io::Error::from(std::io::ErrorKind::Unsupported));
        place(&staged, &target, Existing::Refuse, unsupported).unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "the secret");
        // It still never replaces a file that is there.
        std::fs::write(&staged, "another secret").unwrap();
        let error = place(&staged, &target, Existing::Refuse, unsupported).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "the secret");
        // A link that fails because the target is there is not a drive without links.
        let taken =
            |_: &Path, _: &Path| Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists));
        let other = temp.path().join("other.txt");
        let error = place(&staged, &other, Existing::Refuse, taken).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        assert!(!other.exists(), "the fallback ran for a taken name");
    }

    #[test]
    fn sec_16_a_secret_file_can_refuse_to_replace_a_file() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("code.txt");
        write_private(&file, b"first", Existing::Refuse).unwrap();
        let error = write_private(&file, b"second", Existing::Refuse).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "first");
    }

    #[cfg(unix)]
    #[test]
    fn sec_16_a_secret_file_is_for_its_owner_only_on_linux() {
        use std::os::unix::fs::PermissionsExt as _;
        let temp = tempfile::tempdir().unwrap();
        let mode = |file: &Path| std::fs::metadata(file).unwrap().permissions().mode() & 0o777;
        let fresh = temp.path().join("fresh.txt");
        write_private(&fresh, b"secret", Existing::Replace).unwrap();
        assert_eq!(mode(&fresh), 0o600);
        // A file that was readable by everybody is closed before the secret goes in.
        let open = temp.path().join("open.txt");
        std::fs::write(&open, "old").unwrap();
        std::fs::set_permissions(&open, std::fs::Permissions::from_mode(0o644)).unwrap();
        write_private(&open, b"secret", Existing::Replace).unwrap();
        assert_eq!(mode(&open), 0o600);
    }

    #[cfg(windows)]
    #[test]
    fn sec_16_a_secret_file_is_for_its_owner_and_the_administrators_on_windows() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("code.txt");
        std::fs::write(&file, "old").unwrap();
        // The file starts with the rights of its folder, which the test folder gives to more.
        write_private(&file, b"secret", Existing::Replace).unwrap();
        let output = std::process::Command::new("icacls")
            .arg(&file)
            .output()
            .unwrap();
        let listing = String::from_utf8_lossy(&output.stdout).into_owned();
        // The owner with full control and the administrators with read rights: two entries,
        // none of them inherited.
        assert_eq!(listing.matches(":(").count(), 2, "{listing}");
        assert!(!listing.contains("(I)"), "{listing}");
        assert!(listing.contains("(R)"), "{listing}");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn sec_16_the_owner_of_a_folder_is_found_by_user_number() {
        let passwd =
            "root:x:0:0:root:/root:/bin/bash\nbugaboo:x:998:998::/nonexistent:/usr/sbin/nologin\n";
        assert_eq!(name_in_passwd(passwd, 998).as_deref(), Some("bugaboo"));
        assert_eq!(name_in_passwd(passwd, 0).as_deref(), Some("root"));
        assert_eq!(name_in_passwd(passwd, 5), None);
        // A folder this test made belongs to the account that runs the test.
        let temp = tempfile::tempdir().unwrap();
        assert_eq!(other_owner(temp.path()), None);
    }

    #[test]
    fn sec_16_a_service_may_take_over_only_a_new_or_a_bugaboo_folder() {
        let temp = tempfile::tempdir().unwrap();
        // Not there yet, and empty: fine.
        check_for_service(&temp.path().join("new")).unwrap();
        let empty = temp.path().join("empty");
        std::fs::create_dir(&empty).unwrap();
        check_for_service(&empty).unwrap();
        // A folder with somebody's own files is refused, and nothing in it changes.
        let mine = temp.path().join("documents");
        std::fs::create_dir(&mine).unwrap();
        std::fs::write(mine.join("tax.pdf"), "x").unwrap();
        let error = check_for_service(&mine).unwrap_err();
        assert!(error.message().contains("does not look like"), "{error}");
        // An unrelated folder named `keys` (or with all the names of a data folder but a file
        // in one of them) is no Bugaboo folder either.
        let other = temp.path().join("other");
        std::fs::create_dir_all(other.join(KEYS)).unwrap();
        std::fs::write(other.join(KEYS).join("id_ed25519"), "x").unwrap();
        std::fs::write(other.join("notes.txt"), "x").unwrap();
        assert!(
            check_for_service(&other).is_err(),
            "a folder with a `keys` folder"
        );
        let layout = temp.path().join("layout");
        DataFolder::prepare(&layout).unwrap();
        std::fs::write(layout.join(FILES).join("photo.jpg"), "x").unwrap();
        std::fs::write(layout.join("notes.txt"), "x").unwrap();
        assert!(
            check_for_service(&layout).is_err(),
            "a layout with somebody's file in it"
        );
        // A data folder that Bugaboo made and has not filled yet: all of its folders, all empty.
        let fresh = temp.path().join("fresh");
        DataFolder::prepare(&fresh).unwrap();
        check_for_service(&fresh).unwrap();
        // One that has run holds the master key or the database, also with files added later.
        let used = temp.path().join("used");
        DataFolder::prepare(&used).unwrap();
        std::fs::write(used.join(KEYS).join(bb_store::MASTER_KEY_FILE), "x").unwrap();
        std::fs::write(used.join("notes.txt"), "x").unwrap();
        check_for_service(&used).unwrap();
        let with_database = temp.path().join("with-database");
        DataFolder::prepare(&with_database).unwrap();
        std::fs::write(with_database.join(DATABASE_FILE), "x").unwrap();
        std::fs::write(with_database.join("notes.txt"), "x").unwrap();
        check_for_service(&with_database).unwrap();
        // The known files are the database and its companions and the setup code.
        std::fs::write(with_database.join(format!("{DATABASE_FILE}-wal")), "x").unwrap();
        let known = DataFolder::existing(&with_database).unwrap().known_files();
        let names: Vec<_> = known
            .iter()
            .filter_map(|file| file.file_name()?.to_str())
            .collect();
        assert_eq!(names, ["bugaboo.db", "bugaboo.db-wal"]);
    }

    #[cfg(unix)]
    #[test]
    fn sec_16_a_link_is_not_a_known_file_and_a_linked_sub_folder_is_refused() {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("data");
        DataFolder::prepare(&data).unwrap();
        let elsewhere = temp.path().join("elsewhere.txt");
        std::fs::write(&elsewhere, "x").unwrap();
        // The setup code file is a link to somebody else's file: it is left alone.
        std::os::unix::fs::symlink(&elsewhere, data.join(SETUP_CODE_FILE)).unwrap();
        std::fs::write(data.join(DATABASE_FILE), "x").unwrap();
        let known = DataFolder::existing(&data).unwrap().known_files();
        assert_eq!(known, [data.join(DATABASE_FILE)]);
        // A sub-folder that is a link to another place is refused for the service.
        let other = temp.path().join("other");
        std::fs::create_dir(&other).unwrap();
        std::fs::remove_dir(data.join(TLS)).unwrap();
        std::os::unix::fs::symlink(&other, data.join(TLS)).unwrap();
        let error = check_for_service(&data).unwrap_err();
        assert!(error.message().contains("does not look like"), "{error}");
    }

    #[test]
    fn dist_01_a_blocked_path_is_explained() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("a-file");
        std::fs::write(&file, "x").unwrap();
        let error = DataFolder::prepare(&file.join("data")).unwrap_err();
        assert!(error.message().contains("could not be prepared"), "{error}");
    }
}
