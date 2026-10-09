//! SEC-PRIVATE: nothing private about the owner in Git (coding-standards §2).
//!
//! What it looks for, in every tracked text file and in every commit message:
//!
//! 1. a user-profile path with an account name (`X:\Users\<name>\`, `X:/Users/<name>/`,
//!    `/c/Users/<name>/`, `/home/<name>/`, `/Users/<name>/`); `%USERPROFILE%` and `~` are the
//!    allowed forms, and so are the generic accounts of [`GENERIC_ACCOUNTS`];
//! 2. a Claude session ID or link: the memory-note key with a value, a session link with an ID,
//!    and `session_` followed by a long ID (letters and digits, 20 or more); in a commit message
//!    also any session trailer line;
//! 3. a private word, as a whole word in any case: the values of `USERNAME`, `COMPUTERNAME`,
//!    `USER` and `HOSTNAME` of the machine the check runs on (a value of fewer than
//!    [`MIN_NAME_LEN`] characters or a generic word of [`GENERIC_NAMES`] is ignored, because it
//!    would match ordinary text), and the words of the optional list
//!    `~/.config/bugaboo/private-words.txt` (one per line; blank lines and `#` lines ignored; the
//!    list lives outside the repository).
//!
//! The check never prints a private word: a finding names the place (file and line, or commit and
//! line) and the rule only. Files in `tools/checks/private-exemptions.toml` are exempt from the
//! rule on private words (not from the others), each with a reason: they are data the project
//! does not write.
//!
//! The words of a machine come from [`PrivateWords`], which tests build by hand, so no example
//! depends on the real machine.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::exemptions::{self, Exemption};
use super::git_commit_msg;
use super::{Check, Context, Finding};
use crate::source::{is_word_char, word_positions};

/// The line Claude Code adds to every memory note it saves; `cargo xtask hook post-edit` removes
/// it again.
pub(crate) const MEMORY_SESSION_KEY: &str = "originSessionId:";
/// The folder of the memory notes, and the one above it.
const MEMORY_FOLDER: &str = "memory";
const MEMORY_PARENT: &str = ".claude";

const SESSION_LINK: &str = "claude.ai/code/session_";
const SESSION_PREFIX: &str = "session_";
/// The trailer Claude Code can ask for at the end of a commit message, in lower case.
const COMMIT_TRAILER: &str = "claude-session:";
/// The shortest ID after the key or the link.
const MIN_ID_LEN: usize = 8;
/// The shortest run of letters and digits after `session_` that counts as an ID.
const LONG_ID_LEN: usize = 20;

/// The variables that name the account and the computer.
const NAME_VARIABLES: [&str; 4] = ["USERNAME", "COMPUTERNAME", "USER", "HOSTNAME"];
/// A value shorter than this would match ordinary words.
const MIN_NAME_LEN: usize = 4;
/// Values of the variables that name no one (shared by many machines), in lower case.
const GENERIC_NAMES: [&str; 17] = [
    "administrator",
    "admin",
    "root",
    "user",
    "users",
    "default",
    "localhost",
    "runner",
    "runneradmin",
    "ubuntu",
    "desktop",
    "laptop",
    "computer",
    "guest",
    "system",
    "docker",
    "vagrant",
];
/// Accounts in a path that name no one (CI machines, shared folders, placeholders) and the
/// made-up names of test data, in lower case.
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
/// The list of private words, below the home folder.
const WORDS_FILE: &str = ".config/bugaboo/private-words.txt";
const EXEMPTIONS_PATH: &str = "tools/checks/private-exemptions.toml";
/// The characters between the parts of a path.
const SEPARATORS: [char; 2] = ['/', '\\'];

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

/// The words that must not appear, in lower case.
#[derive(Default)]
pub(crate) struct PrivateWords(BTreeSet<String>);

impl PrivateWords {
    /// The words of a machine: `variable` answers for an environment variable, `list` is the text
    /// of the private list if there is one.
    pub(crate) fn from_sources(
        variable: impl Fn(&str) -> Option<String>,
        list: Option<&str>,
    ) -> Self {
        let mut words = BTreeSet::new();
        for name in NAME_VARIABLES {
            let Some(value) = variable(name) else {
                continue;
            };
            let value = value.trim().to_ascii_lowercase();
            if value.chars().count() >= MIN_NAME_LEN && !GENERIC_NAMES.contains(&value.as_str()) {
                words.insert(value);
            }
        }
        for line in list.unwrap_or_default().lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('#') {
                words.insert(line.to_ascii_lowercase());
            }
        }
        Self(words)
    }

    /// The words of the machine this runs on.
    pub(crate) fn from_machine() -> Self {
        let list = home_folder()
            .and_then(|home| fs::read_to_string(Path::new(&home).join(WORDS_FILE)).ok());
        Self::from_sources(|name| std::env::var(name).ok(), list.as_deref())
    }

    fn found_in(&self, lower_line: &str) -> bool {
        self.0
            .iter()
            .any(|word| word_positions(lower_line, word).next().is_some())
    }
}

fn home_folder() -> Option<String> {
    ["USERPROFILE", "HOME"]
        .iter()
        .find_map(|name| std::env::var(name).ok())
}

/// Whether the text `rest` (after `Users` or `home`) is a separator run followed by an account
/// name that is not a placeholder; `needs_end` also asks for a separator (or the end) after it.
fn account_follows(rest: &str, needs_end: bool) -> bool {
    let after_separators = rest.trim_start_matches(SEPARATORS);
    if after_separators.len() == rest.len() {
        return false;
    }
    let name: String = after_separators
        .chars()
        .take_while(|c| c.is_alphanumeric() || matches!(c, '_' | '.' | '-'))
        .collect();
    if !name.chars().next().is_some_and(char::is_alphanumeric)
        || GENERIC_ACCOUNTS.contains(&name.to_ascii_lowercase().as_str())
    {
        return false;
    }
    !needs_end
        || after_separators[name.len()..]
            .chars()
            .next()
            .is_none_or(|c| SEPARATORS.contains(&c))
}

/// Whether the keyword at `at` in `line` starts a profile path.
fn profile_path_at(line: &str, at: usize, keyword: &str, exact: &str) -> bool {
    let before = &line[..at];
    let prefix = before.trim_end_matches(SEPARATORS);
    if prefix.len() == before.len() {
        return false;
    }
    let rest = &line[at + keyword.len()..];
    let drive = prefix
        .strip_suffix(':')
        .and_then(|p| p.chars().next_back())
        .is_some_and(|c| c.is_ascii_alphabetic());
    let mounted = prefix
        .strip_suffix(|c: char| c.is_ascii_alphabetic())
        .is_some_and(|p| p.ends_with('/'));
    // `C:\Users\…` and `/c/Users/…` are certain. A bare `/Users/…` or `/home/…` has to look like a
    // home folder: the exact word, a name and a slash, and not the end of a longer path.
    if drive || mounted {
        return account_follows(rest, false);
    }
    let alone = prefix.is_empty() || prefix.ends_with(|c: char| !is_word_char(c) && c != '/');
    &line[at..at + keyword.len()] == exact
        && alone
        && before.ends_with('/')
        && account_follows(rest, true)
}

/// Whether `line` holds a user-profile path with an account name.
fn has_profile_path(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    [("users", "Users"), ("home", "home")]
        .iter()
        .any(|(keyword, exact)| {
            lower
                .match_indices(keyword)
                .any(|(at, _)| profile_path_at(line, at, keyword, exact))
        })
}

/// Whether `line` holds a session ID or link.
fn has_session_id(line: &str) -> bool {
    let id_len = |rest: &str| {
        rest.chars()
            .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
            .count()
    };
    let key = line
        .match_indices(MEMORY_SESSION_KEY)
        .any(|(at, _)| id_len(line[at + MEMORY_SESSION_KEY.len()..].trim_start()) >= MIN_ID_LEN);
    let in_url = line
        .match_indices(SESSION_LINK)
        .any(|(at, _)| id_len(&line[at + SESSION_LINK.len()..]) >= MIN_ID_LEN);
    let long = line.match_indices(SESSION_PREFIX).any(|(at, _)| {
        let run: String = line[at + SESSION_PREFIX.len()..]
            .chars()
            .take_while(char::is_ascii_alphanumeric)
            .collect();
        run.len() >= LONG_ID_LEN
            && run.chars().any(|c| c.is_ascii_digit())
            && run.chars().any(|c| c.is_ascii_alphabetic())
    });
    key || in_url || long
}

/// The rules one line breaks. `in_message` adds the rules of commit messages; `names` says
/// whether the rule on private words applies.
fn rules_of(line: &str, words: &PrivateWords, in_message: bool, names: bool) -> Vec<Rule> {
    let mut rules = Vec::new();
    if has_profile_path(line) {
        rules.push(Rule::ProfilePath);
    }
    if has_session_id(line) {
        rules.push(Rule::SessionId);
    }
    let lower = line.to_ascii_lowercase();
    if in_message && lower.trim_start().starts_with(COMMIT_TRAILER) {
        rules.push(Rule::SessionTrailer);
    }
    if names && words.found_in(&lower) {
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

pub(crate) struct SecPrivate;

impl Check for SecPrivate {
    fn id(&self) -> &'static str {
        "SEC-PRIVATE"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let words = PrivateWords::from_machine();
        let exempt: Exemptions = exemptions::load(context.repo, EXEMPTIONS_PATH)?;
        // Whether an exemption is still needed depends on the machine (a name may or may not be in
        // the file), so the file is judged on its reasons and on the file it names only.
        let mut findings = exemptions::reason_findings(EXEMPTIONS_PATH, &exempt.exempt);
        findings.extend(
            exempt
                .exempt
                .iter()
                .filter(|entry| !context.repo.path(&entry.path).is_file())
                .map(|entry| {
                    Finding::at(
                        EXEMPTIONS_PATH,
                        format!("The entry `{}` names no file. Remove it.", entry.path),
                    )
                }),
        );
        let files = context.selected(self.id())?;
        findings.extend(scan_files(
            context.repo.root(),
            &files,
            &words,
            &exempt.exempt,
        ));
        // Commit messages belong to the history, which a staged run does not look at (the
        // commit-msg hook checks the message being written).
        if context.files.is_none() {
            findings.extend(scan_history(context.repo.real_root(), &words)?);
        }
        Ok(findings)
    }
}

fn finding(place: &str, line: usize, rule: Rule) -> Finding {
    Finding::at(format!("{place}:{line}"), rule.message())
}

fn scan_files(
    root: &Path,
    files: &[String],
    words: &PrivateWords,
    exempt: &[Exempt],
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for file in files {
        // A file that is not UTF-8 text is binary: nothing to read.
        let Some(text) = fs::read(root.join(file))
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
        else {
            continue;
        };
        let names = !exempt.iter().any(|entry| &entry.path == file);
        findings.extend(
            scan_text(&text, words, false, names)
                .into_iter()
                .map(|(line, rule)| finding(file, line, rule)),
        );
    }
    findings
}

/// The messages of all commits reachable from `HEAD` (a superset of any pushed range).
fn scan_history(root: &Path, words: &PrivateWords) -> anyhow::Result<Vec<Finding>> {
    let Some(log) = git_commit_msg::git(root, &["log", "--format=%h%x1f%B%x1e", "HEAD"])? else {
        return Ok(Vec::new());
    };
    let mut findings = Vec::new();
    for record in log.split('\u{1e}') {
        let Some((hash, message)) = record.trim_start().split_once('\u{1f}') else {
            continue;
        };
        findings.extend(
            scan_text(message, words, true, true)
                .into_iter()
                .map(|(line, rule)| finding(&format!("commit {hash}"), line, rule)),
        );
    }
    Ok(findings)
}

/// What is private in a commit message file written by Git (comment lines are skipped).
pub(crate) fn message_problems(file: &Path) -> anyhow::Result<Vec<String>> {
    let text = fs::read_to_string(file)?;
    let message: String = text
        .lines()
        .map(|line| if line.starts_with('#') { "" } else { line })
        .collect::<Vec<_>>()
        .join("\n");
    Ok(
        scan_text(&message, &PrivateWords::from_machine(), true, true)
            .into_iter()
            .map(|(line, rule)| format!("The message, line {line}, {}", rule.message()))
            .collect(),
    )
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
            .is_some_and(|name| name == MEMORY_PARENT)
}

/// Removes the session line from a memory note after an edit. Returns whether it changed the file.
pub(crate) fn strip_memory_session(path: &Path) -> anyhow::Result<bool> {
    if !is_memory_note(path) || !path.is_file() {
        return Ok(false);
    }
    let text = fs::read_to_string(path)?;
    let kept: String = text
        .split_inclusive('\n')
        .filter(|line| !line.trim_start().starts_with(MEMORY_SESSION_KEY))
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
    use crate::testutil::TempDir;

    const FIXTURES: &str = "../tools/checks/fixtures/SEC-PRIVATE";

    /// The made-up machine of the examples.
    fn machine() -> PrivateWords {
        let variables = |name: &str| match name {
            "USERNAME" => Some("Zanzibar".to_owned()),
            "COMPUTERNAME" => Some("QUUX-STATION".to_owned()),
            "USER" => Some("ab".to_owned()),
            "HOSTNAME" => Some("Admin".to_owned()),
            _ => None,
        };
        PrivateWords::from_sources(variables, Some("# mine\n\nQuuxcorp\n"))
    }

    fn fixture(kind: &str, name: &str) -> String {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(FIXTURES)
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

    /// Every line of the fixture breaks exactly the one rule.
    fn assert_every_line_breaks(name: &str, rule: Rule) {
        let text = fixture("bad", name);
        let hits = scan_text(&text, &machine(), false, true);
        assert_eq!(hits.len(), text.lines().count(), "{name}: {hits:?}");
        assert!(hits.iter().all(|(_, hit)| *hit == rule), "{name}: {hits:?}");
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
    fn the_words_skip_short_and_generic_values() {
        let words = machine();
        assert!(words.found_in("hello zanzibar!"));
        assert!(words.found_in("on quux-station."));
        assert!(words.found_in("a quuxcorp b"));
        // `ab` is too short and `Admin` is generic: neither is a word.
        assert!(!words.found_in("ab admin"));
        // Whole words only.
        assert!(!words.found_in("zanzibarish"));
        assert!(!words.found_in("my_quuxcorp"));
    }

    #[test]
    fn a_file_with_a_private_word_names_place_and_rule_but_never_the_word() {
        let temp = TempDir::new("private-names");
        temp.write("notes.txt", "fine\nowner is Zanzibar\n");
        let findings = scan_files(temp.path(), &["notes.txt".to_owned()], &machine(), &[]);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].location.as_deref(), Some("notes.txt:2"));
        assert!(!findings[0].message.to_lowercase().contains("zanzibar"));
    }

    #[test]
    fn an_exempt_file_is_free_of_private_words_but_not_of_paths() {
        let temp = TempDir::new("private-exempt");
        let path = format!("{}:{}Users{}ana{}x", "C", "\\", "\\", "\\");
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
    fn history_messages_are_read_and_the_hash_is_the_place() {
        let temp = TempDir::new("private-history");
        temp.write("a.txt", "a\n");
        assert!(temp.git(&["init", "--quiet"]));
        assert!(temp.git(&["add", "."]));
        assert!(temp.git(&["commit", "--quiet", "-m", "chore: fine"]));
        assert!(scan_history(temp.path(), &machine()).unwrap().is_empty());
        let message = "chore: more\n\nClaude-Session: made-up";
        assert!(temp.git(&["commit", "--quiet", "--allow-empty", "-m", message]));
        let findings = scan_history(temp.path(), &machine()).unwrap();
        assert_eq!(findings.len(), 1);
        let place = findings[0].location.as_deref().unwrap();
        assert!(place.starts_with("commit "), "{place}");
        assert!(place.ends_with(":3"), "{place}");
    }

    #[test]
    fn a_repository_without_commits_has_no_history_to_read() {
        let temp = TempDir::new("private-empty");
        assert!(temp.git(&["init", "--quiet"]));
        assert!(scan_history(temp.path(), &machine()).unwrap().is_empty());
    }

    #[test]
    fn the_message_file_skips_git_comment_lines() {
        let temp = TempDir::new("private-message");
        let file = temp.write("MSG", "fix: a\n\n# a comment with a path\n");
        assert!(message_problems(&file).unwrap().is_empty());
    }

    #[test]
    fn the_session_line_is_removed_from_a_memory_note_and_nothing_else() {
        let temp = TempDir::new("private-memory");
        let note = temp.write(
            ".claude/memory/note.md",
            format!("---\nname: n\n{MEMORY_SESSION_KEY} 0123456789abcdef\n---\r\nBody\n"),
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
        let text = format!("{MEMORY_SESSION_KEY} 0123456789abcdef\n");
        let elsewhere = temp.write("docs/note.md", &text);
        let not_markdown = temp.write(".claude/memory/note.txt", &text);
        assert!(!strip_memory_session(&elsewhere).unwrap());
        assert!(!strip_memory_session(&not_markdown).unwrap());
        assert_eq!(fs::read_to_string(&elsewhere).unwrap(), text);
        assert!(!strip_memory_session(&temp.path().join(".claude/memory/none.md")).unwrap());
    }
}
