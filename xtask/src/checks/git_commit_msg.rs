//! GIT-COMMIT-MSG: commit messages follow Conventional Commits (coding-standards §2).
//!
//! `cargo xtask check commit-msg <file>` is what the Git `commit-msg` hook calls. As a catalogue
//! check it looks at the messages of every commit that is on this branch but not on `main`.

use std::fs;
use std::path::Path;

use anyhow::Context as _;

use super::{Check, Context, Finding};
use crate::repo::Repo;

/// The commit types the project uses (coding-standards §2).
const TYPES: [&str; 10] = [
    "feat", "fix", "docs", "refactor", "test", "build", "ci", "chore", "perf", "style",
];

/// Subjects Git itself writes (merges, reverts, autosquash markers) are accepted as they are.
const GIT_OWN: [&str; 5] = ["Merge ", "Revert \"", "fixup! ", "squash! ", "amend! "];

/// The longest allowed subject line.
const MAX_SUBJECT: usize = 72;

pub(crate) struct GitCommitMsg;

impl Check for GitCommitMsg {
    fn id(&self) -> &'static str {
        "GIT-COMMIT-MSG"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let mut findings = Vec::new();
        for (short_hash, subject) in branch_subjects(context.repo) {
            for problem in problems(&subject) {
                findings.push(Finding::at(short_hash.clone(), problem));
            }
        }
        Ok(findings)
    }
}

/// `(short hash, subject)` of the commits that are not on `main` (or only the last one).
fn branch_subjects(repo: &Repo) -> Vec<(String, String)> {
    let has_main = repo
        .git_bytes(&["rev-parse", "--verify", "--quiet", "main"])
        .is_ok();
    let range = if has_main { "main..HEAD" } else { "HEAD" };
    let mut args = vec!["log", "--no-merges", "--format=%h%x09%s", range];
    if !has_main {
        args.insert(1, "-n1");
    }
    let Ok(text) = repo.git_bytes(&args) else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&text);
    text.lines()
        .filter_map(|line| line.split_once('\t'))
        .map(|(hash, subject)| (hash.to_owned(), subject.to_owned()))
        .collect()
}

/// Checks the message in a file written by Git; returns the problems, empty when it is fine.
pub(crate) fn check_file(file: &Path) -> anyhow::Result<Vec<String>> {
    let text = fs::read_to_string(file)
        .with_context(|| format!("could not read the commit message file {}", file.display()))?;
    Ok(problems(subject_of(&text)))
}

/// What Git writes after the comment character to mark where the diff of `git commit -v` starts;
/// it and what follows are not the message.
const SCISSORS: &str = " ------------------------ >8 ------------------------";

/// The text of a message file up to the scissors line. Comment lines stay: a hook that reads the
/// message must not depend on the comment character Git is set to use, so any one character
/// before the scissors counts as the comment character.
pub(crate) fn before_scissors(text: &str) -> &str {
    let mut start = 0;
    for line in text.split_inclusive('\n') {
        let mut chars = line.trim_end().chars();
        let comment = chars.next().is_some_and(|c| !c.is_alphanumeric());
        if comment && chars.as_str() == SCISSORS {
            return &text[..start];
        }
        start += line.len();
    }
    text
}

/// The first line that is not a comment (Git adds `#` lines to the message file).
fn subject_of(message: &str) -> &str {
    message
        .lines()
        .find(|line| !line.starts_with('#') && !line.trim().is_empty())
        .unwrap_or("")
        .trim_end()
}

/// What is wrong with a subject line, in plain words. Empty means it is fine.
pub(crate) fn problems(subject: &str) -> Vec<String> {
    if GIT_OWN.iter().any(|prefix| subject.starts_with(prefix)) {
        return Vec::new();
    }
    let form =
        "Write the first line as `type(scope): subject`, for example `feat(xtask): add setup`.";
    let Some((head, text)) = subject.split_once(": ") else {
        return vec![format!(
            "The first line `{subject}` is not a Conventional Commit. {form}"
        )];
    };
    let mut found = Vec::new();
    let (kind, scope) = match head.split_once('(') {
        Some((kind, rest)) => (kind, Some(rest)),
        None => (head, None),
    };
    let kind = kind.strip_suffix('!').unwrap_or(kind);
    if !TYPES.contains(&kind) {
        found.push(format!(
            "`{kind}` is not a commit type. Use one of: {}.",
            TYPES.join(", ")
        ));
    }
    if let Some(rest) = scope {
        let scope = rest.strip_suffix(')').or_else(|| rest.strip_suffix(")!"));
        if scope.is_none_or(|scope| scope.trim().is_empty()) {
            found.push(format!("The scope in `{head}` must look like `(name)`."));
        }
    }
    if text.trim().is_empty() {
        found.push("The subject after the colon is empty. Say what the commit does.".to_owned());
    }
    let length = subject.chars().count();
    if length > MAX_SUBJECT {
        found.push(format!(
            "The first line has {length} characters; keep it to {MAX_SUBJECT} or fewer."
        ));
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn good_example() {
        for subject in [
            "feat(xtask): workspace, strict lints and check runner (M0 WP-01)",
            "docs: complete design phase",
            "fix(web)!: drop the old route",
            "chore: bump",
        ] {
            assert!(
                problems(subject).is_empty(),
                "{subject}: {:?}",
                problems(subject)
            );
        }
    }

    #[test]
    fn bad_example() {
        for subject in [
            "added some stuff",
            "feature(x): nope",
            "feat(): empty scope",
            "feat(x):no space",
            "feat: ",
            &format!("feat: {}", "x".repeat(80)),
            "",
        ] {
            assert!(!problems(subject).is_empty(), "should fail: {subject:?}");
        }
    }

    #[test]
    fn git_own_messages_are_accepted() {
        for subject in [
            "Merge branch 'WIN3' into main",
            "Merge pull request #4 from x/y",
            "Revert \"feat(web): add board\"",
            "fixup! feat(xtask): add setup",
            "squash! fix(web): x",
            "amend! docs: y",
        ] {
            assert!(problems(subject).is_empty(), "{subject}");
        }
        assert!(!problems("Merged stuff").is_empty());
        assert!(!problems("Reverting things").is_empty());
    }

    #[test]
    fn comment_lines_are_skipped() {
        assert_eq!(subject_of("# Please enter\nfix: a\n\nbody"), "fix: a");
    }

    #[test]
    fn message_file_is_checked() {
        let temp = crate::testutil::TempDir::new("commit-msg");
        let bad = temp.write("bad", "wip\n");
        let good = temp.write("good", "fix(web): x\n\nCo-Authored-By: someone\n");
        assert!(!check_file(&bad).unwrap().is_empty());
        assert!(check_file(&good).unwrap().is_empty());
    }
}
