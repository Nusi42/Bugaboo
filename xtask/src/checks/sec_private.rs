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
//! 3. a private word: the values of `USERNAME`, `COMPUTERNAME`, `USER` and `HOSTNAME` of the
//!    machine the check runs on, and the words of the optional list
//!    `~/.config/bugaboo/private-words.txt` (one per line; blank lines and `#` lines ignored; the
//!    list lives outside the repository). A word matches in any case, and ends where a letter
//!    ends: `_`, digits and punctuation do not continue it (`name_notes` and `name42` match,
//!    `names` does not). For the words of the variables a value of fewer than [`MIN_NAME_LEN`]
//!    characters or a generic word is not used, because it would match ordinary text; the rule on
//!    paths still knows them.
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
    PrivateWord,
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
    /// The words to look for in text.
    words: BTreeSet<String>,
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
        // A list that is not there is fine; one that is there and cannot be used is a problem.
        let list = home.map(|home| fs::read(home.join(WORDS_FILE)));
        let (list, unusable) = match list {
            Some(Ok(bytes)) => {
                let text = decode_text(&bytes);
                let unusable = text.is_none();
                (text, unusable)
            }
            Some(Err(error)) => (None, error.kind() != ErrorKind::NotFound),
            None => (None, false),
        };
        let problem = unusable.then(|| {
            format!(
                "The list of private words (`~/{WORDS_FILE}`) exists but cannot be read as text, \
                 so it was not used. Fix or remove the file."
            )
        });
        (Self::from_sources(variable, list.as_deref()), problem)
    }

    /// Where the words stand in the text, in any case, as whole words: a word ends where a
    /// letter ends.
    fn word_spans(&self, text: &str) -> Vec<Span> {
        // The text in lower case, each character with the place of the character it came from.
        let mut lowered: Vec<(char, usize, usize)> = Vec::new();
        let mut positions = text.char_indices().peekable();
        while let Some((start, c)) = positions.next() {
            let end = positions.peek().map_or(text.len(), |next| next.0);
            lowered.extend(c.to_lowercase().map(|lower| (lower, start, end)));
        }
        let mut spans = Vec::new();
        for word in &self.words {
            let word: Vec<char> = word.chars().collect();
            for from in 0..lowered.len().saturating_sub(word.len().saturating_sub(1)) {
                let found = &lowered[from..from + word.len()];
                if word.is_empty() || found.iter().zip(&word).any(|(a, b)| a.0 != *b) {
                    continue;
                }
                let before = from.checked_sub(1).map(|at| lowered[at].0);
                let after = lowered.get(from + word.len()).map(|next| next.0);
                if !before.is_some_and(|c| c.is_ascii_alphabetic())
                    && !after.is_some_and(|c| c.is_ascii_alphabetic())
                {
                    spans.push((found[0].1, found[word.len() - 1].2));
                }
            }
        }
        spans
    }

    /// Whether this account is the machine's own or on the private list.
    fn is_own(&self, account: &str) -> bool {
        self.own.contains(account)
    }
}

/// The place of the name of an account when the text `rest` (after `Users` or `home`) is a
/// separator run followed by an account that is not a placeholder.
fn account_follows(rest: &str, words: &PrivateWords) -> Option<Span> {
    let after_separators = rest.trim_start_matches(SEPARATORS);
    if after_separators.len() == rest.len() {
        return None;
    }
    let skip = rest.len() - after_separators.len();
    let name: String = after_separators
        .chars()
        .take_while(|c| c.is_alphanumeric() || matches!(c, '_' | '.' | '-'))
        .collect();
    let lower = name.to_lowercase();
    (name.chars().next().is_some_and(char::is_alphanumeric)
        && (!GENERIC_ACCOUNTS.contains(&lower.as_str()) || words.is_own(&lower)))
    .then_some((skip, skip + name.len()))
}

/// The place of the account when the folder at `at` in `line` starts a profile path.
fn profile_path_at(
    line: &str,
    at: usize,
    keyword: &str,
    exact: &str,
    words: &PrivateWords,
) -> Option<Span> {
    let before = &line[..at];
    let prefix = before.trim_end_matches(SEPARATORS);
    if prefix.len() == before.len() {
        return None;
    }
    let start = at + keyword.len();
    let (from, to) = account_follows(&line[start..], words)?;
    let drive = prefix
        .strip_suffix(':')
        .and_then(|p| p.chars().next_back())
        .is_some_and(|c| c.is_ascii_alphabetic());
    let mounted = prefix
        .strip_suffix(|c: char| c.is_ascii_alphabetic())
        .is_some_and(|p| p.ends_with('/'));
    let admin_share = prefix.ends_with('$');
    // `\\wsl$\Ubuntu\home\…`, `\\host\share\Users\…`: a network path.
    let network = prefix
        .rsplit(|c: char| c.is_whitespace() || QUOTES.contains(&c))
        .next()
        .is_some_and(|token| token.starts_with("\\\\"));
    let var_home = keyword == "home" && prefix.ends_with("/var");
    let alone = prefix.is_empty()
        || prefix.ends_with(|c: char| !is_word_char(c) && !matches!(c, '/' | '.' | '~'));
    // `C:\Users\…`, `/c/Users/…`, `\\host\c$\Users\…`, `\\wsl$\…\home\…`, `/var/home/…` and
    // `\Users\…` are certain. A bare `/Users/…` or `/home/…` has to be the exact word at the start
    // of a path, not the end of a longer one.
    let certain = drive
        || mounted
        || admin_share
        || network
        || var_home
        || (before.ends_with('\\') && alone)
        || (&line[at..start] == exact && before.ends_with('/') && alone);
    certain.then_some((start + from, start + to))
}

/// The text with the characters of a path that URLs write as `%5C`, `%2F` and `%3A` decoded.
fn decode_url_path(line: &str) -> String {
    let mut text = line.to_owned();
    for (code, character) in [("%5c", "\\"), ("%2f", "/"), ("%3a", ":")] {
        while let Some(at) = text.to_ascii_lowercase().find(code) {
            text.replace_range(at..at + code.len(), character);
        }
    }
    text
}

/// Where the accounts of profile paths stand in the text.
fn path_spans(text: &str, words: &PrivateWords) -> Vec<Span> {
    let lower = text.to_ascii_lowercase();
    PROFILE_FOLDERS
        .iter()
        .flat_map(|(keyword, exact)| {
            lower
                .match_indices(keyword)
                .filter_map(|(at, _)| profile_path_at(text, at, keyword, exact, words))
        })
        .collect()
}

/// Whether `line` holds a user-profile path with an account name, also when it is URL-encoded.
fn has_profile_path(line: &str, words: &PrivateWords) -> bool {
    !path_spans(line, words).is_empty() || !path_spans(&decode_url_path(line), words).is_empty()
}

/// Where the values stand that follow `key` and a colon or an equals sign in `line` (the key in any
/// case, with or without quotes, spaces around the sign).
fn value_starts(line: &str, key: &str) -> Vec<usize> {
    let lower = line.to_ascii_lowercase();
    lower
        .match_indices(&key.to_ascii_lowercase())
        .filter_map(|(at, _)| {
            let rest = line[at + key.len()..]
                .trim_start_matches(QUOTES)
                .trim_start()
                .strip_prefix([':', '='])?;
            let value = rest.trim_start().trim_start_matches(QUOTES);
            Some(line.len() - value.len())
        })
        .collect()
}

fn id_len(rest: &str) -> usize {
    rest.chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        .count()
}

/// Whether the text starts with a UUID.
fn starts_with_uuid(text: &str) -> bool {
    let mut groups = text.splitn(UUID_GROUPS.len(), '-');
    UUID_GROUPS.iter().all(|length| {
        groups.next().is_some_and(|group| {
            group.len() >= *length && group[..*length].chars().all(|c| c.is_ascii_hexdigit())
        })
    })
}

/// Where the session IDs stand in the line.
fn session_spans(line: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    for start in value_starts(line, MEMORY_SESSION_KEY) {
        let length = id_len(&line[start..]);
        if length >= MIN_ID_LEN {
            spans.push((start, start + length));
        }
    }
    for key in UUID_SESSION_KEYS {
        for start in value_starts(line, key) {
            if starts_with_uuid(&line[start..]) {
                spans.push((start, start + id_len(&line[start..])));
            }
        }
    }
    let lower = line.to_ascii_lowercase();
    for (at, _) in lower.match_indices(SESSION_LINK) {
        let start = at + SESSION_LINK.len();
        let length = id_len(&line[start..]);
        if length >= MIN_ID_LEN {
            spans.push((start, start + length));
        }
    }
    for (at, _) in lower.match_indices(SESSION_PREFIX) {
        let start = at + SESSION_PREFIX.len();
        let run: String = line[start..]
            .chars()
            .take_while(char::is_ascii_alphanumeric)
            .collect();
        if run.len() >= LONG_ID_LEN
            && run.chars().any(|c| c.is_ascii_digit())
            && run.chars().any(|c| c.is_ascii_alphabetic())
        {
            spans.push((start, start + run.len()));
        }
    }
    spans
}

/// Whether the line is a session trailer (`Claude-Session` and a colon, spaces allowed between;
/// a comment mark in front does not hide it, whatever character Git uses for comments).
fn is_session_trailer(line: &str) -> bool {
    line.trim_start_matches(|c: char| !c.is_alphanumeric())
        .to_ascii_lowercase()
        .strip_prefix(COMMIT_TRAILER)
        .is_some_and(|rest| rest.trim_start().starts_with(':'))
}

/// The rules one line breaks. `in_message` adds the rules of commit messages; `names` says
/// whether the rule on private words applies.
fn rules_of(line: &str, words: &PrivateWords, in_message: bool, names: bool) -> Vec<Rule> {
    let mut rules = Vec::new();
    if has_profile_path(line, words) {
        rules.push(Rule::ProfilePath);
    }
    if !session_spans(line).is_empty() {
        rules.push(Rule::SessionId);
    }
    if in_message && is_session_trailer(line) {
        rules.push(Rule::SessionTrailer);
    }
    if names && !words.word_spans(line).is_empty() {
        rules.push(Rule::PrivateWord);
    }
    rules
}

/// `(line number, rule)` for every rule broken in `text`.
fn scan_text(
    text: &str,
    words: &PrivateWords,
    in_message: bool,
    names: bool,
) -> Vec<(usize, Rule)> {
    text.lines()
        .enumerate()
        .flat_map(|(index, line)| {
            rules_of(line, words, in_message, names)
                .into_iter()
                .map(move |rule| (index + 1, rule))
        })
        .collect()
}

/// A name (of a file, a branch or a tag) as a finding may show it: the private words, accounts
/// and IDs in it replaced by [`MASK`].
fn masked(name: &str, words: &PrivateWords) -> String {
    let decoded = decode_url_path(name);
    let text = if path_spans(name, words).is_empty() && !path_spans(&decoded, words).is_empty() {
        decoded.as_str()
    } else {
        name
    };
    let mut spans = words.word_spans(text);
    spans.extend(session_spans(text));
    spans.extend(path_spans(text, words));
    spans.sort_unstable();
    let mut shown = String::new();
    let mut done = 0;
    for (from, to) in spans {
        if to <= done {
            continue;
        }
        shown.push_str(&text[done..from.max(done)]);
        shown.push_str(MASK);
        done = to;
    }
    shown.push_str(&text[done..]);
    shown
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
fn rules_in(text: &str, words: &PrivateWords, names: bool) -> BTreeSet<Rule> {
    scan_text(text, words, false, names)
        .into_iter()
        .map(|(_, rule)| rule)
        .collect()
}

/// The findings when the name of a file holds something private. `place` says which file it is.
fn scan_name(place: &str, name: &str, words: &PrivateWords) -> Vec<Finding> {
    findings_without_line(&format!("{place} (its name)"), rules_in(name, words, true)).collect()
}

/// The findings in the bytes of a file: its text, or the printable runs of a file that is no text
/// (then without lines). `names` says whether the rule on private words applies.
fn scan_content(place: &str, bytes: &[u8], words: &PrivateWords, names: bool) -> Vec<Finding> {
    if let Some(text) = decode_text(bytes) {
        scan_text(&text, words, false, names)
            .into_iter()
            .map(|(line, rule)| finding(place, line, rule))
            .collect()
    } else {
        findings_without_line(
            &format!("{place} (a file that is no text)"),
            rules_in(&printable_runs(bytes), words, names),
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
            findings.extend(scan_content(
                &shown,
                &bytes,
                words,
                !is_exempt(exempt, file),
            ));
        }
    }
    findings
}

/// The records of a Git listing that ends each record with the separator `0x1e`.
fn records(listing: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(listing)
        .split('\u{1e}')
        .map(|record| record.trim_start().to_owned())
        .filter(|record| !record.is_empty())
        .collect()
}

/// What a push can send: the messages of the commits on every branch, on every tag and on `HEAD`,
/// the messages of annotated tags, the names of branches and tags, and the names and contents of
/// the files that the commits not on a remote add or change.
fn scan_history(
    repo: &Repo,
    words: &PrivateWords,
    exempt: &[Exempt],
    skipped: &str,
) -> anyhow::Result<Vec<Finding>> {
    if repo
        .git_bytes(&["rev-parse", "--verify", "--quiet", "HEAD"])
        .is_err()
    {
        return Ok(Vec::new());
    }
    let commits = repo.git_bytes(&[
        "log",
        "--format=%h%x1f%B%x1e",
        "--branches",
        "--tags",
        "HEAD",
    ])?;
    let tags = repo.git_bytes(&[
        "for-each-ref",
        "--format=%(objecttype)%1f%(refname:short)%1f%(contents)%1e",
        "refs/tags",
    ])?;
    let branches =
        repo.git_bytes(&["for-each-ref", "--format=%(refname:short)%1e", "refs/heads"])?;
    // Each tag as (kind of object, name, message).
    let tags: Vec<(String, String, String)> = records(&tags)
        .iter()
        .map(|record| {
            let mut parts = record.splitn(3, '\u{1f}').map(str::to_owned);
            (
                parts.next().unwrap_or_default(),
                parts.next().unwrap_or_default(),
                parts.next().unwrap_or_default(),
            )
        })
        .collect();
    let mut findings = Vec::new();
    for record in records(&commits) {
        if let Some((hash, message)) = record.split_once('\u{1f}') {
            findings.extend(scan_message(&format!("commit {hash}"), message, words));
        }
    }
    for (kind, name, message) in &tags {
        if kind == "tag" {
            let place = format!("tag {}", masked(name, words));
            findings.extend(scan_message(&place, message, words));
        }
    }
    for (kind, name) in records(&branches)
        .iter()
        .map(|name| ("branch", name.as_str()))
        .chain(tags.iter().map(|(_, name, _)| ("tag", name.as_str())))
    {
        findings.extend(scan_name(
            &format!("{kind} {}", masked(name, words)),
            name,
            words,
        ));
    }
    findings.extend(scan_unpushed_changes(repo, words, exempt, skipped)?);
    Ok(findings)
}

fn scan_message(place: &str, message: &str, words: &PrivateWords) -> Vec<Finding> {
    scan_text(message, words, true, true)
        .into_iter()
        .map(|(line, rule)| finding(place, line, rule))
        .collect()
}

/// How many characters of a commit hash a finding shows.
const SHORT_HASH: usize = 7;

/// A file that a commit adds or changes: the commit, the path and the content (a blob).
struct Change {
    hash: String,
    path: String,
    blob: String,
}

/// The files that the commits add or change on any branch or tag and on `HEAD`, without the
/// commits a remote has already: a new branch, a second local branch and a clone are covered, and
/// a repository without remotes yields its whole history.
fn unpushed_changes(repo: &Repo, skipped: &str) -> anyhow::Result<Vec<Change>> {
    let raw = repo.git_paths_bytes(&[
        "log",
        "--raw",
        "--no-renames",
        "--no-abbrev",
        "--format=%x1e%H",
        "--branches",
        "--tags",
        "HEAD",
        "--not",
        "--remotes",
    ])?;
    let mut changes = Vec::new();
    let mut hash = String::new();
    for line in String::from_utf8_lossy(&raw).lines() {
        if let Some(short) = line.strip_prefix('\u{1e}') {
            hash = short.chars().take(SHORT_HASH).collect();
            continue;
        }
        // `:old-mode new-mode old-blob new-blob status<TAB>path`
        let Some((fields, path)) = line.strip_prefix(':').and_then(|l| l.split_once('\t')) else {
            continue;
        };
        let fields: Vec<&str> = fields.split_whitespace().collect();
        let (Some(mode), Some(blob), Some(status)) = (fields.get(1), fields.get(3), fields.get(4))
        else {
            continue;
        };
        // A deleted file adds nothing, and a submodule is no file of ours.
        if status.starts_with('D') || *mode == "160000" || path.starts_with(skipped) {
            continue;
        }
        changes.push(Change {
            hash: hash.clone(),
            path: path.to_owned(),
            blob: (*blob).to_owned(),
        });
    }
    Ok(changes)
}

/// The contents of the blobs, by blob, read with one `git cat-file --batch`.
fn read_blobs(repo: &Repo, blobs: &BTreeSet<&str>) -> anyhow::Result<BTreeMap<String, Vec<u8>>> {
    let mut found = BTreeMap::new();
    if blobs.is_empty() {
        return Ok(found);
    }
    let input: String = blobs.iter().flat_map(|blob| [*blob, "\n"]).collect();
    let output = repo.git_bytes_with_input(&["cat-file", "--batch"], input.as_bytes())?;
    // Each answer is a line `<blob> <type> <size>`, the content and a line end; `<blob> missing`
    // has no content.
    let mut rest: &[u8] = &output;
    while let Some(end) = rest.iter().position(|byte| *byte == b'\n') {
        let header = String::from_utf8_lossy(&rest[..end]).into_owned();
        rest = &rest[end + 1..];
        let mut parts = header.split(' ');
        let (Some(blob), Some(kind), Some(size)) = (
            parts.next(),
            parts.next(),
            parts.next().and_then(|size| size.parse::<usize>().ok()),
        ) else {
            continue;
        };
        if rest.len() < size {
            break;
        }
        if kind == "blob" {
            found.insert(blob.to_owned(), rest[..size].to_vec());
        }
        rest = rest.get(size + 1..).unwrap_or_default();
    }
    Ok(found)
}

/// The names and contents of the files that the commits not on a remote add or change, through
/// the same reading as the files of the tree. A file that was committed with private data and
/// cleaned or renamed in a later commit is found here.
fn scan_unpushed_changes(
    repo: &Repo,
    words: &PrivateWords,
    exempt: &[Exempt],
    skipped: &str,
) -> anyhow::Result<Vec<Finding>> {
    let changes = unpushed_changes(repo, skipped)?;
    let blobs: BTreeSet<&str> = changes.iter().map(|change| change.blob.as_str()).collect();
    let contents = read_blobs(repo, &blobs)?;
    let mut findings = Vec::new();
    let mut named = BTreeSet::new();
    let mut read = BTreeSet::new();
    for change in &changes {
        let place = format!("commit {}, {}", change.hash, masked(&change.path, words));
        if named.insert(&change.path) {
            findings.extend(scan_name(&place, &change.path, words));
        }
        if let (true, Some(bytes)) = (
            read.insert((&change.path, &change.blob)),
            contents.get(&change.blob),
        ) {
            let words_apply = !is_exempt(exempt, &change.path);
            findings.extend(scan_content(&place, bytes, words, words_apply));
        }
    }
    Ok(findings)
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
        scan_text(message, &words, true, true)
            .into_iter()
            .map(|(line, rule)| format!("The message, line {line}, {}", rule.message())),
    );
    Ok(problems)
}

/// Whether `path` is a memory note of Claude Code (`.claude/memory/*.md`).
fn is_memory_note(path: &Path) -> bool {
    let folder = path.parent();
    path.extension().is_some_and(|ending| ending == "md")
        && folder
            .and_then(Path::file_name)
            .is_some_and(|name| name == MEMORY_FOLDER)
        && folder
            .and_then(Path::parent)
            .and_then(Path::file_name)
            .is_some_and(|name| name == CLAUDE_FOLDER)
}

/// Whether the line is the session line of a memory note: the key as the first thing on it.
fn is_session_line(line: &str) -> bool {
    let start = line.trim_start().trim_start_matches(QUOTES);
    start
        .get(..MEMORY_SESSION_KEY.len())
        .is_some_and(|key| key.eq_ignore_ascii_case(MEMORY_SESSION_KEY))
        && !value_starts(line, MEMORY_SESSION_KEY).is_empty()
}

/// Removes the session line from a memory note after an edit. Returns whether it changed the file.
pub(crate) fn strip_memory_session(path: &Path) -> anyhow::Result<bool> {
    if !is_memory_note(path) || !path.is_file() {
        return Ok(false);
    }
    let text = fs::read_to_string(path)?;
    let kept: String = text
        .split_inclusive('\n')
        .filter(|line| !is_session_line(line))
        .collect();
    if kept == text {
        return Ok(false);
    }
    fs::write(path, kept)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use crate::testutil::TempDir;

    /// The made-up machine of the examples.
    fn machine() -> PrivateWords {
        let variables = |name: &str| match name {
            "USERNAME" => Some("Zanzibar".to_owned()),
            "COMPUTERNAME" => Some("QUUX-STATION".to_owned()),
            "USER" => Some("ab".to_owned()),
            "HOSTNAME" => Some("Admin".to_owned()),
            _ => None,
        };
        PrivateWords::from_sources(variables, Some("# mine\n\nQuuxcorp\nMüller\n"))
    }

    fn fixture(kind: &str, name: &str) -> String {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(FIXTURES_FOLDER)
            .join(SecPrivate::ID)
            .join(kind)
            .join(name);
        crate::source::normalize(&fs::read_to_string(path).unwrap())
    }

    fn rules(text: &str, in_message: bool) -> Vec<Rule> {
        let mut found: Vec<Rule> = scan_text(text, &machine(), in_message, true)
            .into_iter()
            .map(|(_, rule)| rule)
            .collect();
        found.dedup();
        found
    }

    fn has_word(words: &PrivateWords, text: &str) -> bool {
        !words.word_spans(text).is_empty()
    }

    /// Every line of the fixture breaks exactly the one rule.
    fn assert_every_line_breaks(name: &str, rule: Rule) {
        let text = fixture("bad", name);
        for (number, line) in text.lines().enumerate() {
            let hits = rules(line, false);
            assert_eq!(hits, [rule], "{name}, line {}", number + 1);
        }
    }

    #[test]
    fn bad_example_profile_paths() {
        assert_every_line_breaks("profile-paths.txt", Rule::ProfilePath);
    }

    #[test]
    fn bad_example_session_ids() {
        assert_every_line_breaks("session-ids.txt", Rule::SessionId);
    }

    #[test]
    fn bad_example_private_words() {
        assert_every_line_breaks("private-words.txt", Rule::PrivateWord);
    }

    #[test]
    fn bad_example_a_commit_message_with_a_session_trailer() {
        let text = fixture("bad", "commit-message.txt");
        assert_eq!(rules(&text, true), [Rule::SessionTrailer]);
        // The trailer is a rule of messages: in a file it is only text.
        assert!(rules(&text, false).is_empty());
    }

    #[test]
    fn good_example_allowed_forms_and_made_up_names() {
        let text = fixture("good", "allowed.txt");
        let hits = scan_text(&text, &machine(), false, true);
        assert!(hits.is_empty(), "{hits:?}");
    }

    #[test]
    fn good_example_a_commit_message_without_private_things() {
        let text = fixture("good", "commit-message.txt");
        assert!(scan_text(&text, &machine(), true, true).is_empty());
    }

    #[test]
    fn a_word_ends_where_a_letter_ends() {
        let words = machine();
        for found in [
            "hello zanzibar!",
            "on quux-station.",
            "a quuxcorp b",
            "zanzibar_notes",
            "x_zanzibar",
            "zanzibar2",
            "zanzibar42@mail",
            "ZANZIBAR",
            "MÜLLER and müller",
        ] {
            assert!(has_word(&words, found), "{found}");
        }
        // `ab` is too short and `Admin` is generic: neither is a word.
        assert!(!has_word(&words, "ab admin"));
        // Whole words only: a letter before or after continues the word.
        for missed in ["zanzibarish", "azanzibar", "my quuxcorps"] {
            assert!(!has_word(&words, missed), "{missed}");
        }
    }

    #[test]
    fn the_account_of_the_machine_is_never_a_placeholder_in_a_path() {
        let words = machine();
        let path = |name: &str| format!("{}/{name}/work", "/home");
        // `ab` and `admin` are too short or generic to be words, but in a path they name the owner.
        for own in ["ab", "admin", "zanzibar"] {
            assert!(has_profile_path(&path(own), &words), "{own}");
        }
        assert!(!has_profile_path(&path("runner"), &words));
        assert!(!has_profile_path(&path("runner"), &PrivateWords::default()));
    }

    #[test]
    fn the_name_and_the_text_of_a_file_are_read_in_any_encoding() {
        let temp = TempDir::new("private-names");
        temp.write("notes.txt", "fine\nowner is Zanzibar\n");
        temp.write("zanzibar-notes.md", "fine\n");
        let utf16: Vec<u8> = [0xFF, 0xFE]
            .into_iter()
            .chain("zanzibar\r\n".encode_utf16().flat_map(u16::to_le_bytes))
            .collect();
        temp.write("wide.txt", utf16);
        // Windows-1252, as German Excel writes it: `Müller` is a word of the private list.
        temp.write("latin.txt", b"M\xFCller lives here\n");
        // UTF-16 without a mark, and a file that is no text with a name inside.
        let bare: Vec<u8> = "zanzibar\r\n"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        temp.write("bare.txt", bare.clone());
        let mut binary = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        binary.extend(&bare);
        temp.write("picture.png", binary);
        let files: Vec<String> = [
            "notes.txt",
            "zanzibar-notes.md",
            "wide.txt",
            "latin.txt",
            "bare.txt",
            "picture.png",
        ]
        .map(str::to_owned)
        .into();
        let findings = scan_files(temp.path(), &files, &machine(), &[]);
        let places: Vec<_> = findings
            .iter()
            .filter_map(|finding| finding.location.clone())
            .collect();
        // A name that holds the word is shown masked, in its own finding and in the others.
        assert_eq!(
            places,
            [
                "notes.txt:2",
                "***-notes.md (its name)",
                "wide.txt:1",
                "latin.txt:1",
                "bare.txt:1",
                "picture.png (a file that is no text)",
            ]
            .map(str::to_owned)
        );
        assert_nothing_private_is_said(&findings);
    }

    /// No finding holds one of the words of the made-up machine.
    fn assert_nothing_private_is_said(findings: &[Finding]) {
        for finding in findings {
            let said = format!(
                "{} {}",
                finding.location.as_deref().unwrap_or_default(),
                finding.message
            )
            .to_lowercase();
            for word in ["zanzibar", "quux", "müller"] {
                assert!(!said.contains(word), "{said}");
            }
        }
    }

    #[test]
    fn names_of_branches_tags_and_files_are_masked_in_every_finding() {
        let temp = repo_with_commit("private-masked", "chore: fine");
        temp.write("zanzibar-file.txt", "zanzibar\n");
        assert!(temp.git(&["branch", "quuxcorp-work"]));
        assert!(temp.git(&["tag", "-a", "zanzibar-1", "-m", "Claude-Session: made-up"]));
        let repo = Repo::at(temp.path().to_path_buf());
        let mut findings = scan_files(
            temp.path(),
            &["zanzibar-file.txt".to_owned()],
            &machine(),
            &[],
        );
        findings.extend(scan_history(&repo, &machine(), &[], "").unwrap());
        let places: Vec<_> = findings
            .iter()
            .filter_map(|finding| finding.location.clone())
            .collect();
        for wanted in [
            "***-file.txt (its name)",
            "***-file.txt:1",
            "branch ***-work (its name)",
            "tag ***-1 (its name)",
            "tag ***-1:1",
        ] {
            assert!(
                places.iter().any(|place| place.starts_with(wanted)),
                "{wanted}: {places:?}"
            );
        }
        assert_nothing_private_is_said(&findings);
    }

    #[test]
    fn what_a_commit_that_is_not_pushed_adds_is_read_even_if_a_later_one_removes_it() {
        let temp = repo_with_commit("private-unpushed", "chore: fine");
        let path = format!("{}:{}Users{}mira{}x", "C", "\\", "\\", "\\");
        temp.write("notes.txt", format!("fine\n{path}\n"));
        assert!(temp.git(&["add", "."]));
        assert!(temp.git(&["commit", "--quiet", "-m", "chore: add"]));
        temp.write("notes.txt", "fine\n");
        assert!(temp.git(&["commit", "--quiet", "-a", "-m", "chore: clean"]));
        // The tree is clean; the history is not.
        let places = run_check(&temp, None);
        assert_eq!(places.len(), 1, "{places:?}");
        assert!(places[0].ends_with(", notes.txt:2"), "{places:?}");
        // Once the commits are on a remote, they are the remote's business: only the new ones count.
        let remote = TempDir::new("private-remote");
        assert!(remote.git(&["init", "--quiet", "--bare"]));
        let url = remote.path().to_string_lossy().into_owned();
        assert!(temp.git(&["remote", "add", "origin", &url]));
        assert!(temp.git(&["push", "--quiet", "-u", "origin", "HEAD"]));
        assert!(run_check(&temp, None).is_empty());
        temp.write("more.txt", format!("{path}\n"));
        assert!(temp.git(&["add", "."]));
        assert!(temp.git(&["commit", "--quiet", "-m", "chore: more"]));
        let places = run_check(&temp, None);
        assert_eq!(places.len(), 2, "{places:?}");
        assert!(
            places.iter().any(|place| place.ends_with(", more.txt:1")),
            "{places:?}"
        );
    }

    #[test]
    fn an_exempt_file_is_free_of_private_words_but_not_of_paths() {
        let temp = TempDir::new("private-exempt");
        let path = format!("{}:{}Users{}mira{}x", "C", "\\", "\\", "\\");
        temp.write("list.txt", format!("zanzibar\n{path}\n"));
        let exempt = [Exempt {
            path: "list.txt".to_owned(),
            reason: "A data file the project does not write.".to_owned(),
        }];
        let findings = scan_files(temp.path(), &["list.txt".to_owned()], &machine(), &exempt);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].location.as_deref(), Some("list.txt:2"));
    }

    #[test]
    fn the_private_list_is_read_with_its_marks_and_a_broken_list_is_reported() {
        let temp = TempDir::new("private-list");
        let none = |_: &str| None;
        // No list: nothing to say.
        let (words, problem) = PrivateWords::from_home(Some(temp.path()), none);
        assert!(problem.is_none() && !has_word(&words, "quuxcorp"));
        // A UTF-8 mark on the first line does not hide the first word.
        temp.write(WORDS_FILE, b"\xEF\xBB\xBFQuuxcorp\n");
        let (words, problem) = PrivateWords::from_home(Some(temp.path()), none);
        assert!(problem.is_none() && has_word(&words, "quuxcorp"));
        // So does a UTF-16 list.
        let utf16: Vec<u8> = [0xFF, 0xFE]
            .into_iter()
            .chain("Quuxcorp\r\n".encode_utf16().flat_map(u16::to_le_bytes))
            .collect();
        temp.write(WORDS_FILE, utf16);
        let (words, _) = PrivateWords::from_home(Some(temp.path()), none);
        assert!(has_word(&words, "quuxcorp"));
        // A list that cannot be used is a problem, and the problem names no word.
        temp.write(WORDS_FILE, b"Quux\0corp\n");
        let (_, problem) = PrivateWords::from_home(Some(temp.path()), none);
        assert!(problem.is_some_and(|text| !text.to_lowercase().contains("quux")));
        fs::remove_file(temp.path().join(WORDS_FILE)).unwrap();
        fs::create_dir(temp.path().join(WORDS_FILE)).unwrap();
        let (_, problem) = PrivateWords::from_home(Some(temp.path()), none);
        assert!(problem.is_some());
    }

    /// A repository with the files the check reads, and a commit.
    fn repo_with_commit(name: &str, message: &str) -> TempDir {
        let temp = TempDir::new(name);
        temp.write(EXEMPTIONS_PATH, "");
        temp.write("a.txt", "a\n");
        assert!(temp.git(&["init", "--quiet"]));
        assert!(temp.git(&["add", "."]));
        assert!(temp.git(&["commit", "--quiet", "-m", message]));
        temp
    }

    fn run_check(temp: &TempDir, files: Option<&[String]>) -> Vec<String> {
        let catalog = Catalog::load(&Repo::discover().unwrap()).unwrap();
        let repo = Repo::at(temp.path().to_path_buf());
        let context = Context {
            repo: &repo,
            catalog: &catalog,
            files,
        };
        SecPrivate
            .run(&context)
            .unwrap()
            .into_iter()
            .filter_map(|finding| finding.location)
            .collect()
    }

    #[test]
    fn the_check_reads_every_message_of_every_branch_and_tag() {
        let bad = "chore: more\n\nClaude-Session: made-up";
        let temp = repo_with_commit("private-run", "chore: fine");
        assert!(run_check(&temp, None).is_empty());
        // A commit on another branch, which a push of that branch would send.
        assert!(temp.git(&["switch", "--quiet", "-c", "other"]));
        assert!(temp.git(&["commit", "--quiet", "--allow-empty", "-m", bad]));
        assert!(temp.git(&["switch", "--quiet", "-"]));
        let places = run_check(&temp, None);
        assert_eq!(places.len(), 1, "{places:?}");
        assert!(places[0].starts_with("commit ") && places[0].ends_with(":3"));
        // A staged run looks at files only.
        assert!(run_check(&temp, Some(&[])).is_empty());
        // The message of a tag.
        let temp = repo_with_commit("private-tag", "chore: fine");
        assert!(temp.git(&["tag", "-a", "v1", "-m", bad]));
        let places = run_check(&temp, None);
        assert!(
            places.iter().any(|place| place.starts_with("tag v1")),
            "{places:?}"
        );
    }

    /// A user-profile path with a made-up account, built so this file holds none.
    fn made_up_path() -> String {
        format!("{}:{}Users{}mira{}x", "C", "\\", "\\", "\\")
    }

    #[test]
    fn a_file_name_in_a_commit_is_read_even_if_the_file_is_renamed_later() {
        let temp = repo_with_commit("private-renamed", "chore: fine");
        let id = format!("{}_{}", "session", "01AbCdEfGhIjKlMnOpQrStUv");
        temp.write(&format!("{id}-notes.txt"), "fine\n");
        assert!(temp.git(&["add", "."]));
        assert!(temp.git(&["commit", "--quiet", "-m", "chore: add"]));
        assert!(temp.git(&["mv", &format!("{id}-notes.txt"), "notes.txt"]));
        assert!(temp.git(&["commit", "--quiet", "-m", "chore: rename"]));
        let places = run_check(&temp, None);
        assert_eq!(places.len(), 1, "{places:?}");
        assert!(
            places[0].ends_with(", session_***-notes.txt (its name)"),
            "{places:?}"
        );
    }

    #[test]
    fn text_in_utf16_and_binary_files_of_a_commit_is_read() {
        let temp = repo_with_commit("private-binary-history", "chore: fine");
        let wide: Vec<u8> = [0xFF, 0xFE]
            .into_iter()
            .chain(made_up_path().encode_utf16().flat_map(u16::to_le_bytes))
            .collect();
        temp.write("wide.txt", wide);
        let mut picture = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        picture.extend(made_up_path().bytes());
        picture.extend(b"\0\0");
        temp.write("picture.png", picture);
        assert!(temp.git(&["add", "."]));
        assert!(temp.git(&["commit", "--quiet", "-m", "chore: add"]));
        assert!(temp.git(&["rm", "--quiet", "wide.txt", "picture.png"]));
        assert!(temp.git(&["commit", "--quiet", "-m", "chore: remove"]));
        let places = run_check(&temp, None);
        assert_eq!(places.len(), 2, "{places:?}");
        assert!(places.iter().any(|place| place.ends_with(", wide.txt:1")));
        assert!(
            places
                .iter()
                .any(|place| place.ends_with(", picture.png (a file that is no text)"))
        );
    }

    #[test]
    fn commits_on_other_local_branches_are_read() {
        let temp = repo_with_commit("private-other-branch", "chore: fine");
        assert!(temp.git(&["switch", "--quiet", "-c", "other"]));
        temp.write("notes.txt", made_up_path());
        assert!(temp.git(&["add", "."]));
        assert!(temp.git(&["commit", "--quiet", "-m", "chore: add"]));
        assert!(temp.git(&["switch", "--quiet", "-"]));
        let places = run_check(&temp, None);
        assert_eq!(places.len(), 1, "{places:?}");
        assert!(places[0].ends_with(", notes.txt:1"), "{places:?}");
    }

    #[test]
    fn the_check_reads_the_examples_of_other_checks_but_not_its_own_failing_ones() {
        let temp = repo_with_commit("private-fixtures", "chore: fine");
        let path = format!("{}:{}Users{}mira{}x", "C", "\\", "\\", "\\");
        for folder in [
            "OTHER/bad",
            "OTHER/good",
            "SEC-PRIVATE/good",
            "SEC-PRIVATE/bad",
        ] {
            temp.write(&format!("{FIXTURES_FOLDER}/{folder}/p.txt"), &path);
        }
        temp.write("package-lock.json", &path);
        let places = run_check(&temp, None);
        assert_eq!(places.len(), 4, "{places:?}");
        assert!(
            places
                .iter()
                .all(|place| !place.contains("SEC-PRIVATE/bad"))
        );
        assert!(
            places
                .iter()
                .any(|place| place.starts_with("package-lock.json"))
        );
    }

    #[test]
    fn the_message_file_is_read_up_to_the_scissors_and_comments_count() {
        let temp = TempDir::new("private-message");
        let at = |text: &str| message_problems(&temp.write("MSG", text)).unwrap();
        assert!(at("fix: a\n\nCo-Authored-By: x\n").is_empty());
        assert_eq!(at("fix: a\n\nClaude-Session: x\n").len(), 1);
        // A comment line is read, whatever character Git is set to use for them.
        assert_eq!(at("fix: a\n\n; Claude-Session : x\n").len(), 1);
        // The diff after the scissors line is not the message.
        let diff = format!(
            "fix: a\n\n# {} >8 {}\nClaude-Session: x\n",
            "-".repeat(24),
            "-".repeat(24)
        );
        assert!(at(&diff).is_empty());
        // Nor after it when Git is set to another comment character.
        assert!(at(&diff.replace("# ", "; ")).is_empty());
    }

    #[test]
    fn the_session_line_is_removed_from_a_memory_note_and_nothing_else() {
        let temp = TempDir::new("private-memory");
        let note = temp.write(
            ".claude/memory/note.md",
            format!("---\nname: n\n{MEMORY_SESSION_KEY}: 0123456789abcdef\n---\r\nBody\n"),
        );
        assert!(strip_memory_session(&note).unwrap());
        assert_eq!(
            fs::read_to_string(&note).unwrap(),
            "---\nname: n\n---\r\nBody\n"
        );
        // A second run has nothing to do.
        assert!(!strip_memory_session(&note).unwrap());
    }

    #[test]
    fn other_files_keep_their_text() {
        let temp = TempDir::new("private-memory-other");
        let text = format!("{MEMORY_SESSION_KEY}: 0123456789abcdef\n");
        let elsewhere = temp.write("docs/note.md", &text);
        let not_markdown = temp.write(".claude/memory/note.txt", &text);
        assert!(!strip_memory_session(&elsewhere).unwrap());
        assert!(!strip_memory_session(&not_markdown).unwrap());
        assert_eq!(fs::read_to_string(&elsewhere).unwrap(), text);
        assert!(!strip_memory_session(&temp.path().join(".claude/memory/none.md")).unwrap());
    }
}
