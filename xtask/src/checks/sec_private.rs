//! SEC-PRIVATE: nothing private about the owner in Git (coding-standards §2).
//!
//! What it looks for, in every file (its name and its text, whatever way the text is encoded; in a
//! file that is no text, the printable runs of bytes, like the tool `strings`), in the names of
//! branches and tags, in every commit message and tag message, and in what the commits that are
//! not pushed yet add:
//!
//! 1. a user-profile path with an account name (`X:\Users\<name>\`, `X:/Users/<name>/`,
//!    `X:\Documents and Settings\<name>`, a share like `\\host\c$\Users\<name>` or
//!    `\\wsl$\<distro>\home\<name>`, `\Users\<name>`, `/c/Users/<name>/`, `/home/<name>`,
//!    `/var/home/<name>`, `/Users/<name>`, also URL-encoded); `%USERPROFILE%` and `~` are the
//!    allowed forms, and so are the placeholders and made-up names of [`GENERIC_ACCOUNTS`]. The
//!    account of the machine the check runs on is never generic, however short;
//! 2. a Claude session ID or link: the memory-note key with a value (any case, quoted or not, after
//!    `:` or `=`), the keys `sessionId`, `session_id` and `session-id` with a UUID, a session link
//!    with an ID, and `session_` followed by a long ID (letters and digits, 20 or more; any case);
//!    in a commit message also any session trailer line;
//! 3. a link to a published page (an artifact on claude.ai) with an ID: the owner keeps those
//!    links outside the repository;
//! 4. a private word: the values of `USERNAME`, `COMPUTERNAME`, `USER` and `HOSTNAME` of the
//!    machine the check runs on, and the entries of the optional list
//!    `~/.config/bugaboo/private-words.txt` (the variable `BUGABOO_PRIVATE_WORDS` names another
//!    file; one entry per line; blank lines and `#` lines ignored; the list lives outside the
//!    repository). An entry is a word, several words (they match with spaces, `_`, `-` or `.`
//!    between them) or an e-mail address (an entry with `@`). A word matches in any case, and ends
//!    where a letter ends: `_`, digits and punctuation do not continue it (`name_notes` and
//!    `name42` match, `names` does not). A private word inside a GitHub form is allowed: the
//!    login in `github.com/<login>` and the login in a noreply address
//!    (`<id>+<login>@users.noreply.github.com`), because those are public by nature. For the
//!    words of the variables a value of fewer than [`MIN_NAME_LEN`] characters or a generic word
//!    is not used, because it would match ordinary text; the rule on paths still knows them;
//! 5. an author or committer that is not a public identity (see [`identity`]): the commit about to
//!    be recorded (commit-msg hook) and every commit that is on no remote yet (push and the full
//!    check);
//! 6. the session key of Claude Code in a memory note, with or without a value.
//!
//! The check never prints a private word: a finding names the place (file and line, or commit and
//! line) and the rule only, and a name that holds a private word, an account or an ID is shown with
//! that part replaced by `***`. Files in `tools/checks/private-exemptions.toml` are exempt from the
//! rule on private words in their text (not from the others), each with a reason: they are data
//! the project does not write. The failing examples of this check are the one place it does not
//! look at.
//!
//! The words of a machine come from [`PrivateWords`], which tests build by hand, so no example
//! depends on the real machine.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde::Deserialize;

use super::exemptions::{self, Exemption};
use super::git_commit_msg;
use super::meta_wiring::CLAUDE_FOLDER;
use super::{Check, Context, Finding};
use crate::repo::Repo;
use crate::source::{FIXTURES_FOLDER, decode_text, is_word_char, printable_runs};

mod history;
mod identity;
mod memory;
mod patterns;
mod words;

use history::scan_history;
pub(crate) use identity::Identity;
pub(crate) use identity::recorded_problems as recorded_identity_problems;
pub(crate) use memory::strip_generated_keys;
use patterns::{Origin, PrivateWordRule, masked, scan_text};
use words::Pattern;

/// A part of a text: byte positions from and to.
type Span = (usize, usize);

/// The key Claude Code adds to every memory note it saves; `cargo xtask hook post-edit` removes
/// the line again.
pub(crate) const MEMORY_SESSION_KEY: &str = "originSessionId";
/// Other keys that hold a session ID, which then has the form of a UUID.
const UUID_SESSION_KEYS: [&str; 3] = ["sessionId", "session_id", "session-id"];
/// The folder of the memory notes, below [`CLAUDE_FOLDER`].
const MEMORY_FOLDER: &str = "memory";

const SESSION_LINK: &str = "claude.ai/code/session_";
/// The links to a published page; an ID follows.
const PAGE_LINKS: [&str; 4] = [
    "claude.ai/artifact/",
    "claude.ai/code/artifact/",
    "claude.ai/public/artifacts/",
    "claude.site/artifacts/",
];
const SESSION_PREFIX: &str = "session_";
/// The trailer Claude Code can ask for at the end of a commit message, in lower case.
const COMMIT_TRAILER: &str = "claude-session";
/// The shortest ID after the key or the link.
const MIN_ID_LEN: usize = 8;
/// The shortest run of letters and digits after `session_` that counts as an ID.
const LONG_ID_LEN: usize = 20;
/// The lengths of the groups of a UUID.
const UUID_GROUPS: [usize; 5] = [8, 4, 4, 4, 12];
/// What replaces the private part of a name in a finding.
const MASK: &str = "***";

/// The folders of a profile, in lower case, and the spelling a bare path has to have.
const PROFILE_FOLDERS: [(&str, &str); 3] = [
    ("users", "Users"),
    ("home", "home"),
    ("documents and settings", "Documents and Settings"),
];

/// The variables that name the account and the computer.
const NAME_VARIABLES: [&str; 4] = ["USERNAME", "COMPUTERNAME", "USER", "HOSTNAME"];
/// A value shorter than this would match ordinary words.
const MIN_NAME_LEN: usize = 4;
/// Accounts in a path that name no one (CI machines, shared folders, placeholders) and the
/// made-up names of test data, in lower case. A value of a variable that is one of these is not a
/// private word either.
const GENERIC_ACCOUNTS: [&str; 24] = [
    "ana",
    "bo",
    "a",
    "x",
    "public",
    "default",
    "shared",
    "all",
    "runner",
    "runneradmin",
    "user",
    "username",
    "name",
    "you",
    "me",
    "example",
    "someone",
    "ubuntu",
    "vagrant",
    "root",
    "guest",
    "admin",
    "administrator",
    "nobody",
];
/// More values of the variables that name no one, in lower case.
const GENERIC_MACHINE_NAMES: [&str; 7] = [
    "users",
    "localhost",
    "desktop",
    "laptop",
    "computer",
    "system",
    "docker",
];
/// The list of private words, below the home folder.
const WORDS_FILE: &str = ".config/bugaboo/private-words.txt";
/// The variable that names another file for the list.
const WORDS_VARIABLE: &str = "BUGABOO_PRIVATE_WORDS";
const EXEMPTIONS_PATH: &str = "tools/checks/private-exemptions.toml";
/// The characters between the parts of a path.
const SEPARATORS: [char; 2] = ['/', '\\'];
/// The quotes around a key or a value.
const QUOTES: [char; 2] = ['"', '\''];

/// Which rule a line broke.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum Rule {
    ProfilePath,
    SessionId,
    SessionTrailer,
    PageLink,
    PrivateWord,
    Identity,
    MemorySessionKey,
}

impl Rule {
    /// What is wrong and what to do, without ever quoting the text.
    fn message(self) -> &'static str {
        match self {
            Self::ProfilePath => {
                "has a user-profile path with an account name. Write `%USERPROFILE%` or `~` instead."
            }
            Self::SessionId => {
                "has a Claude session ID or link. Remove it; it identifies the owner's sessions."
            }
            Self::SessionTrailer => {
                "has a session trailer. A commit ends with the Co-Authored-By line only."
            }
            Self::PageLink => {
                "has a link to a published page. Keep it in the owner's list outside Git and \
                 name the page instead."
            }
            Self::Identity => {
                "is not a public identity. Use a name with the noreply address of its GitHub \
                 account (`Name <id+Name@users.noreply.github.com>`): set `user.name` and \
                 `user.email`; `cargo xtask setup` copies them from the master."
            }
            Self::MemorySessionKey => {
                "has the session key of Claude Code in a memory note. Remove the line; \
                 `cargo xtask hook post-edit` removes it after an edit."
            }
            Self::PrivateWord => {
                "has a private word (the account or computer name of this machine, or a word of \
                 the private list). Use a made-up name instead."
            }
        }
    }
}

fn is_generic(value: &str) -> bool {
    GENERIC_ACCOUNTS.contains(&value) || GENERIC_MACHINE_NAMES.contains(&value)
}

/// The words that must not appear, in lower case.
#[derive(Default)]
pub(crate) struct PrivateWords {
    /// The words to look for in text, in lower case.
    words: BTreeSet<String>,
    /// The same words, ready to be found.
    patterns: Vec<Pattern>,
    /// The names of the machine and the words of the list, whatever their length: an account
    /// among them is never taken for a placeholder in a path.
    own: BTreeSet<String>,
}

impl PrivateWords {
    /// The words of a machine: `variable` answers for an environment variable, `list` is the text
    /// of the private list if there is one.
    pub(crate) fn from_sources(
        variable: impl Fn(&str) -> Option<String>,
        list: Option<&str>,
    ) -> Self {
        let mut found = Self::default();
        for name in NAME_VARIABLES {
            let Some(value) = variable(name) else {
                continue;
            };
            let value = value.trim().to_lowercase();
            if value.is_empty() {
                continue;
            }
            if value.chars().count() >= MIN_NAME_LEN && !is_generic(&value) {
                found.words.insert(value.clone());
            }
            found.own.insert(value);
        }
        for line in list.unwrap_or_default().lines() {
            let line = line.trim().to_lowercase();
            if !line.is_empty() && !line.starts_with('#') {
                found.own.insert(line.clone());
                found.words.insert(line);
            }
        }
        found.patterns = found.words.iter().map(|word| Pattern::new(word)).collect();
        found
    }

    /// The words of the machine this runs on, and what is wrong with the list if it exists but
    /// cannot be used (the text never holds a word or the home folder).
    pub(crate) fn from_machine() -> (Self, Option<String>) {
        let home = ["USERPROFILE", "HOME"]
            .iter()
            .find_map(|name| std::env::var(name).ok());
        Self::from_home(home.as_deref().map(Path::new), |name| {
            std::env::var(name).ok()
        })
    }

    fn from_home(
        home: Option<&Path>,
        variable: impl Fn(&str) -> Option<String>,
    ) -> (Self, Option<String>) {
        // A list that is not there is fine; one that is there and cannot be used is a problem. A
        // file that the variable names must be there: a list that is silently missing guards
        // nothing.
        let named = variable(WORDS_VARIABLE).filter(|path| !path.trim().is_empty());
        let list = match &named {
            Some(path) => Some(fs::read(path.trim())),
            None => home.map(|home| fs::read(home.join(WORDS_FILE))),
        };
        let (list, unusable) = match list {
            Some(Ok(bytes)) => {
                let text = decode_text(&bytes);
                let unusable = text.is_none();
                (text, unusable)
            }
            Some(Err(error)) => (None, named.is_some() || error.kind() != ErrorKind::NotFound),
            None => (None, false),
        };
        let problem = unusable.then(|| {
            format!(
                "The list of private words (`~/{WORDS_FILE}`, or the file that `{WORDS_VARIABLE}` names) cannot be read as text, \
                 so it was not used. Fix or remove the file."
            )
        });
        (Self::from_sources(variable, list.as_deref()), problem)
    }

    /// Where the words stand in the text, in any case, as whole words: a word ends where a
    /// letter ends. A word inside a GitHub form is public and not found.
    fn word_spans(&self, text: &str) -> Vec<Span> {
        // The text in lower case, each character with the place of the character it came from.
        let mut lowered: Vec<words::Lowered> = Vec::new();
        let mut positions = text.char_indices().peekable();
        while let Some((start, c)) = positions.next() {
            let end = positions.peek().map_or(text.len(), |next| next.0);
            lowered.extend(c.to_lowercase().map(|lower| (lower, start, end)));
        }
        let public = words::github_forms(text);
        self.patterns
            .iter()
            .flat_map(|pattern| pattern.spans(&lowered))
            .filter(|(from, to)| !public.iter().any(|(a, b)| a <= from && to <= b))
            .collect()
    }

    /// Whether this account is the machine's own or on the private list.
    fn is_own(&self, account: &str) -> bool {
        self.own.contains(account)
    }
}

#[derive(Deserialize)]
struct Exemptions {
    #[serde(default)]
    exempt: Vec<Exempt>,
}

/// A file that is exempt from the rule on private words.
#[derive(Deserialize)]
struct Exempt {
    path: String,
    reason: String,
}

impl Exemption for Exempt {
    fn name(&self) -> String {
        self.path.clone()
    }

    fn reason(&self) -> &str {
        &self.reason
    }
}

/// Whether the file may hold private words in its text.
fn is_exempt(exempt: &[Exempt], file: &str) -> bool {
    exempt.iter().any(|entry| entry.path == file)
}

pub(crate) struct SecPrivate;

impl SecPrivate {
    const ID: &'static str = "SEC-PRIVATE";

    /// The failing examples of this check hold what it looks for, on purpose: they are the only
    /// files it does not read.
    fn failing_examples() -> String {
        format!("{FIXTURES_FOLDER}/{}/bad/", Self::ID)
    }
}

impl Check for SecPrivate {
    fn id(&self) -> &'static str {
        Self::ID
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let (words, list_problem) = PrivateWords::from_machine();
        let repo = context.repo;
        let exempt: Exemptions = exemptions::load(repo, EXEMPTIONS_PATH)?;
        let mut findings: Vec<Finding> = list_problem.into_iter().map(Finding::general).collect();
        // Whether an exemption is still needed depends on the machine (a name may or may not be in
        // the file), so the file is judged on its reasons and on the file it names only.
        findings.extend(exemptions::reason_findings(EXEMPTIONS_PATH, &exempt.exempt));
        findings.extend(
            exempt
                .exempt
                .iter()
                .filter(|entry| !repo.path(&entry.path).is_file())
                .map(|entry| {
                    Finding::at(
                        EXEMPTIONS_PATH,
                        format!("The entry `{}` names no file. Remove it.", entry.path),
                    )
                }),
        );
        // Every file counts, the examples of other checks and the lock file too.
        let mut files = match context.files_for(self.id())? {
            Some(files) => files,
            None => repo.all_files()?,
        };
        let skipped = Self::failing_examples();
        files.retain(|file| !file.starts_with(&skipped));
        findings.extend(scan_files(repo.root(), &files, &words, &exempt.exempt));
        // The history belongs to a run on the whole repository; a staged run looks at files only
        // (the commit-msg hook checks the message being written).
        if context.files.is_none() {
            findings.extend(scan_history(repo, &words, &exempt.exempt, &skipped)?);
            findings.extend(identity::unpushed_findings(repo)?);
        }
        Ok(findings)
    }
}

fn finding(place: &str, line: usize, rule: Rule) -> Finding {
    Finding::at(format!("{place}:{line}"), rule.message())
}

/// One finding for each rule in the set, at a place that has no line.
fn findings_without_line(place: &str, rules: BTreeSet<Rule>) -> impl Iterator<Item = Finding> {
    let place = place.to_owned();
    rules
        .into_iter()
        .map(move |rule| Finding::at(place.clone(), rule.message()))
}

/// The rules a text breaks, without the lines.
fn rules_in(text: &str, words: &PrivateWords, private_words: PrivateWordRule) -> BTreeSet<Rule> {
    scan_text(text, words, Origin::Content, private_words)
        .into_iter()
        .map(|(_, rule)| rule)
        .collect()
}

/// The findings when the name of a file holds something private. `place` says which file it is.
fn scan_name(place: &str, name: &str, words: &PrivateWords) -> Vec<Finding> {
    findings_without_line(
        &format!("{place} (its name)"),
        rules_in(name, words, PrivateWordRule::Applies),
    )
    .collect()
}

/// The findings in the bytes of a file: its text, or the printable runs of a file that is no text
/// (then without lines). `private_words` says whether the rule on private words applies.
fn scan_content(
    place: &str,
    bytes: &[u8],
    words: &PrivateWords,
    private_words: PrivateWordRule,
) -> Vec<Finding> {
    if let Some(text) = decode_text(bytes) {
        scan_text(&text, words, Origin::Content, private_words)
            .into_iter()
            .map(|(line, rule)| finding(place, line, rule))
            .collect()
    } else {
        findings_without_line(
            &format!("{place} (a file that is no text)"),
            rules_in(&printable_runs(bytes), words, private_words),
        )
        .collect()
    }
}

/// The names of the files, and their text (or the printable runs of a file that is no text).
fn scan_files(
    root: &Path,
    files: &[String],
    words: &PrivateWords,
    exempt: &[Exempt],
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for file in files {
        let shown = masked(file, words);
        findings.extend(scan_name(&shown, file, words));
        if let Ok(bytes) = fs::read(root.join(file)) {
            if memory::is_memory_note(Path::new(file)) {
                findings.extend(
                    memory::session_key_lines(&decode_text(&bytes).unwrap_or_default())
                        .into_iter()
                        .map(|line| finding(&shown, line, Rule::MemorySessionKey)),
                );
            }
            findings.extend(scan_content(
                &shown,
                &bytes,
                words,
                PrivateWordRule::of_file(is_exempt(exempt, file)),
            ));
        }
    }
    findings
}

/// What is private in a commit message file written by Git (the diff of `git commit -v` is not
/// part of the message; comment lines are read too, whatever character Git is set to use).
pub(crate) fn message_problems(file: &Path) -> anyhow::Result<Vec<String>> {
    let (words, list_problem) = PrivateWords::from_machine();
    let bytes = fs::read(file)?;
    let text = decode_text(&bytes).unwrap_or_default();
    let message = git_commit_msg::before_scissors(&text);
    let mut problems: Vec<String> = list_problem.into_iter().collect();
    problems.extend(
        scan_text(
            message,
            &words,
            Origin::CommitMessage,
            PrivateWordRule::Applies,
        )
        .into_iter()
        .map(|(line, rule)| format!("The message, line {line}, {}", rule.message())),
    );
    Ok(problems)
}

#[cfg(test)]
mod tests;
