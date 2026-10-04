//! The data folder and its layout (architecture §6): everything Bugaboo stores lives here.
//!
//! ```text
//! bugaboo.db   the built-in database (made by the store)
//! files/       attachments
//! keys/        the master key and other secrets
//! backups/     backups
//! tls/         the certificate and key made by Bugaboo
//! ```
//!
//! Folders this program creates are readable by their owner only: on Linux mode `0700`, on
//! Windows an access list that removes inherited rights and grants only the current account
//! (found by its SID, which no language or domain setting can change). `keys/` and `tls/` hold
//! secrets, so on every start they are checked and, if someone loosened them, made private
//! again.

use std::fmt;
use std::path::{Path, PathBuf};

use crate::text::t;

const FILES: &str = "files";
const KEYS: &str = "keys";
const BACKUPS: &str = "backups";
const TLS: &str = "tls";

/// The prepared data folder.
#[derive(Debug)]
pub(crate) struct DataFolder {
    root: PathBuf,
}

/// Why the data folder could not be prepared.
#[derive(Debug)]
pub(crate) enum DataFolderError {
    Create { path: PathBuf, reason: String },
    Protect { path: PathBuf, reason: String },
}

impl DataFolderError {
    pub(crate) fn message(&self) -> String {
        match self {
            Self::Create { path, reason } => t(
                "data-folder.create-failed",
                &[("path", &path.display().to_string()), ("reason", reason)],
            ),
            Self::Protect { path, reason } => t(
                "data-folder.protect-failed",
                &[("path", &path.display().to_string()), ("reason", reason)],
            ),
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
        for name in [FILES, KEYS, BACKUPS, TLS] {
            create_private(&root.join(name))?;
        }
        for name in [KEYS, TLS] {
            let path = root.join(name);
            ensure_private(&path).map_err(|reason| DataFolderError::Protect { path, reason })?;
        }
        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn tls(&self) -> PathBuf {
        self.root.join(TLS)
    }
}

/// Creates `path` (and missing parents) for its owner only. A folder that already exists is
/// left as its owner set it up.
fn create_private(path: &Path) -> Result<(), DataFolderError> {
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
    let grant = format!("*{sid}:(OI)(CI)F");
    for arguments in [
        vec!["/reset".to_owned()],
        vec!["/inheritance:r".to_owned(), "/grant:r".to_owned(), grant],
    ] {
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

/// The SID of the account that runs Bugaboo. Names can be translated, renamed or ambiguous;
/// the SID is not.
#[cfg(windows)]
fn current_sid() -> Result<String, String> {
    let output = std::process::Command::new(system_tool("whoami.exe"))
        .args(["/user", "/fo", "csv", "/nh"])
        .output()
        .map_err(|error| error.to_string())?;
    parse_sid(&String::from_utf8_lossy(&output.stdout))
        .ok_or_else(|| "the account of the running program could not be identified".to_owned())
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
        for name in [FILES, KEYS, BACKUPS, TLS] {
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
        for name in ["", KEYS, TLS] {
            assert_eq!(mode(name), 0o700, "{name}");
        }
        // Someone loosens the secrets; the next start makes them private again.
        let secret = root.join(KEYS).join("master.key");
        std::fs::write(&secret, "x").unwrap();
        std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o644)).unwrap();
        std::fs::set_permissions(root.join(KEYS), std::fs::Permissions::from_mode(0o755)).unwrap();
        DataFolder::prepare(&root).unwrap();
        assert_eq!(mode(KEYS), 0o700);
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
        for name in [KEYS, TLS] {
            assert_eq!(entries(&root.join(name)), 1, "{name}");
        }
        // Someone opens the keys folder again; the next start closes it.
        let keys = root.join(KEYS);
        let sid = current_sid().unwrap();
        let reset = std::process::Command::new("icacls")
            .arg(&keys)
            .args(["/inheritance:e", "/grant", "*S-1-5-32-545:(OI)(CI)R"])
            .output()
            .unwrap();
        assert!(reset.status.success());
        assert!(entries(&keys) > 1, "the test could not open the folder");
        DataFolder::prepare(&root).unwrap();
        assert_eq!(entries(&keys), 1, "owner {sid} only");
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
    fn dist_01_a_blocked_path_is_explained() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("a-file");
        std::fs::write(&file, "x").unwrap();
        let error = DataFolder::prepare(&file.join("data")).unwrap_err();
        assert!(error.message().contains("could not be prepared"), "{error}");
    }
}
