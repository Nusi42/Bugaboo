//! TXT-TERMS: the words of the glossary's *Don't say* column do not appear in what people read
//! (glossary rule 4).
//!
//! Three places are read: every value of the message catalogs (`messages/*.json`), the user
//! documentation (`userDocs/**/*.md`, prose only: code spans and code blocks are left out) and the
//! names in the API description (paths, schema names and property names). The words come from the
//! tables of `docs/glossary.md`: in every table with a *Don't say* column, each entry of that
//! column is a banned word or phrase.
//!
//! How an entry is read:
//!
//! - Entries are separated by commas (commas inside brackets do not count). A word that stands
//!   inside a longer word does not count: `login` fails, `blogging` does not. In text, spaces and
//!   hyphens match each other (`sub-task` finds "sub task"); an underscore belongs to the word, so
//!   a setting name such as `public_url` is not the phrase "public URL". API names are cut into
//!   words first (`link_path`, `LinkPath` and `/link-path` all read "link path").
//! - Upper and lower case are the same, except for an entry written with a capital letter
//!   (`To Do`, `PAT`, `OAuth clients`): that one is matched exactly as written, in text.
//! - A bracket at the end of a cell says where the entry applies. `(in UI)`, `(UI)`, `(as UI label)`,
//!   `(as label)` and `(as tab name)` name something people read, which is what this check reads.
//!   Any other bracket (`(as object)`, `(unless the organization works in shifts)`) says the word
//!   is only wrong in one sense, which no program can tell: those entries are left to review.
//!   Entries that hold a quote mark (`any "xQL" name`) describe a family of words and are left to
//!   review as well.
//!
//! A word that is right in another sense (a *link* to send, the *network* a session came from)
//! gets an entry in `tools/checks/terms-exemptions.toml`, with the places it is allowed in and a
//! reason. An entry that no longer matches anything fails, so the file cannot go stale.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use globset::{Glob, GlobMatcher};
use serde::Deserialize;
use serde_json::Value;

use super::{Check, Context, Finding};
use crate::generate::api::SPEC_PATH;

const GLOSSARY: &str = "docs/glossary.md";
const MESSAGES_FOLDER: &str = "messages";
const USER_DOCS_FOLDER: &str = "userDocs";
const EXEMPTIONS_PATH: &str = "tools/checks/terms-exemptions.toml";
/// The header of the column that holds the banned words.
const COLUMN: &str = "don't say";
/// Words in a bracket that say the entry is about something people read.
const READ_BY_PEOPLE: [&str; 3] = ["UI", "label", "tab"];

pub(crate) struct TxtTerms;

impl Check for TxtTerms {
    fn id(&self) -> &'static str {
        "TXT-TERMS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        scan(context.repo.root())
    }
}

/// One banned word or phrase.
#[derive(Debug, PartialEq, Eq)]
struct Term {
    /// The entry as written in the glossary.
    written: String,
    /// What to say instead (the first column of its row).
    say: String,
    /// The words of the entry, lower case unless `exact`.
    words: Vec<String>,
    /// Matched exactly as written (the entry has a capital letter).
    exact: bool,
}

/// Where a word was found: `key` is what an exemption names, `shown` is what a finding names.
struct Place {
    key: String,
    shown: String,
}

/// What kind of text it is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// Prose or a message: read as written.
    Text,
    /// A name in the API description: cut into words first.
    Name,
}

struct Source {
    place: Place,
    kind: Kind,
    text: String,
}

/// The file of exemptions.
#[derive(Deserialize, Default)]
struct Exemptions {
    #[serde(default)]
    allow: Vec<Allow>,
}

/// A word that is right in the places named: the banned entry as written in the glossary, the
/// places (`messages/en.json#key`, `userDocs/file.md`, `api:property Schema.name`; `*` matches
/// anything), and why.
#[derive(Deserialize)]
struct Allow {
    term: String,
    at: Vec<String>,
    reason: String,
}

/// Reads the glossary and the three places under `root` and reports every banned word.
pub(crate) fn scan(root: &Path) -> anyhow::Result<Vec<Finding>> {
    let glossary = fs::read_to_string(root.join(GLOSSARY))
        .with_context(|| format!("could not read {GLOSSARY}"))?;
    let terms = banned_terms(&glossary);
    let mut findings = Vec::new();
    if terms.is_empty() {
        findings.push(Finding::at(
            GLOSSARY,
            "No table has a \"Don't say\" column with words in it, so nothing can be checked.",
        ));
    }
    let exemptions = match fs::read_to_string(root.join(EXEMPTIONS_PATH)) {
        Ok(text) => {
            toml::from_str(&text).with_context(|| format!("{EXEMPTIONS_PATH} is not valid"))?
        }
        Err(_) => Exemptions::default(),
    };
    let sources = sources(root)?;
    findings.extend(evaluate(&terms, &sources, &exemptions)?);
    Ok(findings)
}

/// Every text people read, with its place.
fn sources(root: &Path) -> anyhow::Result<Vec<Source>> {
    let mut sources = Vec::new();
    let mut catalogs = files(&root.join(MESSAGES_FOLDER), "json")?;
    catalogs.sort();
    for path in catalogs {
        let name = relative(root, &path);
        let text = fs::read_to_string(&path).with_context(|| format!("could not read {name}"))?;
        let pairs: serde_json::Map<String, Value> =
            serde_json::from_str(&text).with_context(|| format!("{name} is not a JSON object"))?;
        for (key, value) in pairs {
            if let Some(message) = value.as_str() {
                sources.push(Source {
                    place: Place {
                        key: format!("{name}#{key}"),
                        shown: format!("{name}: {key}"),
                    },
                    kind: Kind::Text,
                    text: without_placeholders(message),
                });
            }
        }
    }
    let mut pages = files(&root.join(USER_DOCS_FOLDER), "md")?;
    pages.sort();
    for path in pages {
        let name = relative(root, &path);
        let text = fs::read_to_string(&path).with_context(|| format!("could not read {name}"))?;
        sources.push(Source {
            place: Place {
                key: name.clone(),
                shown: name,
            },
            kind: Kind::Text,
            text: prose(&text),
        });
    }
    let spec_path = root.join(SPEC_PATH);
    let spec = fs::read_to_string(&spec_path)
        .with_context(|| format!("could not read {SPEC_PATH}; run `cargo xtask gen api`"))?;
    let spec: Value =
        serde_json::from_str(&spec).with_context(|| format!("{SPEC_PATH} is not valid JSON"))?;
    sources.extend(api_names(&spec));
    Ok(sources)
}

/// The path of `file` below `root`, with `/` between the parts.
fn relative(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .unwrap_or(file)
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// The files with this ending in `folder` and below it. A folder that is not there has none.
fn files(folder: &Path, ending: &str) -> anyhow::Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(folder) else {
        return Ok(found);
    };
    for entry in entries {
        let path = entry?.path();
        if path.is_dir() {
            found.extend(files(&path, ending)?);
        } else if path
            .extension()
            .is_some_and(|extension| extension == ending)
        {
            found.push(path);
        }
    }
    Ok(found)
}

/// A message without the names of its placeholders: `{path}` is code, not text. The words
/// around it and the text inside `{count, plural, one {…}}` stay.
fn without_placeholders(message: &str) -> String {
    let mut text = String::with_capacity(message.len());
    let mut rest = message;
    while let Some(start) = rest.find('{') {
        let (before, from) = rest.split_at(start);
        text.push_str(before);
        match from.find('}') {
            Some(end)
                if from[1..end]
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_') =>
            {
                text.push(' ');
                rest = &from[end + 1..];
            }
            _ => {
                text.push('{');
                rest = &from[1..];
            }
        }
    }
    text.push_str(rest);
    text
}

/// A Markdown page without what is not prose: code blocks, code spans and the addresses of links.
fn prose(page: &str) -> String {
    let mut text = String::with_capacity(page.len());
    let mut in_block = false;
    for line in page.lines() {
        if line.trim_start().starts_with("```") {
            in_block = !in_block;
            text.push('\n');
            continue;
        }
        if !in_block {
            text.push_str(&without_spans(line));
        }
        text.push('\n');
    }
    text
}

/// A line without `code spans` and without the address of a link: `[text](address)`.
fn without_spans(line: &str) -> String {
    let mut text = String::with_capacity(line.len());
    let mut in_span = false;
    let mut in_address = false;
    let mut previous = ' ';
    for c in line.chars() {
        match c {
            '`' => in_span = !in_span,
            '(' if previous == ']' && !in_span => in_address = true,
            ')' if in_address => in_address = false,
            _ if in_span || in_address => {}
            _ => text.push(c),
        }
        previous = c;
    }
    text
}

/// The names in the API description: paths, schema names and property names.
fn api_names(spec: &Value) -> Vec<Source> {
    let mut names = Vec::new();
    let mut add = |kind: &str, name: &str, shown: String| {
        names.push(Source {
            place: Place {
                key: format!("api:{kind} {name}"),
                shown: format!("{SPEC_PATH}: {kind} {name}"),
            },
            kind: Kind::Name,
            text: shown,
        });
    };
    if let Some(paths) = spec.get("paths").and_then(Value::as_object) {
        for path in paths.keys() {
            add("path", path, path.clone());
        }
    }
    if let Some(schemas) = spec
        .pointer("/components/schemas")
        .and_then(Value::as_object)
    {
        for (name, schema) in schemas {
            add("schema", name, name.clone());
            if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
                for property in properties.keys() {
                    add("property", &format!("{name}.{property}"), property.clone());
                }
            }
        }
    }
    names
}

/// The banned words of every table with a *Don't say* column.
fn banned_terms(glossary: &str) -> Vec<Term> {
    let mut terms = Vec::new();
    let mut column: Option<usize> = None;
    for line in glossary.lines() {
        if !line.trim_start().starts_with('|') {
            column = None;
            continue;
        }
        let cells: Vec<&str> = line
            .trim()
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        if let Some(at) = cells
            .iter()
            .position(|cell| cell.to_lowercase().starts_with(COLUMN))
        {
            column = Some(at);
            continue;
        }
        let Some(cell) = column.and_then(|at| cells.get(at)) else {
            continue;
        };
        if cells
            .iter()
            .all(|cell| cell.chars().all(|c| "-: ".contains(c)))
        {
            continue;
        }
        let say = cells[0].replace("**", "");
        terms.extend(cell_terms(cell, &say));
    }
    terms
}

/// The entries of one cell of the *Don't say* column that this check reads.
fn cell_terms(cell: &str, say: &str) -> Vec<Term> {
    let (entries, cell_bracket) = split_bracket(cell.trim());
    let mut terms = Vec::new();
    for entry in split_entries(entries) {
        let (written, own) = split_bracket(&entry);
        let bracket = own.or(cell_bracket);
        let read = bracket.is_none_or(|bracket| {
            bracket
                .split(|c: char| !c.is_alphanumeric())
                .any(|word| READ_BY_PEOPLE.contains(&word))
        });
        let written = written.trim();
        if !read || written.is_empty() || written == "—" || written.contains('"') {
            continue;
        }
        let exact = written.chars().any(char::is_uppercase);
        let words = words(written, true)
            .into_iter()
            .map(|(word, _)| {
                if exact {
                    word.to_owned()
                } else {
                    word.to_lowercase()
                }
            })
            .collect::<Vec<_>>();
        if !words.is_empty() {
            terms.push(Term {
                written: written.to_owned(),
                say: say.to_owned(),
                words,
                exact,
            });
        }
    }
    terms
}

/// The text before a bracket at its end, and what is in that bracket.
fn split_bracket(text: &str) -> (&str, Option<&str>) {
    let text = text.trim();
    if let Some(start) = text.rfind('(')
        && text.ends_with(')')
        && !text[start..text.len() - 1].contains(')')
    {
        return (
            text[..start].trim_end(),
            Some(&text[start + 1..text.len() - 1]),
        );
    }
    (text, None)
}

/// Splits on commas that are not inside brackets.
fn split_entries(text: &str) -> Vec<String> {
    let mut entries = vec![String::new()];
    let mut depth = 0_u32;
    for c in text.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                entries.push(String::new());
                continue;
            }
            _ => {}
        }
        if let Some(last) = entries.last_mut() {
            last.push(c);
        }
    }
    entries
}

/// The words of a text with the place of each (byte offset). A word is a run of letters, digits,
/// apostrophes and, in text, underscores.
fn words(text: &str, underscore_in_word: bool) -> Vec<(&str, usize)> {
    let in_word =
        |c: char| c.is_alphanumeric() || c == '\'' || c == '’' || (underscore_in_word && c == '_');
    let mut found = Vec::new();
    let mut start = None;
    for (at, c) in text.char_indices() {
        match (in_word(c), start) {
            (true, None) => start = Some(at),
            (false, Some(from)) => {
                found.push((&text[from..at], from));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(from) = start {
        found.push((&text[from..], from));
    }
    found
}

/// A name cut into words: `link_path`, `LinkPath` and `/link-path` all become "link path".
fn name_words(name: &str) -> String {
    let mut text = String::with_capacity(name.len() + 4);
    let mut previous = ' ';
    for c in name.chars() {
        if c.is_uppercase() && (previous.is_lowercase() || previous.is_ascii_digit()) {
            text.push(' ');
        }
        text.push(if c.is_alphanumeric() { c } else { ' ' });
        previous = c;
    }
    text
}

/// Where `term` stands in `text` (the byte offset of its first word), if it does.
fn find(text: &str, term: &Term, kind: Kind) -> Option<usize> {
    let words = words(text, kind == Kind::Text);
    let count = term.words.len();
    'start: for first in 0..words.len().checked_sub(count - 1)? {
        for (offset, wanted) in term.words.iter().enumerate() {
            let (word, at) = words[first + offset];
            let same = match (kind, term.exact) {
                (Kind::Name, _) => word.to_lowercase() == wanted.to_lowercase(),
                (Kind::Text, true) => word == wanted,
                (Kind::Text, false) => word.to_lowercase() == *wanted,
            };
            if !same {
                continue 'start;
            }
            if offset > 0 {
                let (previous, from) = words[first + offset - 1];
                let between = &text[from + previous.len()..at];
                if !between.chars().all(|c| c.is_whitespace() || c == '-') {
                    continue 'start;
                }
            }
        }
        return Some(words[first].1);
    }
    None
}

/// The line (counting from 1) the byte offset is on.
fn line_of(text: &str, at: usize) -> usize {
    text[..at].matches('\n').count() + 1
}

fn evaluate(
    terms: &[Term],
    sources: &[Source],
    exemptions: &Exemptions,
) -> anyhow::Result<Vec<Finding>> {
    let allowed = exemptions
        .allow
        .iter()
        .map(|allow| {
            allow
                .at
                .iter()
                .map(|pattern| {
                    Glob::new(pattern)
                        .map(|glob| glob.compile_matcher())
                        .with_context(|| format!("{EXEMPTIONS_PATH}: `{pattern}` is not a pattern"))
                })
                .collect::<anyhow::Result<Vec<GlobMatcher>>>()
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let mut used: BTreeSet<(usize, usize)> = BTreeSet::new();
    let mut findings = Vec::new();
    for allow in &exemptions.allow {
        if allow.reason.trim().is_empty() {
            findings.push(Finding::at(
                EXEMPTIONS_PATH,
                format!("The exemption of \"{}\" has no reason.", allow.term),
            ));
        }
    }
    for source in sources {
        let text = if source.kind == Kind::Name {
            name_words(&source.text)
        } else {
            source.text.clone()
        };
        for term in terms {
            let Some(at) = find(&text, term, source.kind) else {
                continue;
            };
            let exemption = exemptions
                .allow
                .iter()
                .enumerate()
                .filter(|(_, allow)| allow.term.eq_ignore_ascii_case(&term.written))
                .find_map(|(number, _)| {
                    allowed[number]
                        .iter()
                        .position(|matcher| matcher.is_match(&source.place.key))
                        .map(|pattern| (number, pattern))
                });
            if let Some(entry) = exemption {
                used.insert(entry);
                continue;
            }
            let shown = if source.kind == Kind::Text && !source.place.key.contains('#') {
                format!("{}:{}", source.place.shown, line_of(&text, at))
            } else {
                source.place.shown.clone()
            };
            findings.push(Finding::at(
                shown,
                format!(
                    "\"{}\" is on the Don't say list of the glossary. Say \"{}\" instead. If the word is right in this place in another sense, name the place in {EXEMPTIONS_PATH} with the reason.",
                    term.written, term.say
                ),
            ));
        }
    }
    for (number, allow) in exemptions.allow.iter().enumerate() {
        for (pattern, text) in allow.at.iter().enumerate() {
            if !used.contains(&(number, pattern)) {
                findings.push(Finding::at(
                    EXEMPTIONS_PATH,
                    format!(
                        "The exemption of \"{}\" at `{text}` matches nothing. Remove it.",
                        allow.term
                    ),
                ));
            }
        }
    }
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GLOSSARY_TEXT: &str = "\
| Say | Meaning | Don't say |
|-----|---------|-----------|
| **Sign in** | Open your use | log in, login, logout |
| **Group** | A set of people | team (as object) |
| **Search language** | The text you type | any \"xQL\" name |
| **Second step** | The check after the password | MFA, 2FA, TOTP (in UI) |
| **Status group** | One of three | status category, To Do |
| **Issue** | Anything tracked | ticket, work item |
| **Sub-issue** | An issue under another | sub-task, custom field (UI), attribute |
| **Page** | A document | — |

| Term | Meaning |
|------|---------|
| Instance | One server |
";

    fn terms() -> Vec<Term> {
        banned_terms(GLOSSARY_TEXT)
    }

    fn written(terms: &[Term]) -> Vec<&str> {
        terms.iter().map(|term| term.written.as_str()).collect()
    }

    fn text(page: &str) -> Source {
        Source {
            place: Place {
                key: "page.md".to_owned(),
                shown: "page.md".to_owned(),
            },
            kind: Kind::Text,
            text: page.to_owned(),
        }
    }

    fn hits(page: &str) -> Vec<String> {
        evaluate(&terms(), &[text(page)], &Exemptions::default())
            .unwrap()
            .into_iter()
            .map(|finding| finding.message)
            .collect()
    }

    #[test]
    fn txt_terms_reads_the_dont_say_column_of_every_table() {
        let terms = terms();
        assert_eq!(
            written(&terms),
            [
                "log in",
                "login",
                "logout",
                "MFA",
                "2FA",
                "TOTP",
                "status category",
                "To Do",
                "ticket",
                "work item",
                "sub-task",
                "custom field",
                "attribute"
            ]
        );
        assert_eq!(terms[0].say, "Sign in");
    }

    #[test]
    fn txt_terms_leaves_the_entries_of_one_sense_to_review() {
        // `team (as object)` and `any "xQL" name` cannot be told from ordinary use.
        let terms = terms();
        let names = written(&terms);
        assert!(!names.contains(&"team"));
        assert!(!names.iter().any(|name| name.contains("xQL")));
    }

    #[test]
    fn txt_terms_a_word_inside_a_longer_word_does_not_count() {
        assert_eq!(hits("Please login.").len(), 1);
        assert_eq!(hits("Please Login now.").len(), 1);
        assert!(hits("Blogging is fun, and so is logins.").is_empty());
        assert!(hits("A ticketing system.").is_empty());
    }

    #[test]
    fn txt_terms_a_phrase_matches_with_a_space_or_a_hyphen() {
        assert_eq!(hits("You can log in here.").len(), 1);
        assert_eq!(hits("You can log-in here.").len(), 1);
        assert_eq!(hits("You can log\nin here.").len(), 1);
        assert_eq!(hits("A sub task and a sub-task.").len(), 1);
        assert!(hits("Log, then in the morning.").is_empty());
    }

    #[test]
    fn txt_terms_an_entry_with_a_capital_letter_is_matched_as_written() {
        assert_eq!(hits("The status To Do is gone.").len(), 1);
        assert!(hits("You have to do that.").is_empty());
        assert_eq!(hits("Turn on MFA.").len(), 1);
        assert!(hits("Turn on mfa.").is_empty());
    }

    #[test]
    fn txt_terms_a_name_in_a_setting_or_a_placeholder_is_not_text() {
        assert!(hits("Set log_in_url first.").is_empty());
        assert!(without_placeholders("Open {ticket} now").contains("Open"));
        assert!(
            evaluate(
                &terms(),
                &[text(&without_placeholders("Open {ticket} now"))],
                &Exemptions::default()
            )
            .unwrap()
            .is_empty()
        );
        // The text inside a plural stays.
        assert_eq!(
            evaluate(
                &terms(),
                &[text(&without_placeholders(
                    "{count, plural, one {# ticket} other {# tickets}}"
                ))],
                &Exemptions::default()
            )
            .unwrap()
            .len(),
            1
        );
    }

    #[test]
    fn txt_terms_pages_are_read_without_code() {
        let page = "A `login` span and [a link](https://example.test/login) and\n```\nlogin\n```\nplain login";
        let prose = prose(page);
        assert_eq!(hits(&prose).len(), 1);
        assert!(prose.contains("plain login"));
        assert!(!prose.contains("example.test"));
    }

    #[test]
    fn txt_terms_api_names_are_cut_into_words() {
        assert_eq!(name_words("link_path"), "link path");
        assert_eq!(name_words("/api/v1/log-in"), " api v1 log in");
        assert_eq!(name_words("TicketNumberBody"), "Ticket Number Body");
        let name = |text: &str| Source {
            place: Place {
                key: format!("api:schema {text}"),
                shown: format!("api: schema {text}"),
            },
            kind: Kind::Name,
            text: text.to_owned(),
        };
        let found = |text: &str| {
            evaluate(&terms(), &[name(text)], &Exemptions::default())
                .unwrap()
                .len()
        };
        assert_eq!(found("log_in_body"), 1);
        assert_eq!(found("LogInBody"), 1);
        assert_eq!(found("/log-in"), 1);
        assert_eq!(found("login"), 1);
        assert_eq!(found("ticket_id"), 1);
        assert_eq!(found("blogging"), 0);
        assert_eq!(found("sign_in"), 0);
    }

    #[test]
    fn txt_terms_a_finding_says_where_and_what_to_say() {
        let findings =
            evaluate(&terms(), &[text("one\ntwo login")], &Exemptions::default()).unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].location.as_deref(), Some("page.md:2"));
        assert!(findings[0].message.contains("\"login\""));
        assert!(findings[0].message.contains("Sign in"));
    }

    fn allow(term: &str, at: &str) -> Exemptions {
        Exemptions {
            allow: vec![Allow {
                term: term.to_owned(),
                at: vec![at.to_owned()],
                reason: "a test".to_owned(),
            }],
        }
    }

    #[test]
    fn txt_terms_an_exemption_covers_its_place_and_goes_stale_without_one() {
        let sources = [text("Please login.")];
        let exempt = allow("login", "page.*");
        assert!(evaluate(&terms(), &sources, &exempt).unwrap().is_empty());
        // Another place is not covered.
        let other = allow("login", "other.md");
        let findings = evaluate(&terms(), &sources, &other).unwrap();
        assert_eq!(findings.len(), 2, "the finding and the stale exemption");
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("matches nothing"))
        );
        // An exemption of a word that is not used fails.
        let stale = allow("login", "page.md");
        let findings = evaluate(&terms(), &[text("Fine.")], &stale).unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].location.as_deref(), Some(EXEMPTIONS_PATH));
    }

    fn fixture(side: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
            .join("tools/checks/fixtures/TXT-TERMS")
            .join(side)
    }

    #[test]
    fn bad_example() {
        let findings = scan(&fixture("bad")).unwrap();
        let places: Vec<String> = findings
            .iter()
            .filter_map(|finding| finding.location.clone())
            .collect();
        // One finding in each of the three places that are read.
        for wanted in [
            "messages/en.json: ",
            "userDocs/page.md:",
            "openapi.json: path",
            "openapi.json: schema",
            "openapi.json: property",
        ] {
            assert!(
                places.iter().any(|place| place.contains(wanted)),
                "{wanted}: {places:?}"
            );
        }
    }

    #[test]
    fn good_example() {
        let findings = scan(&fixture("good")).unwrap();
        let shown: Vec<String> = findings
            .iter()
            .map(|finding| format!("{:?} {}", finding.location, finding.message))
            .collect();
        assert!(findings.is_empty(), "{shown:?}");
    }

    #[test]
    fn txt_terms_the_repository_passes() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        let findings = scan(&root).unwrap();
        let shown: Vec<String> = findings
            .iter()
            .map(|finding| format!("{:?} {}", finding.location, finding.message))
            .collect();
        assert!(findings.is_empty(), "{shown:#?}");
    }
}
