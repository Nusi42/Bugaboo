//! GIT-LINEAR: a push adds no merge commit, so `main` stays a straight line (ADR-0041).
//!
//! Branches are combined by rebasing onto the target branch and moving it forward with
//! `git merge --ff-only`. The check reads the commits a push can add, chosen the same way as for
//! SEC-PRIVATE ([`unpushed`]), and refuses every merge commit among them.
//!
//! It runs where the history is made: in the master (its push to the public remote) and in a
//! folder whose `origin` is the master (its push to the master). A plain clone, whose `origin` is
//! no marked master, is not checked. The merge commits from before the decision stay: everything
//! under [`SETTLED`] is left out.

use std::path::Path;

use super::{Check, Context, Finding, unpushed};
use crate::location::{self, Role};
use crate::repo::Repo;

/// The tip of `main` when the decision was taken (ADR-0041). The merge commits up to it stay in
/// the history; the master still passes when it publishes them.
const SETTLED: &str = "8edb273f271017a4c6438dfc7eb9a60dbe319827";

/// What to do instead of a merge commit; the commit-message check says the same.
pub(crate) const REBASE_INSTEAD: &str = "Combine branches by rebasing onto the target branch \
     (`git rebase <target>` on a copy of the branch, then `git merge --ff-only` on the target), \
     never with a merge commit (ADR-0041).";

pub(crate) struct GitLinear;

impl Check for GitLinear {
    fn id(&self) -> &'static str {
        "GIT-LINEAR"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let repo = context.repo;
        if !is_guarded(repo, |name| std::env::var(name).ok()) {
            return Ok(Vec::new());
        }
        merge_findings(repo, SETTLED)
    }
}

/// Whether the history of this checkout is checked: it is the master (the environment `variable`
/// or its `.env.local` says so), or its `origin` is a folder marked as the master.
fn is_guarded(repo: &Repo, variable: impl Fn(&str) -> Option<String>) -> bool {
    if location::role_of(repo.root(), variable) == Role::Master {
        return true;
    }
    let Ok(url) = repo.git_bytes(&["config", "--get", "remote.origin.url"]) else {
        return false;
    };
    let url = String::from_utf8_lossy(&url).trim().to_owned();
    let origin = repo.root().join(Path::new(&url));
    // The environment names the role of this checkout, not of its origin: only the mark counts.
    !url.is_empty() && origin.is_dir() && location::role_of(&origin, |_| None) == Role::Master
}

/// One finding for each merge commit a push can add, leaving out the commits under `settled` when
/// the repository has that commit.
fn merge_findings(repo: &Repo, settled: &str) -> anyhow::Result<Vec<Finding>> {
    if !unpushed::has_commits(repo) {
        return Ok(Vec::new());
    }
    let known = repo
        .git_bytes(&["cat-file", "-e", &format!("{settled}^{{commit}}")])
        .is_ok();
    let left_out: &[&str] = if known { &[settled] } else { &[] };
    let listing = repo.git_bytes(&unpushed::log_arguments(
        &["--merges", "--format=%h"],
        left_out,
    ))?;
    Ok(String::from_utf8_lossy(&listing)
        .lines()
        .map(str::trim)
        .filter(|hash| !hash.is_empty())
        .map(|hash| {
            Finding::at(
                format!("commit {hash}"),
                format!("is a merge commit. {REBASE_INSTEAD}"),
            )
        })
        .collect())
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
        assert!(is_guarded(&repo, none), "the folder should be checked");
        merge_findings(&repo, SETTLED)
            .unwrap()
            .into_iter()
            .map(|finding| format!("{:?} {}", finding.location, finding.message))
            .collect()
    }

    #[test]
    fn good_example_a_push_on_top_of_a_main_with_merge_commits() {
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
    fn the_master_is_checked_and_its_settled_merge_commits_stay() {
        let master = master("linear-publish", true);
        let repo = Repo::at(master.path().to_path_buf());
        assert!(is_guarded(&repo, none));
        // Nothing is on the public remote yet, so the old merge commit counts ...
        assert_eq!(merge_findings(&repo, SETTLED).unwrap().len(), 1);
        // ... unless it is under the commit where the history became a straight line.
        let tip = String::from_utf8(repo.git_bytes(&["rev-parse", "HEAD"]).unwrap()).unwrap();
        assert!(merge_findings(&repo, tip.trim()).unwrap().is_empty());
        // A new merge commit after it is refused.
        merge_a_branch(&master, "late");
        assert_eq!(merge_findings(&repo, tip.trim()).unwrap().len(), 1);
    }

    #[test]
    fn a_plain_clone_is_not_checked() {
        let upstream = master("linear-plain-upstream", false);
        let plain = clone_of("linear-plain", &upstream);
        let repo = Repo::at(plain.path().to_path_buf());
        assert!(!is_guarded(&repo, none));
        // A repository without a remote is no master either, unless the environment says so.
        let alone = master("linear-alone", false);
        let repo = Repo::at(alone.path().to_path_buf());
        assert!(!is_guarded(&repo, none));
        assert!(is_guarded(&repo, |name: &str| {
            (name == ROLE_VARIABLE).then(|| "master".to_owned())
        }));
    }
}
