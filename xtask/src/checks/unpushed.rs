//! The commits a push can add, chosen in one place for every check that reads them (SEC-PRIVATE,
//! GIT-LINEAR).
//!
//! A commit counts when it is on a local branch, on a tag or on `HEAD`, and on no remote-tracking
//! branch: a new branch, a second local branch and a clone are covered, and a repository without
//! remotes yields its whole history.

use crate::repo::Repo;

/// The revisions of `git log` that select the commits; what follows them is left out as well.
const REVISIONS: [&str; 5] = ["--branches", "--tags", "HEAD", "--not", "--remotes"];

/// Whether the repository has a commit at all: a new repository has no `HEAD` yet, and `git log`
/// would fail on it.
pub(crate) fn has_commits(repo: &Repo) -> bool {
    repo.git_bytes(&["rev-parse", "--verify", "--quiet", "HEAD"])
        .is_ok()
}

/// The arguments of `git log` with `options` over the commits a push can add, without the commits
/// under `left_out` as well (commits that are known to be fine).
pub(crate) fn log_arguments<'a>(options: &[&'a str], left_out: &[&'a str]) -> Vec<&'a str> {
    ["log"]
        .into_iter()
        .chain(options.iter().copied())
        .chain(REVISIONS)
        .chain(left_out.iter().copied())
        .collect()
}
