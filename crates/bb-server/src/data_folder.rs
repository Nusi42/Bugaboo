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
//! Windows an access list that removes inherited rights and grants only the current account.

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
    Create {
        path: PathBuf,
        reason: String,
    },
    /// Only Windows needs a separate step to make a folder private.
    #[cfg(windows)]
    Protect {
        path: PathBuf,
        reason: String,
    },
}

impl DataFolderError {
    pub(crate) fn message(&self) -> String {
        match self {
            Self::Create { path, reason } => t(
                "data-folder.create-failed",
                &[("path", &path.display().to_string()), ("reason", reason)],
            ),
            #[cfg(windows)]
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
    /// Creates the folder and its sub-folders where they are missing.
    pub(crate) fn prepare(root: &Path) -> Result<Self, DataFolderError> {
        create_private(root)?;
        for name in [FILES, KEYS, BACKUPS, TLS] {
            create_private(&root.join(name))?;
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

/// Windows: remove inherited rights and grant the current account full control, also for
/// everything created inside later (`OI` objects, `CI` containers).
#[cfg(windows)]
fn restrict(path: &Path) -> Result<(), String> {
    let account = match (std::env::var("USERDOMAIN"), std::env::var("USERNAME")) {
        (Ok(domain), Ok(user)) if !domain.is_empty() => format!("{domain}\\{user}"),
        (_, Ok(user)) => user,
        _ => return Err("the name of the current account is unknown".to_owned()),
    };
    let system = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_owned());
    let output = std::process::Command::new(Path::new(&system).join("System32").join("icacls.exe"))
        .arg(path)
        .args(["/inheritance:r", "/grant:r"])
        .arg(format!("{account}:(OI)(CI)F"))
        .output()
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }
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
    fn sec_16_folders_are_for_the_owner_only_on_linux() {
        use std::os::unix::fs::PermissionsExt as _;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("data");
        DataFolder::prepare(&root).unwrap();
        for name in ["", KEYS, TLS] {
            let mode = std::fs::metadata(root.join(name))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o700, "{name}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn sec_16_folders_are_for_the_owner_only_on_windows() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("data");
        DataFolder::prepare(&root).unwrap();
        for name in [KEYS, TLS] {
            let output = std::process::Command::new("icacls")
                .arg(root.join(name))
                .output()
                .unwrap();
            // Names of groups are translated, so count the entries instead: only one account
            // (the owner) may have rights.
            let rights = String::from_utf8_lossy(&output.stdout);
            assert_eq!(rights.matches(":(").count(), 1, "{name}: {rights}");
        }
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
