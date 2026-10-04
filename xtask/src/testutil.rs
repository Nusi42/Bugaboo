//! Helpers for the `bad_example` / `good_example` tests: small throw-away repositories.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::checks::Finding;

/// A folder under `target/test-tmp` that is removed when dropped.
pub(crate) struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub(crate) fn new(name: &str) -> Self {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
            .join("target")
            .join("test-tmp")
            .join(format!("{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// Writes a file (creating folders) and returns its path.
    pub(crate) fn write(&self, relative: &str, contents: impl AsRef<[u8]>) -> PathBuf {
        let file = self.path.join(relative);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, contents).unwrap();
        file
    }

    /// Runs `git` in this folder and returns whether it succeeded.
    pub(crate) fn git(&self, args: &[&str]) -> bool {
        Command::new("git")
            .args(["-c", "user.name=Test", "-c", "user.email=test@example.test"])
            .args(["-c", "core.autocrlf=false", "-c", "commit.gpgsign=false"])
            .args(args)
            .current_dir(&self.path)
            .output()
            .unwrap()
            .status
            .success()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// A `bad_example` must fail because of the problem in the example, not because a tool is missing.
pub(crate) fn assert_real_findings(findings: &[Finding]) {
    assert!(!findings.is_empty(), "the bad example passed");
    assert!(
        !findings
            .iter()
            .any(|finding| finding.message.contains("cargo xtask setup")),
        "the tool is not installed: {}",
        findings[0].message
    );
}
