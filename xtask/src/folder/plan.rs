//! What `cargo xtask folder clean` deletes, decided before it changes anything.
//!
//! Git names the untracked paths of the folder (`git ls-files --others --directory`, so a big
//! `target/` is one name). This module looks at each of them on the disk and makes the plan: a
//! list of items to delete. A folder with nothing tracked and nothing to keep in it is one item;
//! any other folder is opened and its parts are judged one by one. Git's own answer for what is kept is not used, because its patterns do
//! not hold for a folder that it names as a whole.

use std::collections::HashSet;
use std::fs::{self, Metadata};
use std::path::Path;

use anyhow::{Context as _, anyhow, bail};

use crate::location::ENV_FILE;
use crate::master::CARGO_CONFIG;
use crate::tools::TOOLS_DIR;

/// The paths a clean leaves alone, from the root of the folder: the index of `CodeGraph`, the
/// pinned tools, and the local settings of the person, of Claude Code and of Cargo.
pub(super) const KEEP_AT_ROOT: [&str; 6] = [
    ".codegraph",
    TOOLS_DIR,
    ENV_FILE,
    ".claude/settings.local.json",
    "CLAUDE.local.md",
    CARGO_CONFIG,
];
/// The folders that are kept at any depth: the packages installed by npm.
pub(super) const KEEP_ANYWHERE: &str = "node_modules";

/// One path to delete, from the root of the folder; a folder has a `/` at the end.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Item {
    pub(super) path: String,
    pub(super) size: u64,
}

/// What will be deleted, and what is left alone because it is reached through a link.
#[derive(Debug, Default)]
pub(super) struct Plan {
    pub(super) items: Vec<Item>,
    pub(super) skipped: Vec<String>,
}

/// What the look at one path found.
enum Outcome {
    /// Nothing in it is tracked or kept: all of it goes, this many bytes.
    Whole(u64),
    /// It holds something to keep; the parts that go are in the plan already.
    Partial,
    /// It is tracked or kept itself.
    Protected,
}

/// The decision, for the folder at `root`.
pub(super) struct Planner<'a> {
    root: &'a Path,
    /// The file system does not tell `Web` from `web` (`core.ignorecase`).
    fold: bool,
    files: HashSet<String>,
    folders: HashSet<String>,
}

impl<'a> Planner<'a> {
    /// `tracked` are the files Git holds, in the index and in the branch the folder moves to.
    pub(super) fn new(root: &'a Path, fold: bool, tracked: &[String]) -> Self {
        let mut planner = Self {
            root,
            fold,
            files: HashSet::new(),
            folders: HashSet::new(),
        };
        for file in tracked {
            for (at, _) in file.match_indices('/') {
                planner.folders.insert(planner.key(&file[..at]));
            }
            planner.files.insert(planner.key(file));
        }
        planner
    }

    fn key(&self, path: &str) -> String {
        if self.fold {
            path.to_lowercase()
        } else {
            path.to_owned()
        }
    }

    /// Kept: a path at the root that is to be kept, or below one, or a folder (or a link, which
    /// may be a junction to a shared folder) called like the packages of npm, or anything below
    /// one. Compared by [`Planner::key`], so case is folded where the file system folds it.
    fn is_kept(&self, path: &str, meta: &Metadata) -> bool {
        let key = self.key(path);
        let anywhere = self.key(KEEP_ANYWHERE);
        let parts: Vec<&str> = key.split('/').collect();
        let last = parts.len() - 1;
        KEEP_AT_ROOT.iter().any(|kept| {
            let kept = self.key(kept);
            key == kept || key.starts_with(&format!("{kept}/"))
        }) || parts.iter().enumerate().any(|(at, part)| {
            *part == anywhere && (at < last || meta.is_dir() || meta.file_type().is_symlink())
        })
    }

    /// The plan for the `candidates` that Git names. Fails, so that nothing has been changed, when
    /// a nested Git repository is below one of them (its history would be lost) or a name is not
    /// text.
    pub(super) fn plan(&self, candidates: &[String]) -> anyhow::Result<Plan> {
        let mut plan = Plan::default();
        let mut nested = Vec::new();
        for candidate in candidates {
            let path = candidate.trim_end_matches('/');
            if through_link(self.root, path) {
                plan.skipped.push(path.to_owned());
                continue;
            }
            let meta = fs::symlink_metadata(self.root.join(path))
                .with_context(|| format!("could not look at {path}"))?;
            if let Outcome::Whole(size) = self.visit(path, &meta, &mut plan, &mut nested)? {
                plan.items.push(item(path, &meta, size));
            }
        }
        if !nested.is_empty() {
            bail!(
                "Git repositories of their own are inside the folder: {}. Nothing was changed; \
                 move them away or delete them by hand.",
                nested.join(", ")
            );
        }
        plan.items.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(plan)
    }

    fn visit(
        &self,
        path: &str,
        meta: &Metadata,
        plan: &mut Plan,
        nested: &mut Vec<String>,
    ) -> anyhow::Result<Outcome> {
        let is_dir = meta.is_dir();
        let key = self.key(path);
        if self.files.contains(&key)
            || self.is_kept(path, meta)
            || (!is_dir && self.folders.contains(&key))
        {
            return Ok(Outcome::Protected);
        }
        if key.rsplit('/').next() == Some(self.key(".git").as_str()) {
            let folder = &path[..path.len() - ".git".len()];
            nested.push(folder.trim_end_matches('/').to_owned());
        }
        if !is_dir {
            return Ok(Outcome::Whole(meta.len()));
        }
        let mut whole = true;
        let mut size = 0;
        let mut parts = Vec::new();
        for entry in fs::read_dir(self.root.join(path))
            .with_context(|| format!("could not read the folder {path}"))?
        {
            let entry = entry?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| anyhow!("a name in {path} is not text; delete it by hand"))?;
            let child = format!("{path}/{name}");
            let meta = entry.metadata()?;
            match self.visit(&child, &meta, plan, nested)? {
                Outcome::Whole(bytes) => {
                    size += bytes;
                    parts.push(item(&child, &meta, bytes));
                }
                Outcome::Partial | Outcome::Protected => whole = false,
            }
        }
        if whole && !self.folders.contains(&self.key(path)) {
            return Ok(Outcome::Whole(size));
        }
        plan.items.extend(parts);
        Ok(Outcome::Partial)
    }
}

fn item(path: &str, meta: &Metadata, size: u64) -> Item {
    let slash = if meta.is_dir() { "/" } else { "" };
    Item {
        path: format!("{path}{slash}"),
        size,
    }
}

/// Whether a folder on the way from `root` to `path` is a link (a junction on Windows), or cannot
/// be looked at: what is below it is not deleted, because the delete would go through the link.
pub(super) fn through_link(root: &Path, path: &str) -> bool {
    let parts: Vec<&str> = path.trim_end_matches('/').split('/').collect();
    let mut at = root.to_path_buf();
    parts[..parts.len() - 1].iter().any(|part| {
        at.push(part);
        fs::symlink_metadata(&at).map_or(true, |meta| meta.file_type().is_symlink())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::link_directory;
    use crate::testutil::TempDir;

    fn tree(temp: &TempDir, files: &[&str]) {
        for file in files {
            temp.write(file, "0123456789");
        }
    }

    fn paths(plan: &Plan) -> Vec<&str> {
        plan.items.iter().map(|item| item.path.as_str()).collect()
    }

    fn named(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn a_folder_with_something_to_keep_is_opened_and_the_rest_goes() {
        let temp = TempDir::new("plan-open");
        tree(
            &temp,
            &[
                "target/a/f",
                "target/b",
                "web/app/node_modules/x/f",
                "web/.codegraph/f",
                "web/app/dist/f",
                "web/app/dist/g",
                "web/t.ts",
                ".claude/settings.local.json",
                "lib/node_modules",
            ],
        );
        let tracked = named(&["web/t.ts", ".claude/memory/note.md"]);
        let planner = Planner::new(temp.path(), false, &tracked);
        let plan = planner
            .plan(&named(&[
                "target/",
                "web/app/",
                "web/.codegraph/",
                ".claude/",
                "lib/",
            ]))
            .unwrap();
        // `web/` itself is not named by Git here; `web/app/` holds a kept folder.
        assert_eq!(
            paths(&plan),
            ["lib/", "target/", "web/.codegraph/", "web/app/dist/"]
        );
        assert_eq!(plan.items[1].size, 20);
    }

    #[test]
    fn a_file_named_like_a_kept_folder_goes_and_a_tracked_file_stays() {
        let temp = TempDir::new("plan-names");
        tree(&temp, &["lib/node_modules", "lib/tracked.txt"]);
        let planner = Planner::new(temp.path(), false, &named(&["lib/tracked.txt"]));
        let plan = planner.plan(&named(&["lib/"])).unwrap();
        assert_eq!(paths(&plan), ["lib/node_modules"]);
    }

    #[test]
    fn bad_example_a_folder_that_differs_in_case_from_a_tracked_one() {
        let temp = TempDir::new("plan-case");
        tree(&temp, &["Web/dist/f"]);
        let tracked = named(&["web/x.ts"]);
        // With a file system that ignores case, `Web` is the folder of `web/x.ts`: not whole.
        let folding = Planner::new(temp.path(), true, &tracked);
        let plan = folding.plan(&named(&["Web/"])).unwrap();
        assert_eq!(paths(&plan), ["Web/dist/"]);
        // Where case counts, it is another folder.
        let exact = Planner::new(temp.path(), false, &tracked);
        assert_eq!(paths(&exact.plan(&named(&["Web/"])).unwrap()), ["Web/"]);
    }

    #[test]
    fn bad_example_a_git_repository_inside_is_refused_and_named() {
        let temp = TempDir::new("plan-nested");
        tree(&temp, &[".work/clone/.git/HEAD", ".work/other/f"]);
        let planner = Planner::new(temp.path(), false, &[]);
        let why = planner.plan(&named(&[".work/"])).unwrap_err().to_string();
        assert!(why.contains(".work/clone"), "{why}");
        assert!(temp.path().join(".work/clone/.git/HEAD").exists());
    }

    #[test]
    fn bad_example_nothing_is_deleted_through_a_link() {
        let temp = TempDir::new("plan-link");
        tree(&temp, &["real/x/f"]);
        let link = temp.path().join("link");
        link_directory(&temp.path().join("real"), &link);
        assert!(fs::symlink_metadata(&link).is_ok(), "no link was made");
        assert!(through_link(temp.path(), "link/x/"));
        assert!(!through_link(temp.path(), "real/x/"));
        let planner = Planner::new(temp.path(), false, &[]);
        let plan = planner.plan(&named(&["link/x/", "real/x/"])).unwrap();
        assert_eq!(plan.skipped, ["link/x"]);
        assert_eq!(paths(&plan), ["real/x/"]);
        // The link itself is a path of its own and goes as a link.
        let plan = planner.plan(&named(&["link"])).unwrap();
        assert_eq!(paths(&plan), ["link"]);
    }

    #[test]
    fn a_big_build_folder_is_one_item() {
        let temp = TempDir::new("plan-big");
        for folder in 0..40 {
            for file in 0..50 {
                temp.write(&format!("target/debug/{folder}/{file}"), "x");
            }
        }
        let planner = Planner::new(temp.path(), false, &named(&["Cargo.toml"]));
        let plan = planner.plan(&named(&["target/"])).unwrap();
        assert_eq!(paths(&plan), ["target/"]);
        assert_eq!(plan.items[0].size, 2000);
    }

    #[test]
    fn the_rules_of_keeping_hold_below_the_kept_path_and_in_any_case() {
        let temp = TempDir::new("plan-keep");
        tree(
            &temp,
            &[".tools/x", "Node_Modules/y", "a/node_modules/z/f", "b/c"],
        );
        let tracked = named(&[".tools/t", "node_modules/y/t"]);
        let candidates = named(&[".tools/x", "Node_Modules/y", "a/node_modules/z/f"]);
        // Below a kept path nothing goes, also beside a tracked file; where case counts,
        // `Node_Modules` is not the kept folder.
        let exact = Planner::new(temp.path(), false, &tracked);
        let plan = exact.plan(&candidates).unwrap();
        assert_eq!(paths(&plan), ["Node_Modules/y"]);
        let folding = Planner::new(temp.path(), true, &tracked);
        assert!(folding.plan(&candidates).unwrap().items.is_empty());
        let plan = folding.plan(&named(&["b/"])).unwrap();
        assert_eq!(paths(&plan), ["b/"]);
    }

    #[test]
    fn a_link_called_node_modules_is_kept_and_a_tracked_folder_that_is_a_file_stays() {
        let temp = TempDir::new("plan-links-kept");
        tree(&temp, &["shared/x/f", "a"]);
        let link = temp.path().join("node_modules");
        link_directory(&temp.path().join("shared"), &link);
        assert!(fs::symlink_metadata(&link).is_ok(), "no link was made");
        let planner = Planner::new(temp.path(), false, &named(&["a/b"]));
        let plan = planner.plan(&named(&["node_modules", "a"])).unwrap();
        assert!(plan.items.is_empty());
    }

    #[test]
    fn bad_example_a_git_repository_that_is_a_candidate_itself_beside_tracked_files() {
        let temp = TempDir::new("plan-nested-candidate");
        tree(&temp, &["a/clone/.git/HEAD", "a/clone/f"]);
        let planner = Planner::new(temp.path(), false, &named(&["a/tracked.txt"]));
        let why = planner.plan(&named(&["a/clone/"])).unwrap_err().to_string();
        assert!(why.contains("a/clone"), "{why}");
    }

    #[test]
    fn bad_example_a_git_folder_in_another_case_is_found_where_case_is_folded() {
        let temp = TempDir::new("plan-nested-case");
        tree(&temp, &["a/clone/.GIT/HEAD"]);
        let folding = Planner::new(temp.path(), true, &[]);
        let why = folding.plan(&named(&["a/"])).unwrap_err().to_string();
        assert!(why.contains("a/clone"), "{why}");
        let exact = Planner::new(temp.path(), false, &[]);
        assert!(exact.plan(&named(&["a/"])).is_ok());
    }
}
