//! The repository the checks run on, and the Git questions they ask.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, bail};

/// A Bugaboo repository (a work folder or a check fixture).
pub(crate) struct Repo {
    root: PathBuf,
    /// Set while checking staged content: a read-only export of the index.
    view: Option<PathBuf>,
}

/// An export of the staged files; dropping it removes the export (and only the export).
pub(crate) struct StagedTree {
    pub(crate) repo: Repo,
    dir: PathBuf,
}

impl Drop for StagedTree {
    fn drop(&mut self) {
        remove_export(&self.dir);
    }
}

/// Removes an export. The `node_modules` link is removed as a link first, so the real folder
/// it points to can never be touched.
fn remove_export(dir: &Path) {
    let link = dir.join("node_modules");
    let _ = fs::remove_dir(&link).or_else(|_| fs::remove_file(&link));
    let _ = fs::remove_dir_all(dir);
}

/// A folder link: a symbolic link on Unix, a junction on Windows (no special rights needed).
fn link_directory(target: &Path, link: &Path) {
    if !target.is_dir() {
        return;
    }
    #[cfg(unix)]
    let _ = std::os::unix::fs::symlink(target, link);
    #[cfg(windows)]
    let _ = Command::new("cmd")
        .arg("/C")
        .arg("mklink")
        .arg("/J")
        .arg(link)
        .arg(target)
        .output();
}

impl Repo {
    /// The repository (or fixture) whose folder is `root`.
    #[cfg(test)]
    pub(crate) fn at(root: PathBuf) -> Self {
        Self { root, view: None }
    }

    /// The repository containing this xtask crate.
    pub(crate) fn discover() -> anyhow::Result<Self> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let root = manifest_dir
            .parent()
            .context("xtask must live in a folder inside the repository")?
            .to_path_buf();
        Ok(Self { root, view: None })
    }

    /// Where the files to check are: the working tree, or the export of the staged files.
    pub(crate) fn root(&self) -> &Path {
        self.view.as_deref().unwrap_or(&self.root)
    }

    /// The real repository, for Git itself (it has the `.git` folder).
    pub(crate) fn real_root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn path(&self, relative: &str) -> PathBuf {
        self.root().join(relative)
    }

    /// Files staged for the next commit, as repository-relative paths with `/`.
    pub(crate) fn staged_files(&self) -> anyhow::Result<Vec<String>> {
        self.git_lines(&["diff", "--cached", "--name-only", "--diff-filter=ACMR"])
    }

    /// Every file Git knows (tracked, or untracked but not ignored), with `/` separators.
    pub(crate) fn all_files(&self) -> anyhow::Result<Vec<String>> {
        // In an export only the index counts: untracked files are not part of the commit.
        let mut files = if self.view.is_some() {
            self.git_lines(&["ls-files", "--cached"])?
        } else {
            self.git_lines(&["ls-files", "--cached", "--others", "--exclude-standard"])?
        };
        // A file that is deleted but not yet staged is still listed by Git.
        files.retain(|file| self.root().join(file).is_file());
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

    /// Exports the index (what the next commit will contain) into `target/staged-tree` and
    /// returns a repository whose files are that export (M0-F09).
    ///
    /// The developer's working tree is never written to: no file is swapped, so a crash, a
    /// Ctrl+C or an editor autosave cannot lose anything. Tools still come from the real
    /// repository (`.tools/`), and `node_modules` is linked into the export so tools that look
    /// for it next to the files find it. Checks that need Git itself (secrets, commit message,
    /// file sizes) use [`Repo::real_root`].
    pub(crate) fn export_staged(&self) -> anyhow::Result<StagedTree> {
        let dir = self.root.join("target").join("staged-tree");
        remove_export(&dir);
        fs::create_dir_all(&dir).context("could not create the folder for the staged files")?;
        let prefix = format!("--prefix={}/", dir.to_string_lossy().replace('\\', "/"));
        self.git_bytes(&["checkout-index", "--all", "--force", &prefix])?;
        link_directory(&self.root.join("node_modules"), &dir.join("node_modules"));
        // Tools that look for `.git` to find the root of the project (rumdl and others) must
        // find the export, not the real repository above `target/`, or their path rules would
        // see `target/staged-tree/...` (M1-F17). An empty folder is enough for that; Git itself
        // is only ever run in the real repository.
        fs::create_dir_all(dir.join(".git"))
            .context("could not mark the staged files as a project root")?;
        Ok(StagedTree {
            repo: Self {
                root: self.root.clone(),
                view: Some(dir.clone()),
            },
            dir,
        })
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
            view: None,
        }
    }

    /// Every file below `dir` (except Git's folder and build output) with its bytes.
    fn snapshot(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
        let mut found = Vec::new();
        let mut open = vec![dir.to_path_buf()];
        while let Some(folder) = open.pop() {
            for entry in fs::read_dir(&folder).unwrap().flatten() {
                let path = entry.path();
                let name = entry.file_name();
                if name == ".git" || name == "target" {
                    continue;
                }
                if path.is_dir() {
                    open.push(path);
                } else {
                    found.push((path.clone(), fs::read(&path).unwrap()));
                }
            }
        }
        found.sort();
        found
    }

    #[test]
    fn good_example_the_export_has_the_staged_files_and_the_working_tree_is_untouched() {
        let temp = TempDir::new("staged-tree");
        temp.write("a.txt", "committed\n");
        temp.write("b.txt", "b committed\n");
        assert!(temp.git(&["init", "--quiet"]));
        assert!(temp.git(&["add", "."]));
        assert!(temp.git(&["commit", "--quiet", "-m", "chore: test"]));
        temp.write("a.txt", "staged\n");
        assert!(temp.git(&["add", "a.txt"]));
        temp.write("a.txt", "staged\nplus unstaged\n");
        temp.write("b.txt", "only unstaged\n");
        temp.write("untracked.txt", "new\n");
        let before = snapshot(temp.path());
        let repo = repo_at(&temp);
        {
            let tree = repo.export_staged().unwrap();
            let read = |name: &str| fs::read_to_string(tree.repo.path(name)).unwrap();
            assert_eq!(read("a.txt"), "staged\n");
            assert_eq!(read("b.txt"), "b committed\n");
            assert!(!tree.repo.path("untracked.txt").exists());
            assert_eq!(tree.repo.all_files().unwrap(), ["a.txt", "b.txt"]);
            assert_eq!(tree.repo.real_root(), temp.path());
            // The export is its own project root for the tools that look for `.git` (M1-F17).
            let marker = tree.repo.path(".git");
            assert!(marker.is_dir());
            assert_eq!(fs::read_dir(&marker).unwrap().count(), 0);
        }
        assert_eq!(snapshot(temp.path()), before, "the working tree changed");
        assert!(!temp.path().join("target").join("staged-tree").exists());
    }

    #[test]
    fn a_linked_node_modules_survives_removing_the_export() {
        let temp = TempDir::new("staged-links");
        temp.write("a.txt", "x\n");
        temp.write("node_modules/pkg/index.js", "keep me\n");
        assert!(temp.git(&["init", "--quiet"]));
        assert!(temp.git(&["add", "a.txt"]));
        drop(repo_at(&temp).export_staged().unwrap());
        assert!(temp.path().join("node_modules/pkg/index.js").is_file());
    }
}
