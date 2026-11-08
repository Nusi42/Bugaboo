//! Helpers for the `bad_example` / `good_example` tests: small throw-away repositories.

use std::fs;
use std::path::{Path, PathBuf};

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

    /// The build folder of a cargo project made here: its own, inside this folder. With the
    /// `CARGO_TARGET_DIR` of the run, every example project called `demo` would share one build
    /// and could find the results of another example there.
    pub(crate) fn cargo_env(&self) -> [(&'static str, String); 1] {
        let target = self.path.join("target");
        [("CARGO_TARGET_DIR", target.to_string_lossy().into_owned())]
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
        git_command(&self.path, args)
            .output()
            .unwrap()
            .status
            .success()
    }

    /// Runs `git` in `dir` (a folder made here) and returns its trimmed output; a failure
    /// stops the test and shows what Git said.
    pub(crate) fn git_in(dir: &Path, args: &[&str]) -> String {
        let out = git_command(dir, args).output().unwrap();
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_owned()
    }
}

/// `git` in `dir` with an identity and without the settings of the person who runs the test.
fn git_command(dir: &Path, args: &[&str]) -> std::process::Command {
    let mut command = crate::tools::command("git");
    command
        .args([
            "-c",
            "user.name=tester",
            "-c",
            "user.email=1+tester@users.noreply.github.com",
        ])
        .args(["-c", "core.autocrlf=false", "-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(dir);
    command
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
