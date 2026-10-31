//! GIT-COMMIT-MSG: commit messages follow Conventional Commits and read as a public history
//! (coding-standards §2, ADR-0038).
//!
//! `cargo xtask check commit-msg <file>` is what the Git `commit-msg` hook calls. As a catalogue
//! check it looks at the messages of every commit that is on this branch but not on `main`.
//!
//! The rules, for the whole message:
//!
//! - the first line is `type(scope): subject` with a type of [`TYPES`], at most [`MAX_SUBJECT`]
//!   characters, a subject that starts in lower case with a verb in the imperative (a first word
//!   that ends in `ed` or `ing` is refused, a few exceptions are known) and no full stop at the
//!   end; a merge is `Merge <what it brought>`;
//! - the only trailer is `Co-Authored-By: Claude <model> <noreply@anthropic.com>`;
//! - no link to claude.ai (the lines of a session are refused by SEC-PRIVATE as well);
//! - no jargon of the working process (see [`jargon`]).

use std::fs;
use std::path::Path;

use anyhow::Context as _;

use super::{Check, Context, Finding};
use crate::repo::Repo;

mod jargon;
#[cfg(test)]
mod tests;

/// The commit types the project uses (coding-standards §2).
const TYPES: [&str; 11] = [
    "feat", "fix", "perf", "refactor", "test", "docs", "build", "ci", "chore", "style", "revert",
];

/// Subjects Git itself writes (reverts, autosquash markers) are accepted as they are.
const GIT_OWN: [&str; 4] = ["Revert \"", "fixup! ", "squash! ", "amend! "];

/// A merge commit is `Merge <what it brought>`.
const MERGE: &str = "Merge ";

/// The longest allowed subject line.
const MAX_SUBJECT: usize = 72;

/// The one trailer a message may end with: the start of the line and the end of the address.
const CO_AUTHOR_KEY: &str = "co-authored-by:";
const CO_AUTHOR_FORM: &str = "Co-Authored-By: Claude <model> <noreply@anthropic.com>";
const CO_AUTHOR_ADDRESS: &str = "<noreply@anthropic.com>";

/// First words that end in `ed` or `ing` and are verbs in the imperative anyway.
const IMPERATIVE_EXCEPTIONS: [&str; 16] = [
    "embed", "speed", "need", "feed", "seed", "proceed", "exceed", "succeed", "bleed", "breed",
    "shred", "bring", "string", "spring", "swing", "cling",
];

pub(crate) struct GitCommitMsg;

impl Check for GitCommitMsg {
    fn id(&self) -> &'static str {
        "GIT-COMMIT-MSG"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let mut findings = Vec::new();
        for (short_hash, message) in branch_messages(context.repo) {
            for problem in problems(&message) {
                findings.push(Finding::at(short_hash.clone(), problem));
            }
        }
        Ok(findings)
    }
}

/// `(short hash, message)` of the commits that are not on `main` (or only the last one), merges
/// included.
fn branch_messages(repo: &Repo) -> Vec<(String, String)> {
    let has_main = repo
        .git_bytes(&["rev-parse", "--verify", "--quiet", "main"])
        .is_ok();
    let range = if has_main { "main..HEAD" } else { "HEAD" };
    let mut args = vec!["log", "--format=%h%x1f%B%x1e", range];
    if !has_main {
        args.insert(1, "-n1");
    }
    let Ok(text) = repo.git_bytes(&args) else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&text);
    text.split('\u{1e}')
        .filter_map(|record| record.trim_start().split_once('\u{1f}'))
        .map(|(hash, message)| (hash.to_owned(), message.to_owned()))
        .collect()
}

/// Checks the message in a file written by Git; returns the problems, empty when it is fine.
pub(crate) fn check_file(file: &Path) -> anyhow::Result<Vec<String>> {
    let text = fs::read_to_string(file)
        .with_context(|| format!("could not read the commit message file {}", file.display()))?;
    Ok(problems(before_scissors(&text)))
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

/// The lines of a message without the comment lines Git adds (`#` lines).
fn lines_of(message: &str) -> Vec<&str> {
    message
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(str::trim_end)
        .collect()
}

/// The first line that is not a comment.
fn subject_of(message: &str) -> &str {
    lines_of(message)
        .into_iter()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
}

/// What is wrong with a message, in plain words. Empty means it is fine.
pub(crate) fn problems(message: &str) -> Vec<String> {
    let mut found = subject_problems(subject_of(message));
    let lines = lines_of(message);
    found.extend(trailer_problems(&lines));
    if message.to_lowercase().contains("claude.ai") {
        found.push("The message has a link to claude.ai. Remove it.".to_owned());
    }
    let text = lines.join("\n");
    found.extend(jargon::problems(&text));
    found
}

/// Whether a first word is past tense or a gerund (`added`, `adding`), which an imperative is not.
fn is_not_imperative(word: &str) -> bool {
    let lower = word.to_lowercase();
    let long = lower.chars().count() >= 5;
    long && (lower.ends_with("ed") || lower.ends_with("ing"))
        && !IMPERATIVE_EXCEPTIONS.contains(&lower.as_str())
}

/// What is wrong with the first line.
fn subject_problems(subject: &str) -> Vec<String> {
    if GIT_OWN.iter().any(|prefix| subject.starts_with(prefix)) {
        return Vec::new();
    }
    if let Some(what) = subject
        .strip_prefix(MERGE)
        .or_else(|| (subject == MERGE.trim_end()).then_some(""))
    {
        let empty =
            "Write a merge as `Merge <what it brought>`, for example `Merge the search page`.";
        return if what.trim().is_empty() {
            vec![empty.to_owned()]
        } else {
            Vec::new()
        };
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
    } else {
        found.extend(subject_text_problems(text));
    }
    let length = subject.chars().count();
    if length > MAX_SUBJECT {
        found.push(format!(
            "The first line has {length} characters; keep it to {MAX_SUBJECT} or fewer."
        ));
    }
    found
}

/// What is wrong with the text after `type(scope): `.
fn subject_text_problems(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    if text.chars().next().is_some_and(char::is_uppercase) {
        found.push("The subject starts in lower case, like `add setup`.".to_owned());
    }
    if let Some(word) = text.split_whitespace().next()
        && is_not_imperative(word)
    {
        found.push(format!(
            "The subject starts with `{word}`. Start with a verb in the imperative, like `add` \
             or `fix`, not `added` or `adding`."
        ));
    }
    if text.ends_with('.') {
        found.push("The subject has no full stop at the end.".to_owned());
    }
    found
}

/// What is wrong with the trailers: the lines of the last paragraph that look like `Some-Key: x`,
/// and every `Co-Authored-By` line wherever it stands. The only one allowed is
/// `Co-Authored-By: Claude <model> <noreply@anthropic.com>`.
fn trailer_problems(lines: &[&str]) -> Vec<String> {
    let last = lines
        .rsplit(|line| line.trim().is_empty())
        .find(|paragraph| !paragraph.is_empty())
        .unwrap_or(&[]);
    let mut found = Vec::new();
    for line in lines {
        let lower = line.to_lowercase();
        let is_author_line = lower.starts_with(CO_AUTHOR_KEY);
        let is_trailer = last.contains(line) && looks_like_trailer(line);
        if !is_author_line && !is_trailer {
            continue;
        }
        let value = line.split_once(':').map_or("", |(_, value)| value.trim());
        let is_claude = is_author_line
            && value.starts_with("Claude ")
            && value.ends_with(CO_AUTHOR_ADDRESS)
            && value.len() > "Claude ".len() + CO_AUTHOR_ADDRESS.len();
        if !is_claude {
            let key = line.split_once(':').map_or(*line, |(key, _)| key);
            found.push(format!(
                "The line `{key}:` is not allowed. The only trailer is `{CO_AUTHOR_FORM}`."
            ));
        }
    }
    found
}

/// Whether a line reads as a trailer: a key of hyphenated words, a colon and a space.
fn looks_like_trailer(line: &str) -> bool {
    let Some((key, _)) = line.split_once(": ") else {
        return false;
    };
    key.contains('-')
        && key
            .split('-')
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_alphanumeric()))
}
