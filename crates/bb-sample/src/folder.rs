//! The folder one run writes to: a data folder that a server can start on, the file with the ways
//! to sign in, and the dump of what was made.
//!
//! ```text
//! <out>/
//! ├── data/          a data folder (database, keys, logs): copy it, start `bugaboo serve` on the copy
//! ├── sign-in.txt    e-mail addresses and random passwords of the people: never leaves this folder
//! ├── dump.json      the canonical dump (TEST-SAMPLE-DATA): no secret in it
//! └── sample.json    says that this folder was made by the tool, so the next run may empty it
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::error::SampleError;

/// The data folder inside the output folder.
pub const DATA_FOLDER: &str = "data";
/// The file with the passwords, inside the output folder. The only file with a secret in it.
pub const SIGN_IN_FILE: &str = "sign-in.txt";
/// The canonical dump, inside the output folder.
pub const DUMP_FILE: &str = "dump.json";
/// The file that marks a folder as made by this tool.
pub const MARKER_FILE: &str = "sample.json";
/// Tells Git to ignore everything in the folder, like Cargo does for `target/`: `sign-in.txt` holds
/// passwords, and the folder can be anywhere (`--out`), not only under `target/`.
const GIT_IGNORE_FILE: &str = ".gitignore";
const GIT_IGNORE_TEXT: &str = "*\n";

/// The output folder of a run, emptied of an older sample.
#[derive(Clone, Debug)]
pub struct Output {
    root: PathBuf,
}

impl Output {
    /// Gets `root` ready: made when it is missing, emptied of an older sample, and left alone
    /// when it holds anything else.
    ///
    /// # Errors
    /// [`SampleError::NotASampleFolder`] for a folder with other things in it, and the errors of
    /// the file system.
    pub fn prepare(root: &Path) -> Result<Self, SampleError> {
        let failed = |error: &std::io::Error| SampleError::folder(root, error);
        if root.is_dir() {
            let empty = fs::read_dir(root)
                .map_err(|error| failed(&error))?
                .next()
                .is_none();
            if !empty {
                if !root.join(MARKER_FILE).is_file() {
                    return Err(SampleError::NotASampleFolder(root.to_path_buf()));
                }
                // Only what this tool makes: the marker says the rest is ours too.
                remove_folder(&root.join(DATA_FOLDER)).map_err(|error| failed(&error))?;
                for file in [SIGN_IN_FILE, DUMP_FILE, MARKER_FILE, GIT_IGNORE_FILE] {
                    remove_file(&root.join(file)).map_err(|error| failed(&error))?;
                }
            }
        }
        fs::create_dir_all(root).map_err(|error| failed(&error))?;
        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    /// Writes the marker, from now on the folder may be emptied by the next run, and the file that
    /// keeps everything in the folder out of Git.
    ///
    /// # Errors
    /// The errors of the file system.
    pub fn mark(&self, seed: u64, start: i64) -> Result<(), SampleError> {
        self.write(GIT_IGNORE_FILE, GIT_IGNORE_TEXT)?;
        let text = json!({ "seed": seed, "start": start }).to_string();
        self.write(MARKER_FILE, &text)
    }

    /// The data folder.
    #[must_use]
    pub fn data_folder(&self) -> PathBuf {
        self.root.join(DATA_FOLDER)
    }

    /// The file with the passwords.
    #[must_use]
    pub fn sign_in_file(&self) -> PathBuf {
        self.root.join(SIGN_IN_FILE)
    }

    /// The dump.
    #[must_use]
    pub fn dump_file(&self) -> PathBuf {
        self.root.join(DUMP_FILE)
    }

    /// The output folder.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Writes a file in the folder; one that holds secrets can be read by its owner only (on
    /// Unix; on Windows the rights of the user's folder apply).
    ///
    /// # Errors
    /// The errors of the file system.
    pub fn write(&self, name: &str, text: &str) -> Result<(), SampleError> {
        use std::io::Write as _;
        let path = self.root.join(name);
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        let mut file = options
            .open(&path)
            .map_err(|error| SampleError::folder(&path, &error))?;
        file.write_all(text.as_bytes())
            .map_err(|error| SampleError::folder(&path, &error))
    }
}

fn remove_folder(path: &Path) -> std::io::Result<()> {
    match fs::remove_dir_all(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

fn remove_file(path: &Path) -> std::io::Result<()> {
    match fs::remove_file(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_folder_is_made_and_an_older_sample_is_emptied() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("nested").join("sample");
        let output = Output::prepare(&root).unwrap();
        output.mark(42, 1_800_000_000).unwrap();
        fs::create_dir_all(output.data_folder().join("keys")).unwrap();
        fs::write(output.data_folder().join("keys").join("old"), "x").unwrap();
        fs::write(output.sign_in_file(), "old").unwrap();

        let again = Output::prepare(&root).unwrap();
        assert!(!again.data_folder().exists(), "the old data is gone");
        assert!(!again.sign_in_file().exists());
        assert!(
            !root.join(MARKER_FILE).exists(),
            "marked again when written"
        );
    }

    #[test]
    fn git_ignores_everything_in_the_folder_wherever_it_is() {
        let temp = tempfile::tempdir().unwrap();
        let git = |args: &[&str]| {
            std::process::Command::new("git")
                .args(args)
                .current_dir(temp.path())
                .output()
                .unwrap()
        };
        assert!(git(&["init", "-q"]).status.success());
        // A folder outside `target/`, as `--out` allows.
        let output = Output::prepare(&temp.path().join("somewhere").join("sample")).unwrap();
        output.mark(42, 1_800_000_000).unwrap();
        output
            .write(SIGN_IN_FILE, "ana@bugaboo.example\tsecret\n")
            .unwrap();
        for file in [SIGN_IN_FILE, DUMP_FILE, MARKER_FILE, "data/bugaboo.db"] {
            let path = format!("somewhere/sample/{file}");
            assert!(
                git(&["check-ignore", "-q", &path]).status.success(),
                "{path} is not ignored by Git"
            );
        }
        // The next run empties it again, the file that ignores included.
        Output::prepare(output.root()).unwrap();
        assert!(!output.root().join(GIT_IGNORE_FILE).exists());
    }

    #[test]
    fn bad_example_a_folder_with_other_things_in_it_is_left_alone() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("notes.txt"), "mine").unwrap();
        let error = Output::prepare(temp.path()).unwrap_err();
        assert!(matches!(error, SampleError::NotASampleFolder(_)), "{error}");
        assert_eq!(
            fs::read_to_string(temp.path().join("notes.txt")).unwrap(),
            "mine"
        );
    }

    #[test]
    fn an_empty_existing_folder_is_fine() {
        let temp = tempfile::tempdir().unwrap();
        assert!(Output::prepare(temp.path()).is_ok());
    }
}
