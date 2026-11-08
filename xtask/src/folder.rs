//! `cargo xtask folder clean <n>`: gives back the disk space of an idle work folder (ADR-0024).
//!
//! A work folder `bugaboo.winN` next to the one the command runs from is cleaned in two steps:
//! its branch `WINn` moves to `origin/main`, and every file Git does not track (build output, the
//! built web app, logs in `.work/`) goes, except the paths in `plan.rs` (`KEEP_AT_ROOT`, `KEEP_ANYWHERE`). Before it changes
//! anything it checks that the clean loses no work and refuses, naming why, when it would.
//!
//! The command cannot tell whether an agent still works or waits in the folder. The lead checks
//! that first (04-milestones, "The lead's routine"). The lock of a full run (`fullRunLock`) is one
//! port for the whole computer, not one per folder, so it says nothing about the target.

mod plan;

use std::path::{Path, PathBuf};

use anyhow::{Context as _, bail};

use crate::dataset_large::shown;
use crate::location;
use crate::ports::Layout;
use crate::repo::{Repo, remove_link};
use plan::{Plan, Planner, through_link};

/// The branch of work folder number `n` is this prefix and `n` (ADR-0024).
const BRANCH_PREFIX: &str = "WIN";
/// The branch every clean moves to, on the remote of every work folder.
const MAIN: &str = "origin/main";

/// `cargo xtask folder clean <number>`, run from the work folder of `repo`. Tells `report` what
/// it does, line by line, as it does it.
pub(crate) fn clean(repo: &Repo, number: u32, report: &mut dyn FnMut(&str)) -> anyhow::Result<()> {
    let (layout, _) = Layout::load(repo.root())?;
    clean_folder(&layout, repo.root(), number, report, &REAL)
}

fn same_folder(a: &Path, b: &Path) -> bool {
    matches!((a.canonicalize(), b.canonicalize()), (Ok(a), Ok(b)) if a == b)
}

/// Deletes a file, a link or a folder. A link (a junction on Windows) goes as a link: what it
/// points to stays.
fn remove_path(path: &Path) -> std::io::Result<()> {
    remove_link(path).or_else(|_| {
        if std::fs::symlink_metadata(path)?.is_dir() {
            std::fs::remove_dir_all(path)
        } else {
            std::fs::remove_file(path)
        }
    })
}

/// The things a test replaces: the deleting, and two moments at which something else may happen.
struct Hooks<'a> {
    /// Called after the plan is made, before the branch moves.
    after_plan: &'a dyn Fn(),
    /// Called after the branch moved, before the first delete.
    before_delete: &'a dyn Fn(),
    remove: &'a dyn Fn(&Path) -> std::io::Result<()>,
}

fn nothing() {}

const REAL: Hooks<'static> = Hooks {
    after_plan: &nothing,
    before_delete: &nothing,
    remove: &remove_path,
};

fn clean_folder(
    layout: &Layout,
    own: &Path,
    number: u32,
    report: &mut dyn FnMut(&str),
    hooks: &Hooks<'_>,
) -> anyhow::Result<()> {
    let name = layout.work_folder_name(number);
    let target: PathBuf = own
        .parent()
        .context("this folder has no folder around it to hold the work folders")?
        .join(&name);
    let branch = format!("{BRANCH_PREFIX}{number}");
    if !target.join(".git").exists() {
        bail!(
            "{name} is not a work folder: {} is not a Git checkout.",
            target.display()
        );
    }
    if same_folder(own, &target) {
        bail!("{name} is the folder this command runs from; run it from another work folder.");
    }
    if location::marked_master(&target) {
        bail!("{name} is marked as the master; nothing is cleaned there.");
    }
    let folder = Repo::foreign(target.clone());
    let current = folder.git_text(&["rev-parse", "--abbrev-ref", "HEAD"])?;
    if current != branch {
        bail!("{name} is on the branch {current}, not {branch}; nothing was changed.");
    }
    let head = folder.git_text(&["rev-parse", "HEAD"])?;
    ensure_unchanged(&folder, &name, &head)?;
    folder.git_text(&["fetch", "-q", "--prune", "origin"])?;
    let merges = folder.git_lines(&["rev-list", "--merges", &format!("{MAIN}..HEAD")])?;
    if !merges.is_empty() {
        bail!(
            "{name} holds merge commits ({}); the branches are rebased, never merged. \
             Nothing was changed.",
            merges.join(", ")
        );
    }
    let unsaved = unsaved_commits(&folder, &branch)?;
    if !unsaved.is_empty() {
        bail!(
            "{name} holds commits that are neither in {MAIN} nor pushed to origin/{branch}: {}. \
             Nothing was changed.",
            unsaved.join(", ")
        );
    }
    // Last look before the first change: nobody committed or wrote a file meanwhile.
    ensure_unchanged(&folder, &name, &head)?;
    // Everything that can be refused is looked at before the branch moves.
    let plan = plan_of(&folder, &target)?;
    (hooks.after_plan)();
    ensure_unchanged(&folder, &name, &head)?;

    folder.git_text(&["switch", "-q", "--no-track", "-C", &branch, MAIN])?;
    report(&format!("{name}: branch {branch} moved to {MAIN}"));
    (hooks.before_delete)();
    carry_out(&folder, &target, &plan, report, hooks.remove).with_context(|| {
        format!("branch {branch} already moved to main; {name} is partly cleaned; run it again")
    })
}

/// Stops when `folder` is not at `head` or has files that are not committed. `git status` is
/// asked with the untracked files shown, whatever the setting of the folder says.
fn ensure_unchanged(folder: &Repo, name: &str, head: &str) -> anyhow::Result<()> {
    if folder.git_text(&["rev-parse", "HEAD"])? != head {
        bail!("{name} got a new commit while it was checked; nothing was changed.");
    }
    if !folder
        .git_text(&["status", "--porcelain", "--untracked-files=normal"])?
        .is_empty()
    {
        bail!("{name} has uncommitted or untracked files (git status); nothing was changed.");
    }
    Ok(())
}

/// The commits that only this folder holds: their change is not in `origin/main` (`git cherry`,
/// so a commit that was rebased into `main` counts as merged) and they are not on
/// `origin/<branch>` either.
fn unsaved_commits(folder: &Repo, branch: &str) -> anyhow::Result<Vec<String>> {
    let unmerged: Vec<String> = folder
        .git_lines(&["cherry", MAIN, "HEAD"])?
        .iter()
        .filter_map(|line| line.strip_prefix("+ "))
        .map(str::to_owned)
        .collect();
    let remote = format!("refs/remotes/origin/{branch}");
    if folder
        .git_bytes(&["rev-parse", "--verify", "--quiet", &remote])
        .is_err()
    {
        return Ok(unmerged);
    }
    let only_here = folder.git_lines(&["rev-list", "HEAD", "--not", &remote])?;
    Ok(unmerged
        .into_iter()
        .filter(|commit| only_here.contains(commit))
        .collect())
}

/// What the clean will delete: the untracked paths that Git names, looked at on the disk. The
/// files that Git holds are those of the index and of `origin/main`, where the branch moves to.
fn plan_of(folder: &Repo, root: &Path) -> anyhow::Result<Plan> {
    let fold = folder
        .git_text(&["config", "--bool", "--get", "core.ignorecase"])
        .is_ok_and(|value| value == "true");
    let mut tracked = folder.git_names_z(&["ls-files", "-z"])?;
    tracked.extend(folder.git_names_z(&["ls-tree", "-r", "-z", "--name-only", MAIN])?);
    let candidates = folder.git_names_z(&[
        "ls-files",
        "-z",
        "--others",
        "--directory",
        "--no-empty-directory",
    ])?;
    Planner::new(root, fold, &tracked).plan(&candidates)
}

/// Deletes what the plan names, telling `report` about each path, and ends with a look that no
/// tracked file is missing. The deleting is done here, because Rust opens long paths on Windows,
/// which the `target` folder of a big build needs.
fn carry_out(
    folder: &Repo,
    root: &Path,
    plan: &Plan,
    report: &mut dyn FnMut(&str),
    remove: &dyn Fn(&Path) -> std::io::Result<()>,
) -> anyhow::Result<()> {
    for path in &plan.skipped {
        report(&format!("skipped {path}: a link is on the way to it"));
    }
    let mut total = 0_u64;
    for item in &plan.items {
        let relative = item.path.trim_end_matches('/');
        if through_link(root, relative) {
            report(&format!("skipped {relative}: a link is on the way to it"));
            continue;
        }
        let path = root.join(relative);
        let is_dir = std::fs::symlink_metadata(&path).is_ok_and(|meta| meta.is_dir());
        if is_dir != item.path.ends_with('/') {
            report(&format!(
                "skipped {relative}: it is not what it was when looked at"
            ));
            continue;
        }
        remove(&path).with_context(|| format!("could not delete {}", path.display()))?;
        total += item.size;
        report(&format!("removed {} ({})", item.path, shown(item.size)));
    }
    report(&format!("freed {}", shown(total)));
    let missing = folder.git_names_z(&["ls-files", "-z", "--deleted"])?;
    if !missing.is_empty() {
        bail!(
            "tracked files are missing after the clean: {}",
            missing.join(", ")
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::location::ENV_FILE;
    use crate::repo::link_directory;
    use crate::testutil::TempDir;
    use std::fs;

    const LAYOUT: &str = r#"{"first":47200,"stride":100,"blocks":9,
        "blockVariable":"BUGABOO_PORT_BLOCK","workFolderPrefix":"bugaboo.win",
        "offsets":{"server":0}}"#;

    const IGNORED: &str = "target/\n.work/\nnode_modules/\n.codegraph/\n.tools/\n.env.local\n\
        .claude/settings.local.json\nCLAUDE.local.md\n.cargo/config.local.toml\ndata/\n\
        /lib/node_modules\n/scratch*\ngen/\nold/cache/\npkg/out/\n";

    fn git(dir: &Path, args: &[&str]) -> String {
        TempDir::git_in(dir, args)
    }

    fn commit(dir: &Path, file: &str, text: &str) -> String {
        let path = dir.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
        git(dir, &["add", file]);
        git(dir, &["commit", "-q", "-m", &format!("test: write {file}")]);
        git(dir, &["rev-parse", "HEAD"])
    }

    /// A remote, a clone that feeds it (`seed`), the running folder `bugaboo.win1` and the target
    /// `bugaboo.win2` on `WIN2`, both with ignored build output and local settings.
    struct World {
        _temp: TempDir,
        layout: Layout,
        seed: PathBuf,
        own: PathBuf,
        target: PathBuf,
    }

    impl World {
        fn new(name: &str) -> Self {
            let temp = TempDir::new(name);
            let root = temp.path().to_path_buf();
            let remote = root.join("remote.git");
            fs::create_dir_all(&remote).unwrap();
            git(&remote, &["init", "-q", "--bare", "-b", "main"]);
            let url = remote.to_str().unwrap().to_owned();
            let seed = root.join("seed");
            git(&root, &["clone", "-q", &url, "seed"]);
            git(&seed, &["switch", "-q", "-C", "main"]);
            fs::write(seed.join(".gitignore"), IGNORED).unwrap();
            git(&seed, &["add", ".gitignore"]);
            commit(&seed, "base.txt", "base\n");
            commit(&seed, ".claude/memory/note.md", "a note\n");
            commit(&seed, "pkg/keep.txt", "kept\n");
            git(&seed, &["push", "-q", "origin", "main"]);
            let folder = |name: &str| {
                git(&root, &["clone", "-q", &url, name]);
                let path = root.join(name);
                for dir in [
                    "target/debug",
                    "pkg/out",
                    ".work",
                    "data",
                    "node_modules/x",
                    "web/app/node_modules/x",
                    "web/.codegraph",
                    ".codegraph",
                    ".tools",
                ] {
                    fs::create_dir_all(path.join(dir)).unwrap();
                    fs::write(path.join(dir).join("file"), "0123456789").unwrap();
                }
                for dir in [".cargo", "lib", ".claude"] {
                    fs::create_dir_all(path.join(dir)).unwrap();
                }
                for file in [
                    ".env.local",
                    ".claude/settings.local.json",
                    "CLAUDE.local.md",
                    ".cargo/config.local.toml",
                    "lib/node_modules",
                    "scratch 'q' \u{e9}&$%;.txt",
                ] {
                    fs::write(path.join(file), "x").unwrap();
                }
                path
            };
            let own = folder("bugaboo.win1");
            let target = folder("bugaboo.win2");
            git(&target, &["switch", "-q", "-C", "WIN2", "origin/main"]);
            Self {
                _temp: temp,
                layout: Layout::parse(LAYOUT).unwrap(),
                seed,
                own,
                target,
            }
        }

        /// Cleans folder `number`; returns what was reported, also when it stops half way.
        fn run(&self, number: u32, hooks: &Hooks<'_>) -> (Vec<String>, anyhow::Result<()>) {
            let mut said = Vec::new();
            let result = clean_folder(
                &self.layout,
                &self.own,
                number,
                &mut |line| said.push(line.to_owned()),
                hooks,
            );
            (said, result)
        }

        fn clean(&self, number: u32) -> anyhow::Result<Vec<String>> {
            let (said, result) = self.run(number, &REAL);
            result.map(|()| said)
        }

        /// The clean of folder 2 is refused with all of `words` in the reason and changes nothing.
        fn refused(&self, words: &[&str]) {
            let why = format!("{:#}", self.clean(2).unwrap_err());
            for word in words {
                assert!(why.contains(word), "{why:?} lacks {word:?}");
            }
            assert!(self.target.join("target/debug/file").exists());
            assert_eq!(git(&self.target, &["branch", "--show-current"]), "WIN2");
        }
    }

    #[test]
    fn good_example_a_folder_on_main_is_cleaned_and_keeps_its_settings() {
        let world = World::new("folder-clean-ok");
        let done = world.clean(2).unwrap();
        assert!(done.iter().any(|line| line.starts_with("removed target/")));
        assert!(done.last().unwrap().starts_with("freed "));
        let target = &world.target;
        for gone in [
            "target",
            ".work",
            "data",
            "web/.codegraph",
            "lib/node_modules",
            "scratch 'q' \u{e9}&$%;.txt",
        ] {
            assert!(!target.join(gone).exists(), "{gone} is still there");
        }
        for kept in [
            ".codegraph/file",
            ".tools/file",
            "node_modules/x/file",
            "web/app/node_modules/x/file",
            ".env.local",
            ".claude/settings.local.json",
            ".claude/memory/note.md",
            "CLAUDE.local.md",
            ".cargo/config.local.toml",
            "base.txt",
        ] {
            assert!(target.join(kept).exists(), "{kept} was deleted");
        }
        assert_eq!(git(target, &["branch", "--show-current"]), "WIN2");
        assert_eq!(
            git(target, &["rev-parse", "HEAD"]),
            git(target, &["rev-parse", "origin/main"])
        );
        // The own folder was not touched.
        assert!(world.own.join("target/debug/file").exists());
    }

    #[test]
    fn good_example_a_link_in_the_build_output_goes_and_what_it_points_to_stays() {
        let world = World::new("folder-clean-link");
        let link = world.target.join("target").join("link");
        link_directory(&world.target.join("node_modules"), &link);
        assert!(fs::symlink_metadata(&link).is_ok(), "no link was made");
        world.clean(2).unwrap();
        assert!(!world.target.join("target").exists());
        assert!(world.target.join("node_modules/x/file").exists());
    }

    #[test]
    fn bad_example_a_git_repository_inside_stops_the_clean_before_anything_changes() {
        let world = World::new("folder-clean-nested");
        let clone = world.target.join(".work").join("clone");
        fs::create_dir_all(&clone).unwrap();
        git(&clone, &["init", "-q"]);
        commit(&clone, "precious.txt", "mine\n");
        let why = format!("{:#}", world.clean(2).unwrap_err());
        assert!(why.contains(".work/clone"), "{why}");
        assert!(clone.join("precious.txt").exists());
        assert!(world.target.join("target/debug/file").exists());
        assert_eq!(git(&world.target, &["branch", "--show-current"]), "WIN2");
    }

    #[test]
    fn bad_example_another_branch_is_refused() {
        let world = World::new("folder-clean-branch");
        git(&world.target, &["switch", "-q", "-c", "other"]);
        let why = world.clean(2).unwrap_err().to_string();
        assert!(why.contains("other") && why.contains("WIN2"), "{why}");
        assert!(world.target.join("target/debug/file").exists());
        assert_eq!(git(&world.target, &["branch", "--show-current"]), "other");
    }

    #[test]
    fn bad_example_uncommitted_changes_are_refused() {
        let world = World::new("folder-clean-dirty");
        fs::write(world.target.join("base.txt"), "changed\n").unwrap();
        world.refused(&["uncommitted"]);
    }

    #[test]
    fn bad_example_an_untracked_file_is_refused_whatever_the_setting_of_the_folder() {
        let world = World::new("folder-clean-untracked");
        fs::write(world.target.join("notes.txt"), "mine\n").unwrap();
        world.refused(&["untracked"]);
        // The setting of the folder that hides untracked files in `git status` does not count.
        git(
            &world.target,
            &["config", "status.showUntrackedFiles", "no"],
        );
        assert!(git(&world.target, &["status", "--porcelain"]).is_empty());
        world.refused(&["untracked"]);
    }

    #[test]
    fn bad_example_a_folder_that_changed_since_the_first_look_is_refused() {
        let world = World::new("folder-clean-changed");
        let folder = Repo::at(world.target.clone());
        let head = folder.git_text(&["rev-parse", "HEAD"]).unwrap();
        assert!(ensure_unchanged(&folder, "bugaboo.win2", &head).is_ok());
        fs::write(world.target.join("late.txt"), "late\n").unwrap();
        assert!(ensure_unchanged(&folder, "bugaboo.win2", &head).is_err());
        fs::remove_file(world.target.join("late.txt")).unwrap();
        commit(&world.target, "later.txt", "later\n");
        assert!(ensure_unchanged(&folder, "bugaboo.win2", &head).is_err());
    }

    #[test]
    fn good_example_a_commit_rebased_into_main_counts_as_merged() {
        let world = World::new("folder-clean-rebased");
        commit(&world.target, "work.txt", "work\n");
        // The same change reaches main as another commit, as after a rebase.
        commit(&world.seed, "work.txt", "work\n");
        git(&world.seed, &["push", "-q", "origin", "main"]);
        world.clean(2).unwrap();
        assert!(!world.target.join("target").exists());
        assert!(world.target.join("work.txt").exists());
    }

    #[test]
    fn bad_example_a_commit_only_here_is_refused_and_named() {
        let world = World::new("folder-clean-local");
        let id = commit(&world.target, "work.txt", "work\n");
        world.refused(&[&id]);
        // A pushed older commit does not cover a newer one that is only here.
        git(&world.target, &["push", "-q", "origin", "WIN2"]);
        let newer = commit(&world.target, "more.txt", "more\n");
        world.refused(&[&newer]);
    }

    #[test]
    fn good_example_a_commit_pushed_to_its_branch_is_safe() {
        let world = World::new("folder-clean-pushed");
        commit(&world.target, "work.txt", "work\n");
        git(&world.target, &["push", "-q", "origin", "WIN2"]);
        world.clean(2).unwrap();
        assert!(!world.target.join("target").exists());
        assert!(!world.target.join("work.txt").exists());
        assert_eq!(
            git(&world.target, &["rev-parse", "HEAD"]),
            git(&world.target, &["rev-parse", "origin/main"])
        );
    }

    #[test]
    fn bad_example_an_error_half_way_says_that_the_branch_moved_and_a_second_run_finishes() {
        let world = World::new("folder-clean-half");
        let fails = |path: &Path| {
            if path.ends_with("data") {
                Err(std::io::Error::other("busy"))
            } else {
                remove_path(path)
            }
        };
        let (said, result) = world.run(
            2,
            &Hooks {
                remove: &fails,
                ..REAL
            },
        );
        let why = format!("{:#}", result.unwrap_err());
        assert!(
            why.contains("branch WIN2 already moved to main")
                && why.contains("partly cleaned")
                && why.contains("run it again"),
            "{why}"
        );
        // What was done before the error was told when it happened.
        assert!(said.iter().any(|line| line.starts_with("removed .work/")));
        assert!(world.target.join("data").exists());
        world.clean(2).unwrap();
        assert!(!world.target.join("data").exists());
    }

    #[test]
    fn bad_example_the_master_the_own_folder_and_no_folder_are_refused() {
        let world = World::new("folder-clean-roles");
        fs::write(
            world.target.join(ENV_FILE),
            "BUGABOO_CHECKOUT_ROLE=master\n",
        )
        .unwrap();
        world.refused(&["master"]);
        let own = world.clean(1).unwrap_err().to_string();
        assert!(own.contains("runs from"), "{own}");
        assert!(world.own.join("target/debug/file").exists());
        let none = world.clean(7).unwrap_err().to_string();
        assert!(none.contains("not a work folder"), "{none}");
    }

    #[test]
    fn bad_example_a_commit_made_after_the_plan_is_refused() {
        let world = World::new("folder-clean-late");
        let target = world.target.clone();
        let late = || {
            commit(&target, "late.txt", "late\n");
        };
        let (_, result) = world.run(
            2,
            &Hooks {
                after_plan: &late,
                ..REAL
            },
        );
        let why = format!("{:#}", result.unwrap_err());
        assert!(why.contains("new commit"), "{why}");
        assert!(world.target.join("target/debug/file").exists());
        assert_eq!(git(&world.target, &["branch", "--show-current"]), "WIN2");
    }

    #[test]
    fn bad_example_a_merge_commit_is_refused() {
        let world = World::new("folder-clean-merge");
        git(&world.target, &["switch", "-q", "-c", "side"]);
        commit(&world.target, "side.txt", "side\n");
        git(&world.target, &["switch", "-q", "WIN2"]);
        git(
            &world.target,
            &["merge", "-q", "--no-ff", "-m", "test: merge", "side"],
        );
        world.refused(&["merge commits"]);
    }

    #[test]
    fn bad_example_a_link_that_appears_after_the_plan_is_not_followed() {
        let world = World::new("folder-clean-late-link");
        let target = world.target.clone();
        let swap = || {
            let real = target.join("pkg_real");
            fs::rename(target.join("pkg"), &real).unwrap();
            link_directory(&real, &target.join("pkg"));
        };
        let (said, _) = world.run(
            2,
            &Hooks {
                before_delete: &swap,
                ..REAL
            },
        );
        assert!(said.iter().any(|line| line.starts_with("skipped pkg/out")));
        assert!(target.join("pkg_real/out/file").exists());
    }

    #[test]
    fn bad_example_a_folder_that_became_a_file_is_not_deleted_as_a_folder() {
        let world = World::new("folder-clean-kind");
        let dist = world.target.join("web/app/dist");
        fs::create_dir_all(&dist).unwrap();
        let swap = || {
            fs::remove_dir_all(&dist).unwrap();
            fs::write(&dist, "now a file").unwrap();
        };
        let (said, _) = world.run(
            2,
            &Hooks {
                before_delete: &swap,
                ..REAL
            },
        );
        assert!(
            said.iter()
                .any(|line| line.starts_with("skipped web/app/dist"))
        );
        assert!(world.target.join("web/app/dist").is_file());
    }

    #[test]
    fn bad_example_a_tracked_file_that_is_missing_afterwards_fails_loudly() {
        let world = World::new("folder-clean-missing");
        let base = world.target.join("base.txt");
        let spoil = |path: &Path| {
            let _ = fs::remove_file(&base);
            remove_path(path)
        };
        let (_, result) = world.run(
            2,
            &Hooks {
                remove: &spoil,
                ..REAL
            },
        );
        let why = format!("{:#}", result.unwrap_err());
        assert!(why.contains("tracked files are missing") && why.contains("base.txt"));
    }

    #[test]
    fn good_example_a_path_that_main_tracks_but_the_old_branch_ignored_is_kept() {
        let world = World::new("folder-clean-tracked-in-main");
        fs::create_dir_all(world.seed.join("gen")).unwrap();
        fs::write(world.seed.join("gen/out.txt"), "main\n").unwrap();
        git(&world.seed, &["add", "-f", "gen/out.txt"]);
        git(&world.seed, &["commit", "-q", "-m", "test: track gen"]);
        git(&world.seed, &["push", "-q", "origin", "main"]);
        fs::create_dir_all(world.target.join("gen")).unwrap();
        fs::write(world.target.join("gen/out.txt"), "old\n").unwrap();
        world.clean(2).unwrap();
        let text = fs::read_to_string(world.target.join("gen/out.txt")).unwrap();
        assert_eq!(text, "main\n");
    }

    #[test]
    fn good_example_a_folder_of_the_old_branch_that_main_lacks_loses_its_ignored_files() {
        let world = World::new("folder-clean-old-folder");
        commit(&world.target, "old/t.txt", "old\n");
        git(&world.target, &["push", "-q", "origin", "WIN2"]);
        fs::create_dir_all(world.target.join("old/cache")).unwrap();
        fs::write(world.target.join("old/cache/f"), "x").unwrap();
        world.clean(2).unwrap();
        assert!(!world.target.join("old/cache").exists());
        assert!(!world.target.join("old/t.txt").exists());
    }

    #[test]
    fn git_asks_the_folder_it_runs_in_whatever_the_environment_says() {
        let world = World::new("folder-clean-git-dir");
        let other = world.seed.join(".git");
        let ask = |isolated: bool| {
            let mut command = crate::tools::command("git");
            command.env("GIT_DIR", &other);
            if isolated {
                crate::repo::isolate_git(&mut command);
            }
            let out = command
                .args(["rev-parse", "--absolute-git-dir"])
                .current_dir(&world.target)
                .output()
                .unwrap();
            String::from_utf8_lossy(&out.stdout)
                .trim()
                .replace('\\', "/")
        };
        let own = world
            .target
            .join(".git")
            .to_string_lossy()
            .replace('\\', "/");
        let seed = other.to_string_lossy().replace('\\', "/");
        assert!(ask(false).ends_with(seed.trim_start_matches("//?/")));
        assert!(ask(true).ends_with(own.trim_start_matches("//?/")));
    }

    #[test]
    fn the_help_and_the_docs_name_everything_that_is_kept() {
        use clap::CommandFactory as _;
        let mut cli = crate::Cli::command();
        let clean = cli
            .find_subcommand_mut("folder")
            .and_then(|folder| folder.find_subcommand_mut("clean"))
            .unwrap();
        let help = clean.render_long_help().to_string();
        let docs = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("docs/04-milestones.md");
        let docs = fs::read_to_string(docs).unwrap();
        // Only the line about cleaning counts, not any other line that names a path.
        let docs = docs
            .lines()
            .find(|line| line.contains("Cleaning an idle folder"))
            .unwrap();
        assert!(
            help.contains("merge commit"),
            "the help lacks the merge commits"
        );
        for kept in plan::KEEP_AT_ROOT.iter().chain([&plan::KEEP_ANYWHERE]) {
            assert!(help.contains(kept), "the help lacks {kept}");
            assert!(docs.contains(kept), "docs/04-milestones.md lacks {kept}");
        }
    }
}
