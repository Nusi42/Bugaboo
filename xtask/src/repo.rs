//! The repository the checks run on, and the Git questions they ask.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, bail};

/// The working files that were swapped for their staged content; dropping restores them.
pub(crate) struct StagedView {
    /// The working file, and the copy of its unstaged content (`None`: the file did not exist).
    saved: Vec<(PathBuf, Option<PathBuf>)>,
    backup: PathBuf,
}

impl StagedView {
    fn save(&mut self, path: &Path, name: &str, working: Option<Vec<u8>>) -> anyhow::Result<()> {
        let copy = match working {
            Some(bytes) => {
                let copy = self.backup.join(name);
                fs::create_dir_all(copy.parent().unwrap_or(&self.backup))?;
                fs::write(&copy, bytes)
                    .with_context(|| format!("could not back up {name} before checking"))?;
                Some(copy)
            }
            None => None,
        };
        self.saved.push((path.to_path_buf(), copy));
        Ok(())
    }
}

impl Drop for StagedView {
    fn drop(&mut self) {
        for (path, copy) in &self.saved {
            // Best effort: the backup folder keeps the unstaged content if restoring fails.
            let _ = match copy {
                Some(copy) => fs::copy(copy, path).map(|_| ()),
                None => fs::remove_file(path),
            };
        }
        if !self.saved.is_empty() {
            let _ = fs::remove_dir_all(&self.backup);
        }
    }
}

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

    /// Makes the working tree show exactly what is staged, until the returned guard is dropped
    /// (M0-F09). Files with changes that are not staged are put back afterwards, byte for byte.
    pub(crate) fn staged_view(&self, staged: &[String]) -> anyhow::Result<StagedView> {
        let changed = self.git_lines(&["diff", "--name-only"])?;
        let mut view = StagedView {
            saved: Vec::new(),
            backup: self.root.join("target").join("staged-backup"),
        };
        for file in staged.iter().filter(|file| changed.contains(file)) {
            let path = self.root.join(file);
            let working = fs::read(&path).ok();
            let content = self.git_bytes(&["show", &format!(":{file}")])?;
            view.save(&path, file, working)?;
            fs::write(&path, content).with_context(|| format!("could not write {file}"))?;
        }
        Ok(view)
    }

    fn git_bytes(&self, args: &[&str]) -> anyhow::Result<Vec<u8>> {
        let out = Command::new("git")
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
        Ok(out.stdout)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    fn repo_at(temp: &TempDir) -> Repo {
        Repo {
            root: temp.path().to_path_buf(),
        }
    }

    #[test]
    fn good_example_the_staged_view_shows_staged_content_and_restores_the_rest() {
        let temp = TempDir::new("staged-view");
        temp.write(
            "a.txt",
            "committed
",
        );
        assert!(temp.git(&["init", "--quiet"]));
        assert!(temp.git(&["add", "."]));
        assert!(temp.git(&["commit", "--quiet", "-m", "chore: test"]));
        temp.write(
            "a.txt", "staged
",
        );
        assert!(temp.git(&["add", "a.txt"]));
        temp.write(
            "a.txt",
            "staged
plus unstaged
",
        );
        let repo = repo_at(&temp);
        {
            let _view = repo.staged_view(&["a.txt".to_owned()]).unwrap();
            assert_eq!(
                fs::read_to_string(temp.path().join("a.txt")).unwrap(),
                "staged
"
            );
        }
        assert_eq!(
            fs::read_to_string(temp.path().join("a.txt")).unwrap(),
            "staged
plus unstaged
"
        );
        assert!(!temp.path().join("target").join("staged-backup").exists());
    }
}
