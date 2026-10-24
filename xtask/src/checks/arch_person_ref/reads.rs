//! The reads of ARCH-PERSON-REF: where the code takes the name or the address of a person from a
//! record.
//!
//! A read is `.display_name` or `.email` of any value, and a destructuring that names the field
//! (`let Person { display_name, .. } = p;`, in a parameter, in a `match` arm). The code of `bb-api`
//! and `bb-app` is read without comments, texts and tests. The exemptions name the file *and the
//! function* with the number of reads, so a read cannot be moved from one function to another
//! unseen.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::{EXEMPTIONS_PATH, READ_FIELDS, READ_FOLDERS};
use crate::checks::Finding;
use crate::checks::exemptions;
use crate::rust_source::{strip, without_tests};
use crate::source::Source;

/// A function of a file that may read the name or the address of a person from a record, how many
/// times, and why.
#[derive(Deserialize)]
pub(super) struct ReadExemption {
    pub(super) file: String,
    pub(super) function: String,
    pub(super) reads: usize,
    pub(super) reason: String,
}

impl exemptions::Exemption for ReadExemption {
    fn name(&self) -> String {
        format!("{} {}", self.file, self.function)
    }

    fn reason(&self) -> &str {
        &self.reason
    }
}

/// One read: the function it is in (empty outside a function) and its line.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Read {
    pub(super) function: String,
    pub(super) line: usize,
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The word that starts at `from`, if one does.
fn word_at(code: &[char], from: usize) -> Option<String> {
    let word: String = code[from..].iter().take_while(|c| is_word(**c)).collect();
    (!word.is_empty()).then_some(word)
}

/// The position of the `}` that closes the `{` at `open`.
fn closing(code: &[char], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (at, c) in code.iter().enumerate().skip(open) {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(at);
                }
            }
            _ => {}
        }
    }
    None
}

/// The index of the first character at or after `from` that is not white space.
fn skip_space(code: &[char], from: usize) -> usize {
    (from..code.len())
        .find(|at| !code[*at].is_whitespace())
        .unwrap_or(code.len())
}

/// Where the path of the type name that heads the group `{` at `open` starts (`Person`,
/// `Self`, `crate::Person`). Nothing when the group follows no capitalised name.
fn head(code: &[char], open: usize) -> Option<usize> {
    let mut end = open;
    while end > 0 && code[end - 1].is_whitespace() {
        end -= 1;
    }
    let mut start = end;
    while start > 0 && (is_word(code[start - 1]) || code[start - 1] == ':') {
        start -= 1;
    }
    let path: String = code[start..end].iter().collect();
    let last = path.rsplit("::").next()?;
    last.chars().next().filter(|c| c.is_uppercase())?;
    Some(start)
}

/// Whether the group `{ … }` from `open` to `close`, headed by a name that starts at `start`, is a
/// pattern and not a struct literal or a definition.
fn is_pattern(code: &[char], start: usize, open: usize, close: usize) -> bool {
    // `..` followed by the end or a comma is the rest of a pattern; `..base` is an update.
    let inner = &code[open + 1..close];
    let mut depth = 0usize;
    let mut at = 0;
    while at < inner.len() {
        match inner[at] {
            '{' | '(' | '[' => depth += 1,
            '}' | ')' | ']' => depth = depth.saturating_sub(1),
            '.' if depth == 0
                && inner.get(at + 1) == Some(&'.')
                && (at == 0 || inner[at - 1] != '.')
                && inner.get(at + 2) != Some(&'.')
                && inner.get(at + 2) != Some(&'=') =>
            {
                let next = skip_space(inner, at + 2);
                if next >= inner.len() || inner[next] == ',' {
                    return true;
                }
            }
            _ => {}
        }
        at += 1;
    }
    // After `let`, `if let` or `while let`.
    let mut before = start;
    while before > 0 && code[before - 1].is_whitespace() {
        before -= 1;
    }
    let mut word_start = before;
    while word_start > 0 && is_word(code[word_start - 1]) {
        word_start -= 1;
    }
    let previous: String = code[word_start..before].iter().collect();
    if previous == "let" || (before > 0 && code[before - 1] == '|') {
        return true;
    }
    // Before `=`, `=>` or `:` (a parameter).
    let after = skip_space(code, close + 1);
    match (code.get(after), code.get(after + 1)) {
        (Some('='), Some('>')) => true,
        (Some('='), next) => next != Some(&'='),
        (Some(':'), next) => next != Some(&':'),
        _ => false,
    }
}

/// The reads of the fields of a pattern group, as positions in the code.
fn pattern_reads(code: &[char], open: usize, close: usize) -> Vec<usize> {
    let mut found = Vec::new();
    let mut depth = 0usize;
    let mut at = open + 1;
    let mut entry = true;
    while at < close {
        let c = code[at];
        if c == ',' && depth == 0 {
            entry = true;
        } else if matches!(c, '{' | '(' | '[') {
            depth += 1;
        } else if matches!(c, '}' | ')' | ']') {
            depth = depth.saturating_sub(1);
        } else if entry && !c.is_whitespace() {
            entry = false;
            let mut name_at = at;
            while let Some(word) = word_at(code, name_at) {
                let after = name_at + word.len();
                if matches!(word.as_str(), "ref" | "mut") {
                    name_at = skip_space(code, after);
                    continue;
                }
                if READ_FIELDS.contains(&word.trim_start_matches("r#")) {
                    found.push(name_at);
                }
                break;
            }
        }
        at += 1;
    }
    found
}

/// The reads of a name or an address in the file: where, and in which function.
pub(super) fn reads(path: &str, text: &str) -> Vec<Read> {
    let in_tests = path.contains("/tests/") || path.ends_with("tests.rs");
    if in_tests || !READ_FOLDERS.iter().any(|folder| path.starts_with(folder)) {
        return Vec::new();
    }
    let code: Vec<char> = strip(&without_tests(text)).chars().collect();
    let mut lines = Vec::with_capacity(code.len());
    let mut line = 1;
    for c in &code {
        lines.push(line);
        if *c == '\n' {
            line += 1;
        }
    }
    let mut found = Vec::new();
    // The functions we are inside, with the depth of their body, and the one whose body is coming
    // (with the parentheses still open in its head).
    let mut inside: Vec<(String, usize)> = Vec::new();
    let mut coming: Option<(String, usize)> = None;
    let mut depth = 0usize;
    for at in 0..code.len() {
        match code[at] {
            '(' => {
                if let Some((_, parens)) = coming.as_mut() {
                    *parens += 1;
                }
            }
            ')' => {
                if let Some((_, parens)) = coming.as_mut() {
                    *parens = parens.saturating_sub(1);
                }
            }
            ';' => {
                if coming.as_ref().is_some_and(|(_, parens)| *parens == 0) {
                    coming = None;
                }
            }
            '{' => {
                depth += 1;
                let in_head = coming.as_ref().is_some_and(|(_, parens)| *parens > 0);
                if (in_head || coming.is_none())
                    && let Some(close) = closing(&code, at)
                    && let Some(start) = head(&code, at)
                    && is_pattern(&code, start, at, close)
                {
                    let function = coming
                        .as_ref()
                        .map_or_else(|| current(&inside), |(name, _)| name.clone());
                    for position in pattern_reads(&code, at, close) {
                        found.push(Read {
                            function: function.clone(),
                            line: lines[position],
                        });
                    }
                } else if let Some((name, _)) = coming.take() {
                    inside.push((name, depth));
                }
            }
            '}' => {
                if inside.last().is_some_and(|(_, opened)| *opened == depth) {
                    inside.pop();
                }
                depth = depth.saturating_sub(1);
            }
            '.' => {
                if let Some(word) = word_at(&code, at + 1)
                    && READ_FIELDS.contains(&word.as_str())
                    && (at == 0 || code[at - 1] != '.')
                {
                    found.push(Read {
                        function: current(&inside),
                        line: lines[at],
                    });
                }
            }
            c if is_word(c)
                && (at == 0 || !is_word(code[at - 1]))
                && word_at(&code, at).as_deref() == Some("fn") =>
            {
                let next = skip_space(&code, at + 2);
                if next > at + 2
                    && let Some(name) = word_at(&code, next)
                {
                    coming = Some((name, 0));
                }
            }
            _ => {}
        }
    }
    found
}

/// The name of the innermost function.
fn current(inside: &[(String, usize)]) -> String {
    inside
        .last()
        .map(|(name, _)| name.clone())
        .unwrap_or_default()
}

/// The findings for the reads of all files against the entries.
pub(super) fn read_findings(sources: &[Source], allowed: &[ReadExemption]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut seen: BTreeMap<(String, String), Vec<usize>> = BTreeMap::new();
    for (path, text) in sources {
        for read in reads(path, text) {
            seen.entry((path.clone(), read.function))
                .or_default()
                .push(read.line);
        }
    }
    for ((path, function), lines) in &seen {
        let entry = allowed
            .iter()
            .find(|entry| entry.file == *path && entry.function == *function);
        let where_ = lines
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        match entry {
            Some(entry) if entry.reads == lines.len() => {}
            Some(entry) => findings.push(Finding::at(
                path,
                format!(
                    "`{function}` reads a person's name or address {} times (lines {where_}); {EXEMPTIONS_PATH} allows {}. A new read is a new way to show a person outside `PersonRef`: resolve the person for the viewer, or change the entry with a reason.",
                    lines.len(),
                    entry.reads
                ),
            )),
            None => findings.push(Finding::at(
                path,
                format!(
                    "`{function}` reads a person's name or address {} times (lines {where_}) and shows people outside `PersonRef`, or is not listed. Resolve the person for the viewer, or name the file and function in {EXEMPTIONS_PATH} with a reason.",
                    lines.len()
                ),
            )),
        }
    }
    for entry in allowed {
        if !seen.contains_key(&(entry.file.clone(), entry.function.clone())) {
            findings.push(Finding::at(
                EXEMPTIONS_PATH,
                format!(
                    "The entry for `{}` in `{}` matches no read. Remove it.",
                    entry.function, entry.file
                ),
            ));
        }
    }
    findings.extend(exemptions::reason_findings(EXEMPTIONS_PATH, allowed));
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = "crates/bb-app/src/x.rs";

    fn found(text: &str) -> Vec<(String, usize)> {
        reads(FILE, text)
            .into_iter()
            .map(|read| (read.function, read.line))
            .collect()
    }

    fn of(function: &str, line: usize) -> (String, usize) {
        (function.to_owned(), line)
    }

    #[test]
    fn bad_example_a_destructuring_that_names_the_field_is_a_read() {
        let text = "fn one(p: Person) {
    let Person { display_name, .. } = p;
}
fn two(p: Person) {
    let Person { id, email: address, .. } = p;
}
fn three(Person { email, .. }: Person) -> String {
    email
}
fn four(p: Person) -> String {
    match p {
        Person { ref display_name, .. } => display_name.clone(),
    }
}
fn five(p: Option<Person>) {
    if let Some(Person { email, .. }) = p {}
}
fn six(p: Person) {
    let Person { mut display_name, email } = p;
}
";
        assert_eq!(
            found(text),
            vec![
                of("one", 2),
                of("two", 5),
                of("three", 7),
                of("four", 12),
                of("five", 16),
                of("six", 19),
                of("six", 19),
            ]
        );
    }

    #[test]
    fn good_example_a_literal_a_definition_and_an_update_are_no_reads() {
        let text = "struct Person { display_name: String, email: String }
impl Person {
    fn make(name: String) -> Person {
        Person { display_name: name, email: String::new() }
    }
    fn again(base: Person) -> Person {
        Person { id: 1, ..base }
    }
}
fn other(p: &Person) -> Body { Body { email: p.address.clone(), display_name_of: 1 } }
";
        assert!(found(text).is_empty(), "{:?}", found(text));
    }

    #[test]
    fn bad_example_a_read_belongs_to_the_function_it_is_in() {
        let text = "fn a(p: &Person) -> String {
    let make = |q: &Person| q.email.clone();
    make(p) + &p.display_name
}
impl Mail {
    fn b(&self, p: &Person) -> String { p.email.clone() }
}
fn c(p: &Person) -> String { p.email.clone() }
const NAME: usize = 1;
";
        assert_eq!(
            found(text),
            vec![of("a", 2), of("a", 3), of("b", 6), of("c", 8)]
        );
    }

    #[test]
    fn good_example_a_comment_a_text_a_test_and_another_field_are_no_reads() {
        let text = "// p.email
fn a(p: &Person) { let _ = \"p.display_name\"; let _ = p.display_name_of; let _ = p.emails; }
#[cfg(test)]
mod tests { fn t(p: &Person) { let _ = p.email; } }
";
        assert!(found(text).is_empty());
        assert!(reads("crates/bb-store/src/x.rs", "fn f(p: P) { p.email }").is_empty());
        assert!(reads("crates/bb-app/src/tests/x.rs", "fn f(p: P) { p.email }").is_empty());
    }
}
