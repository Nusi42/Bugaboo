//! The repository the checks run on, and the Git questions they ask.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, bail};

/// A Bugaboo repository (a work folder or a check fixture).
pub(crate) struct Repo {
    root: PathBuf,
}

impl Repo {
    /// The repository containing this xtask crate.
    pub(crate) fn discover() -> anyhow::Result<Self> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let root = manifest_dir
            .parent()
            .context("xtask must live in a folder inside the repository")?
            .to_path_buf();
        Ok(Self { root })
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    /// Files staged for the next commit, as repository-relative paths with `/`.
    pub(crate) fn staged_files(&self) -> anyhow::Result<Vec<String>> {
        self.git_lines(&["diff", "--cached", "--name-only", "--diff-filter=ACMR"])
    }

    /// Every file Git knows (tracked, or untracked but not ignored), with `/` separators.
    pub(crate) fn all_files(&self) -> anyhow::Result<Vec<String>> {
        let mut files =
            self.git_lines(&["ls-files", "--cached", "--others", "--exclude-standard"])?;
        // A file that is deleted but not yet staged is still listed by Git.
        files.retain(|file| self.root.join(file).is_file());
        files.sort();
        files.dedup();
        Ok(files)
    }

    /// Turns paths given on the command line into repository-relative paths with `/`.
    pub(crate) fn relative_paths(&self, paths: &[PathBuf]) -> Vec<String> {
        paths
            .iter()
            .map(|path| {
                let relative = path.strip_prefix(&self.root).unwrap_or(path);
                relative.to_string_lossy().replace('\\', "/")
            })
            .collect()
    }

    fn git_lines(&self, args: &[&str]) -> anyhow::Result<Vec<String>> {
        let out = Command::new("git")
            .args(["-c", "core.quotepath=false"])
            .args(args)
            .current_dir(&self.root)
            .output()
            .context("could not run git; is it installed and on the PATH?")?;
        if !out.status.success() {
            bail!(
                "git {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect())
    }
}
