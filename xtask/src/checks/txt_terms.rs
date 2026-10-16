//! TXT-TERMS: the words of the glossary's *Don't say* column do not appear in what people read
//! (glossary rule 4).
//!
//! Three places are read: every value of the message catalogs (`messages/*.json`, without the
//! names of placeholders and the keywords of plurals and selects), the user documentation
//! (`userDocs/**/*.md`, prose only: code spans, fenced and indented code are left out) and the
//! names in the API description (paths, schema names, property names, the values of enums,
//! operation IDs and the names of parameters). The words come from the
//! tables of `docs/glossary.md`: in every table with a *Don't say* column, each entry of that
//! column is a banned word or phrase.
//!
//! How an entry is read:
//!
//! - Entries are separated by commas (commas inside brackets do not count). An entry with a
//!   slash is read as its parts when one of them has several words (`check out / check in`); a list
//!   of single words (`none / read / write`) names a set of labels and stays one phrase. A word that stands
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
//! gets an entry in `tools/checks/terms-exemptions.toml`, with the places it is allowed in (for a
//! page of the user docs, the phrases it is allowed in) and a reason. An entry that no longer
//! matches anything fails, so the file cannot go stale.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::Context as _;
use globset::{Glob, GlobMatcher};
use serde::Deserialize;
use serde_json::Value;

use super::{Check, Context, Finding};
use super::{api_spec, exemptions};
use crate::repo::Repo;
use crate::source::read_text;

mod code_names;
mod readable;

const GLOSSARY: &str = "docs/glossary.md";
const MESSAGES_FOLDER: &str = "messages";
const USER_DOCS_FOLDER: &str = "userDocs";
const EXEMPTIONS_PATH: &str = "tools/checks/terms-exemptions.toml";
/// The header of the column that holds the banned words.
const COLUMN: &str = "don't say";
/// Words in a bracket that say the entry is about something people read.
const READ_BY_PEOPLE: [&str; 3] = ["UI", "label", "tab"];
/// The word in a bracket that says the entry is also about the names the code declares. Most
/// entries are ordinary words of programming (`state`, `token`, `link`), which are right in code, so
/// code names are only read for the entries that ask for it.
const READ_IN_CODE: &str = "code";

pub(crate) struct TxtTerms;

impl Check for TxtTerms {
    fn id(&self) -> &'static str {
        "TXT-TERMS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let files = context.everything().selected(self.id())?;
        // The words of texts and API names come from places that one changed file can break far
        // away, so they are always read whole. The names in code are read in the files that changed
        // (everywhere in a run of the whole repository): reading every file of the code takes longer
        // than an edit may take.
        match context.files {
            Some(changed) => {
                let code: Vec<String> = files
                    .iter()
                    .filter(|file| changed.contains(file))
                    .cloned()
                    .collect();
                scan_with(context.repo, &files, &code, Coverage::Changed)
            }
            None => scan_with(context.repo, &files, &files, Coverage::Whole),
        }
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
    /// Read in what people read and in the API description (the entry has no bracket, or one with
    /// `UI`, `label` or `tab`).
    people: bool,
    /// Read in the names the code declares (the bracket of the entry says `code`).
    code: bool,
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
    /// A name the code declares (a type, a function, a file, a message key): cut into words first,
    /// and only the entries that say `code` apply.
    Code,
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
/// anything), optionally the phrases it is right in, and why.
#[derive(Deserialize)]
struct Allow {
    term: String,
    at: Vec<String>,
    /// When given, the word is only right inside these phrases of the place, not everywhere in
    /// it (for a page of the user docs). Without phrases the whole place is exempt.
    #[serde(default)]
    phrases: Vec<String>,
    reason: String,
}

impl exemptions::Exemption for Allow {
    fn name(&self) -> String {
        self.term.clone()
    }

    fn reason(&self) -> &str {
        &self.reason
    }
}

/// How much of the code the names were read in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Coverage {
    /// In every file: an exemption that matches no name is stale.
    Whole,
    /// Only in the files that changed: an exemption of a name in code may be for a file that was
    /// not read, so it is not called stale.
    Changed,
}

/// Reads the glossary and the places among `files` (the files the catalogue entry covers), and
/// the names of the code in `code_files`, and reports every banned word.
fn scan_with(
    repo: &Repo,
    files: &[String],
    code_files: &[String],
    coverage: Coverage,
) -> anyhow::Result<Vec<Finding>> {
    let glossary = read_text(repo, GLOSSARY)?;
    let terms = banned_terms(&glossary);
    let mut findings = Vec::new();
    if terms.is_empty() {
        findings.push(Finding::at(
            GLOSSARY,
            "No table has a \"Don't say\" column with words in it, so nothing can be checked.",
        ));
    }
    let exemptions = if files.iter().any(|file| file == EXEMPTIONS_PATH) {
        exemptions::load(repo, EXEMPTIONS_PATH)?
    } else {
        Exemptions::default()
    };
    let sources = sources(repo, files, code_files)?;
    findings.extend(evaluate_with(&terms, &sources, &exemptions, coverage)?);
    Ok(findings)
}

/// The files among `files` in `folder` with this ending.
fn in_folder<'a>(
    files: &'a [String],
    folder: &'a str,
    ending: &'a str,
) -> impl Iterator<Item = &'a String> {
    files.iter().filter(move |file| {
        file.strip_prefix(folder)
            .is_some_and(|rest| rest.starts_with('/') && file.ends_with(ending))
    })
}

/// Every text people read, with its place.
fn sources(repo: &Repo, files: &[String], code_files: &[String]) -> anyhow::Result<Vec<Source>> {
    let mut sources = Vec::new();
    for name in in_folder(files, MESSAGES_FOLDER, ".json") {
        let text = read_text(repo, name)?;
        let pairs: serde_json::Map<String, Value> =
            serde_json::from_str(&text).with_context(|| format!("{name} is not a JSON object"))?;
        for (key, value) in pairs {
            // The key is a name the code uses; an exemption of the message covers both.
            sources.push(Source {
                place: Place {
                    key: format!("{name}#{key}"),
                    shown: format!("{name}: {key} (the key)"),
                },
                kind: Kind::Code,
                text: key.clone(),
            });
            if let Some(message) = value.as_str() {
                sources.push(Source {
                    place: Place {
                        key: format!("{name}#{key}"),
                        shown: format!("{name}: {key}"),
                    },
                    kind: Kind::Text,
                    text: readable::message_text(message),
                });
            }
        }
    }
    for name in in_folder(files, USER_DOCS_FOLDER, ".md") {
        let text = read_text(repo, name)?;
        sources.push(Source {
            place: Place {
                key: name.clone(),
                shown: name.clone(),
            },
            kind: Kind::Text,
            text: readable::prose(&text),
        });
    }
    sources.extend(code_sources(repo, code_files)?);
    for name in readable::api_names(&api_spec::load(repo)?) {
        let (key, shown) = readable::api_place(&name);
        sources.push(Source {
            place: Place { key, shown },
            kind: Kind::Name,
            text: name.text,
        });
    }
    Ok(sources)
}

/// The names the code declares (types, functions, modules, files and the like), each as a name to
/// read. The place of a name is `path#what name`, the place of a folder or file is its path.
fn code_sources(repo: &Repo, files: &[String]) -> anyhow::Result<Vec<Source>> {
    let mut sources = Vec::new();
    let mut named = |path: &str, what: &str, name: String| {
        sources.push(Source {
            place: Place {
                key: format!("{path}#{what} {name}"),
                shown: format!("{path}: {what} {name}"),
            },
            kind: Kind::Code,
            text: name,
        });
    };
    for (path, declared) in code_names::path_names(files) {
        named(&path, declared.what, declared.name);
    }
    for path in files.iter().filter(|path| code_names::is_read(path)) {
        let extension = Path::new(path)
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let declared = match extension.as_str() {
            "rs" => code_names::rust_names(&read_text(repo, path)?)
                .with_context(|| format!("{path} cannot be read for its names"))?,
            "ts" | "tsx" => code_names::web_names(&read_text(repo, path)?),
            _ => continue,
        };
        for declared in declared {
            named(path, declared.what, declared.name);
        }
    }
    Ok(sources)
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
        let people = bracket.is_none_or(|bracket| {
            bracket
                .split(|c: char| !c.is_alphanumeric())
                .any(|word| READ_BY_PEOPLE.contains(&word))
        });
        let code = bracket.is_some_and(|bracket| {
            bracket
                .split(|c: char| !c.is_alphanumeric())
                .any(|word| word == READ_IN_CODE)
        });
        let written = written.trim();
        if !(people || code) || written.is_empty() || written == "—" || written.contains('"') {
            continue;
        }
        // `check out / check in` is two phrases. A list of single words (`none / read / write`)
        // names a set of labels and stays one phrase: `read` and `write` alone are ordinary words.
        let parts: Vec<&str> = written.split(" / ").map(str::trim).collect();
        let parts = if parts.iter().all(|part| !part.contains(' ')) {
            vec![written]
        } else {
            parts
        };
        for part in parts {
            let exact = part.chars().any(char::is_uppercase);
            let words = words(part, true)
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
                    written: part.to_owned(),
                    say: say.to_owned(),
                    words,
                    exact,
                    people,
                    code,
                });
            }
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
                (Kind::Name | Kind::Code, _) => word.to_lowercase() == wanted.to_lowercase(),
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

/// The text with every occurrence of the phrases (in any case) blanked out, keeping its length and
/// its lines.
fn without_phrases(text: &str, phrases: &[String]) -> String {
    let mut bytes = text.as_bytes().to_vec();
    let lower = text.to_ascii_lowercase();
    for phrase in phrases {
        let phrase = phrase.to_ascii_lowercase();
        if phrase.is_empty() {
            continue;
        }
        for (at, _) in lower.match_indices(&phrase) {
            for byte in &mut bytes[at..at + phrase.len()] {
                if *byte != b'\n' {
                    *byte = b' ';
                }
            }
        }
    }
    String::from_utf8(bytes).unwrap_or_else(|_| text.to_owned())
}

/// The line (counting from 1) the byte offset is on.
fn line_of(text: &str, at: usize) -> usize {
    text[..at].matches('\n').count() + 1
}

fn evaluate_with(
    terms: &[Term],
    sources: &[Source],
    exemptions: &Exemptions,
    coverage: Coverage,
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
    let mut findings = exemptions::reason_findings(EXEMPTIONS_PATH, &exemptions.allow);
    for source in sources {
        let text = if source.kind == Kind::Text {
            source.text.clone()
        } else {
            name_words(&source.text)
        };
        for term in terms {
            let reads = if source.kind == Kind::Code {
                term.code
            } else {
                term.people
            };
            if !reads {
                continue;
            }
            let Some(at) = find(&text, term, source.kind) else {
                continue;
            };
            // The entries of the word that name this place. One with phrases is used when it
            // blanks something out, and covers the word when nothing of it is left.
            let mut covered = false;
            for (number, allow) in exemptions.allow.iter().enumerate() {
                if !allow.term.eq_ignore_ascii_case(&term.written) {
                    continue;
                }
                let Some(pattern) = allowed[number]
                    .iter()
                    .position(|matcher| matcher.is_match(&source.place.key))
                else {
                    continue;
                };
                if allow.phrases.is_empty() {
                    used.insert((number, pattern));
                    covered = true;
                    break;
                }
                let rest = without_phrases(&text, &allow.phrases);
                if rest != text {
                    used.insert((number, pattern));
                }
                if find(&rest, term, source.kind).is_none() {
                    covered = true;
                    break;
                }
            }
            if covered {
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
            // A name in code may be in a file that was not read.
            let maybe_unread = coverage == Coverage::Changed && code_names::is_read(text);
            if !used.contains(&(number, pattern)) && !maybe_unread {
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
mod tests;
