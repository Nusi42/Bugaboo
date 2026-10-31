//! GIT-LINEAR: a push adds no merge commit, so `main` stays a straight line (ADR-0041).
//!
//! Branches are combined by rebasing onto the target branch and moving it forward with
//! `git merge --ff-only`. The check reads the commits a push can add, chosen the same way as for
//! SEC-PRIVATE ([`unpushed`]), and refuses every merge commit among them, naming the local
//! branches that hold it. The selection is wide on purpose: a merge commit on a stray local branch
//! blocks the push as well.
//!
//! It runs where the history is made: in the master (its push to the public remote) and in a
//! folder whose `origin` is a folder marked as the master (its push to the master). A plain clone
//! is not checked. When `origin` names a path that is no folder this program can open (a Git Bash
//! path like `/c/…`, a `file://` address, a path of WSL) or a folder without the mark, the check is
//! off; it says so in one line when `origin` looks like a path, and never fails for it.

use std::path::Path;

use super::{Check, Context, Finding, unpushed};
use crate::location::{self, Role};
use crate::output;
use crate::publish::is_local_remote;
use crate::repo::Repo;

/// What to do instead of a merge commit; the commit-message check says the same.
pub(crate) const REBASE_INSTEAD: &str = "Combine branches by rebasing onto the target branch \
     (`git rebase <target>` on a copy of the branch, then `git merge --ff-only` on the target), \
     never with a merge commit (ADR-0041).";

/// How many characters of a commit hash a finding shows.
const SHORT_HASH: usize = 8;

pub(crate) struct GitLinear;

impl Check for GitLinear {
    fn id(&self) -> &'static str {
        "GIT-LINEAR"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let repo = context.repo;
        match guard(repo, |name| std::env::var(name).ok()) {
            Guard::On => merge_findings(repo),
            Guard::Off => Ok(Vec::new()),
            Guard::OffForPath(url) => {
                output::info(&format!(
                    "GIT-LINEAR is off here: `origin` ({url}) is no folder marked as the master \
                     that this program can open (ADR-0041)."
                ));
                Ok(Vec::new())
            }
        }
    }
}

/// Whether the history of a checkout is checked.
#[derive(Debug, PartialEq, Eq)]
enum Guard {
    /// The master, or a folder whose `origin` is the master.
    On,
    /// A plain clone: no `origin`, or one on the network.
    Off,
    /// `origin` looks like a path, but not like a folder marked as the master; the URL.
    OffForPath(String),
}

/// Whether the history of this checkout is checked: it is the master (the environment `variable`
/// or its `.env.local` says so), or its `origin` is a folder marked as the master.
fn guard(repo: &Repo, variable: impl Fn(&str) -> Option<String>) -> Guard {
    if location::role_of(repo.root(), variable) == Role::Master {
        return Guard::On;
    }
    let url = repo
        .git_bytes(&["config", "--get", "remote.origin.url"])
        .map(|url| String::from_utf8_lossy(&url).trim().to_owned())
        .unwrap_or_default();
    if url.is_empty() || !is_local_remote(&url) {
        return Guard::Off;
    }
    let origin = repo.root().join(Path::new(&url));
    // The environment names the role of this checkout, not of its origin: only the mark counts.
    if origin.is_dir() && location::role_of(&origin, |_| None) == Role::Master {
        Guard::On
    } else {
        Guard::OffForPath(url)
    }
}

/// One finding for each merge commit a push can add, with the local branches that hold it.
fn merge_findings(repo: &Repo) -> anyhow::Result<Vec<Finding>> {
    if !unpushed::has_commits(repo) {
        return Ok(Vec::new());
    }
    let listing = repo.git_bytes(&unpushed::log_arguments(&["--merges", "--format=%H"], &[]))?;
    let mut findings = Vec::new();
    for hash in String::from_utf8_lossy(&listing)
        .lines()
        .map(str::trim)
        .filter(|hash| !hash.is_empty())
    {
        let branches = repo.git_bytes(&[
            "for-each-ref",
            "--contains",
            hash,
            "--format=%(refname:short)",
            "refs/heads",
        ])?;
        let branches: Vec<String> = String::from_utf8_lossy(&branches)
            .lines()
            .map(|name| format!("`{}`", name.trim()))
            .collect();
        let held = if branches.is_empty() {
            String::new()
        } else {
            format!(" (on the local branch {})", branches.join(", "))
        };
        let short: String = hash.chars().take(SHORT_HASH).collect();
        findings.push(Finding::at(
            format!("commit {short}{held}"),
            format!(
                "is a merge commit. {REBASE_INSTEAD} A local branch that is not needed any more \
                 is deleted."
            ),
        ));
    }
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::location::{ENV_FILE, ROLE_VARIABLE};
    use crate::testutil::TempDir;

    fn none(_: &str) -> Option<String> {
        None
    }

    fn commit(temp: &TempDir, message: &str) {
        assert!(temp.git(&["commit", "--quiet", "--allow-empty", "-m", message]));
    }

    /// Brings a side branch into the current one with a merge commit.
    fn merge_a_branch(temp: &TempDir, branch: &str) {
        let current = String::from_utf8(
            Repo::at(temp.path().to_path_buf())
                .git_bytes(&["branch", "--show-current"])
                .unwrap(),
        )
        .unwrap();
        assert!(temp.git(&["switch", "--quiet", "-c", branch]));
        commit(temp, "feat(web): add a board");
        assert!(temp.git(&["switch", "--quiet", current.trim()]));
        commit(temp, "fix(web): keep the list");
        assert!(temp.git(&["merge", "--quiet", "--no-ff", branch, "-m", "Merge a board"]));
    }

    /// A master like today's: `main` holds a merge commit. `marked` writes the master's mark.
    fn master(name: &str, marked: bool) -> TempDir {
        let temp = TempDir::new(name);
        assert!(temp.git(&["init", "--quiet", "-b", "main"]));
        commit(&temp, "chore: start");
        merge_a_branch(&temp, "side");
        if marked {
            temp.write(ENV_FILE, format!("{ROLE_VARIABLE}=master\n"));
        }
        temp
    }

    /// A clone of the master, as a work folder is.
    fn clone_of(name: &str, master: &TempDir) -> TempDir {
        let temp = TempDir::new(name);
        let url = master.path().to_string_lossy().into_owned();
        assert!(temp.git(&["clone", "--quiet", &url, "."]));
        assert!(temp.git(&["switch", "--quiet", "-c", "WIN9"]));
        temp
    }

    fn findings(temp: &TempDir) -> Vec<String> {
        let repo = Repo::at(temp.path().to_path_buf());
        assert_eq!(
            guard(&repo, none),
            Guard::On,
            "the folder should be checked"
        );
        merge_findings(&repo)
            .unwrap()
            .into_iter()
            .map(|finding| {
                format!(
                    "{} {}",
                    finding.location.unwrap_or_default(),
                    finding.message
                )
            })
            .collect()
    }

    #[test]
    fn good_example_a_push_on_top_of_a_main_whose_merge_commits_are_on_the_remote() {
        let master = master("linear-good-master", true);
        let work = clone_of("linear-good-work", &master);
        commit(&work, "fix(web): keep the board");
        commit(&work, "test(web): show the board");
        assert_eq!(findings(&work), Vec::<String>::new());
    }

    #[test]
    fn bad_example_a_merge_commit_in_what_a_push_adds() {
        let master = master("linear-bad-master", true);
        let work = clone_of("linear-bad-work", &master);
        merge_a_branch(&work, "other");
        let found = findings(&work);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("is a merge commit"), "{found:?}");
        assert!(found[0].contains("git rebase"), "{found:?}");
        assert!(found[0].contains("`WIN9`"), "{found:?}");
    }

    #[test]
    fn a_merge_commit_on_a_stray_local_branch_is_named() {
        let master = master("linear-stray-master", true);
        let work = clone_of("linear-stray-work", &master);
        assert!(work.git(&["switch", "--quiet", "-c", "old-try"]));
        merge_a_branch(&work, "older-try");
        assert!(work.git(&["switch", "--quiet", "WIN9"]));
        commit(&work, "fix(web): keep the board");
        let found = findings(&work);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("`old-try`"), "{found:?}");
        assert!(!found[0].contains("`WIN9`"), "{found:?}");
    }

    #[test]
    fn a_rebased_branch_passes() {
        let master = master("linear-rebase-master", true);
        let work = clone_of("linear-rebase-work", &master);
        assert!(work.git(&["switch", "--quiet", "-c", "copy"]));
        commit(&work, "feat(web): add a page");
        assert!(work.git(&["switch", "--quiet", "WIN9"]));
        commit(&work, "fix(web): keep a page");
        assert!(work.git(&["switch", "--quiet", "copy"]));
        assert!(work.git(&["rebase", "--quiet", "WIN9"]));
        assert!(work.git(&["switch", "--quiet", "WIN9"]));
        assert!(work.git(&["merge", "--quiet", "--ff-only", "copy"]));
        assert_eq!(findings(&work), Vec::<String>::new());
    }

    #[test]
    fn the_master_is_checked_on_what_the_public_remote_does_not_have() {
        let master = master("linear-publish", true);
        let repo = Repo::at(master.path().to_path_buf());
        assert_eq!(guard(&repo, none), Guard::On);
        // Nothing is on a remote yet, so the merge commit counts ...
        assert_eq!(merge_findings(&repo).unwrap().len(), 1);
        // ... and once the public remote has it, it does not.
        assert!(master.git(&["update-ref", "refs/remotes/github/main", "HEAD"]));
        assert!(merge_findings(&repo).unwrap().is_empty());
        merge_a_branch(&master, "late");
        assert_eq!(merge_findings(&repo).unwrap().len(), 1);
    }

    #[test]
    fn a_plain_clone_is_not_checked_and_an_unknown_path_says_so() {
        let upstream = master("linear-plain-upstream", false);
        let plain = clone_of("linear-plain", &upstream);
        let repo = Repo::at(plain.path().to_path_buf());
        let url = upstream.path().to_string_lossy().into_owned();
        // A folder without the mark: the check is off, and says so.
        assert_eq!(guard(&repo, none), Guard::OffForPath(url));
        // An address on the network is a plain clone.
        assert!(plain.git(&["remote", "set-url", "origin", "https://example.test/x.git"]));
        assert_eq!(guard(&repo, none), Guard::Off);
        // A path this program cannot open, like a Git Bash path that names no folder here.
        assert!(plain.git(&["remote", "set-url", "origin", "/made-up/master"]));
        assert_eq!(
            guard(&repo, none),
            Guard::OffForPath("/made-up/master".to_owned())
        );
        // A repository without a remote is no master either, unless the environment says so.
        let alone = master("linear-alone", false);
        let repo = Repo::at(alone.path().to_path_buf());
        assert_eq!(guard(&repo, none), Guard::Off);
        assert_eq!(
            guard(&repo, |name: &str| (name == ROLE_VARIABLE)
                .then(|| "master".to_owned())),
            Guard::On
        );
    }
}
