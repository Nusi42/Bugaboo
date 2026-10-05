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
/// that Bugaboo made (it has the folder `keys`). A folder with other things in it, such as
/// documents, is refused, so that installing never changes the owner of somebody's files.
pub(crate) fn check_for_service(root: &Path) -> Result<(), DataFolderError> {
    let empty = std::fs::read_dir(root).is_ok_and(|mut entries| entries.next().is_none());
    // A link where a sub-folder belongs would send the changes of rights somewhere else.
    let linked = SUB_FOLDERS.iter().any(|name| {
        std::fs::symlink_metadata(root.join(name)).is_ok_and(|data| data.file_type().is_symlink())
    });
    if !linked && (!root.is_dir() || empty || root.join(KEYS).is_dir()) {
        Ok(())
    } else {
        Err(DataFolderError::NotADataFolder {
            path: root.to_path_buf(),
        })
    }
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
/// setup code of a server without a console, or a copy of the key files. The file is made
/// private before the secret is written into it.
pub(crate) fn write_private(
    path: &Path,
    contents: &[u8],
    existing: Existing,
) -> std::io::Result<()> {
    use std::io::Write as _;
    match existing {
        Existing::Refuse => {
            let mut file = open_private(path, existing)?;
            file.write_all(contents)?;
            file.flush()
        }
        Existing::Replace => {
            // Written beside the file and moved into place, so that somebody who reads the file
            // at that moment (a test, or a person who waits for it) sees the old file or the
            // whole new one, never an empty file that is still being made private.
            let mut name = path.as_os_str().to_owned();
            name.push(".part");
            let part = PathBuf::from(name);
            let mut file = open_private(&part, Existing::Replace)?;
            file.write_all(contents)?;
            file.flush()?;
            drop(file);
            std::fs::rename(&part, path).inspect_err(|_| {
                let _ = std::fs::remove_file(&part);
            })
        }
    }
}

fn open_options(existing: Existing) -> std::fs::OpenOptions {
    let mut options = std::fs::OpenOptions::new();
    options.write(true);
    match existing {
        Existing::Replace => options.create(true).truncate(true),
        Existing::Refuse => options.create_new(true),
    };
    options
}

#[cfg(unix)]
fn open_private(path: &Path, existing: Existing) -> std::io::Result<std::fs::File> {
    use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
    let file = open_options(existing).mode(0o600).open(path)?;
    // `mode` counts only for a new file: a file that was there before may have had more.
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    Ok(file)
}

#[cfg(windows)]
fn open_private(path: &Path, existing: Existing) -> std::io::Result<std::fs::File> {
    // The file exists, empty, before its access list changes and before the secret goes in.
    let file = open_options(existing).open(path)?;
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
        assert!(
            !temp.path().join("code.txt.part").exists(),
            "a half-made file stayed"
        );
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
        // A data folder that Bugaboo made is fine, also with files of its own that were added.
        let data = temp.path().join("data");
        DataFolder::prepare(&data).unwrap();
        std::fs::write(data.join("notes.txt"), "x").unwrap();
        check_for_service(&data).unwrap();
        // The known files are the database and its companions and the setup code.
        std::fs::write(data.join(DATABASE_FILE), "x").unwrap();
        std::fs::write(data.join(format!("{DATABASE_FILE}-wal")), "x").unwrap();
        let known = DataFolder::existing(&data).unwrap().known_files();
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
