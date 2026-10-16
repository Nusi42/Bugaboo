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
    spans.extend(id_spans_after(line, SESSION_LINK));
    let lower = line.to_ascii_lowercase();
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

/// Where the IDs stand that follow `prefix` (in lower case) in the line, in any case.
fn id_spans_after(line: &str, prefix: &str) -> Vec<Span> {
    let lower = line.to_ascii_lowercase();
    lower
        .match_indices(prefix)
        .filter_map(|(at, _)| {
            let start = at + prefix.len();
            let length = id_len(&line[start..]);
            (length >= MIN_ID_LEN).then_some((start, start + length))
        })
        .collect()
}

/// Where the IDs of published pages stand in the line.
fn page_link_spans(line: &str) -> Vec<Span> {
    PAGE_LINKS
        .iter()
        .flat_map(|link| id_spans_after(line, link))
        .collect()
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
    if !page_link_spans(line).is_empty() {
        rules.push(Rule::PageLink);
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
    spans.extend(page_link_spans(text));
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
mod tests;
