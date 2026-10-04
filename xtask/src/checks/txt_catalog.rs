//! TXT-CATALOG: the message catalog has no missing, unused or duplicate keys, and the
//! placeholders of every language match (UX-07).
//!
//! Code shows text only through `t("key")`. A key counts as used when its exact text appears as
//! a string in non-test code under `web/` or `crates/`; a `t("…")` call with an unknown key is
//! missing.

use std::collections::{BTreeMap, HashSet};
use std::fs;

use super::node_tool::Selection;
use super::{Check, Context, Finding};
use crate::generate::icu::{Placeholders, placeholders};
use crate::generate::messages::{SOURCE_LOCALE_PATH, parse_pairs};

const MESSAGES_FOLDER: &str = "messages";

/// Code that may use keys.
const CODE: Selection = Selection {
    folders: &["web", "crates"],
    files: &[],
    endings: &[".ts", ".tsx", ".rs"],
};

/// Test files check behavior; their strings must not keep a key alive.
const TEST_MARKERS: [&str; 5] = [".test.", ".spec.", "/e2e/", "/tests/", "generated.ts"];

/// One piece of code: where it is and what it says.
struct Source {
    path: String,
    text: String,
}

pub(crate) struct TxtCatalog;

impl Check for TxtCatalog {
    fn id(&self) -> &'static str {
        "TXT-CATALOG"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let root = context.repo.root();
        let mut locales = Vec::new();
        for entry in fs::read_dir(root.join(MESSAGES_FOLDER))? {
            let path = entry?.path();
            if path
                .extension()
                .is_some_and(|extension| extension == "json")
            {
                let name = format!(
                    "{MESSAGES_FOLDER}/{}",
                    path.file_name()
                        .map(|n| n.to_string_lossy())
                        .unwrap_or_default()
                );
                locales.push((name, fs::read_to_string(&path)?));
            }
        }
        locales.sort_by(|a, b| a.0.cmp(&b.0));
        // Whatever changed, the whole repository is read: one unused key depends on all code.
        let mut sources = Vec::new();
        let everything = Context {
            repo: context.repo,
            catalog: context.catalog,
            files: None,
        };
        for path in CODE.pick(&everything) {
            let text = fs::read_to_string(root.join(&path))?;
            sources.push(Source { path, text });
        }
        Ok(evaluate(&locales, &sources))
    }
}

fn evaluate(locales: &[(String, String)], sources: &[Source]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut catalogs: Vec<(&str, BTreeMap<String, Placeholders>)> = Vec::new();
    for (path, text) in locales {
        match read_locale(path, text) {
            Ok((catalog, mut problems)) => {
                findings.append(&mut problems);
                catalogs.push((path.as_str(), catalog));
            }
            Err(problem) => findings.push(Finding::at(path, problem)),
        }
    }
    let Some((_, source)) = catalogs
        .iter()
        .find(|(path, _)| *path == SOURCE_LOCALE_PATH)
    else {
        findings.push(Finding::at(
            SOURCE_LOCALE_PATH,
            "The source catalog is missing.",
        ));
        return findings;
    };
    for (path, other) in &catalogs {
        if *path != SOURCE_LOCALE_PATH {
            compare_with_source(path, other, source, &mut findings);
        }
    }
    check_usage(source, sources, &mut findings);
    findings
}

/// Parses one catalog; returns its keys with their placeholders and the problems found inside.
fn read_locale(
    path: &str,
    text: &str,
) -> Result<(BTreeMap<String, Placeholders>, Vec<Finding>), String> {
    let pairs =
        parse_pairs(text).map_err(|error| format!("Not a flat JSON object of text: {error}."))?;
    let mut problems = Vec::new();
    let mut catalog = BTreeMap::new();
    let mut seen = HashSet::new();
    for (key, message) in pairs {
        if !valid_key(&key) {
            problems.push(Finding::at(
                path,
                format!("`{key}` is not a valid key; use lower-case words joined by dots, like `issue.title`."),
            ));
        }
        if !seen.insert(key.clone()) {
            problems.push(Finding::at(path, format!("The key `{key}` appears twice.")));
            continue;
        }
        if message.trim().is_empty() {
            problems.push(Finding::at(path, format!("`{key}` has no text.")));
        }
        match placeholders(&message) {
            Ok(found) => {
                catalog.insert(key, found);
            }
            Err(problem) => {
                problems.push(Finding::at(path, format!("`{key}`: {problem}")));
                catalog.insert(key, Placeholders::new());
            }
        }
    }
    Ok((catalog, problems))
}

fn valid_key(key: &str) -> bool {
    let mut parts = key.split('.');
    let word = |part: &str| {
        part.starts_with(|c: char| c.is_ascii_lowercase())
            && part
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    };
    parts.next().is_some_and(word) && key.contains('.') && parts.all(word)
}

fn compare_with_source(
    path: &str,
    other: &BTreeMap<String, Placeholders>,
    source: &BTreeMap<String, Placeholders>,
    findings: &mut Vec<Finding>,
) {
    for (key, expected) in source {
        match other.get(key) {
            None => findings.push(Finding::at(
                path,
                format!("`{key}` is missing; add the translation."),
            )),
            Some(found) => {
                let names = |list: &Placeholders| list.keys().cloned().collect::<Vec<_>>();
                if names(found) != names(expected) {
                    findings.push(Finding::at(
                        path,
                        format!(
                            "`{key}` uses the placeholders {:?}, but {SOURCE_LOCALE_PATH} uses {:?}.",
                            names(found),
                            names(expected)
                        ),
                    ));
                }
            }
        }
    }
    for key in other.keys().filter(|key| !source.contains_key(*key)) {
        findings.push(Finding::at(
            path,
            format!("`{key}` is not in {SOURCE_LOCALE_PATH}; remove it or add it there first."),
        ));
    }
}

fn check_usage(
    source: &BTreeMap<String, Placeholders>,
    sources: &[Source],
    findings: &mut Vec<Finding>,
) {
    let mut used: HashSet<&str> = HashSet::new();
    for file in sources {
        let is_test = TEST_MARKERS.iter().any(|marker| file.path.contains(marker));
        for literal in string_literals(&file.text, is_rust(&file.path)) {
            if !is_test && let Some((key, _)) = source.get_key_value(literal.as_str()) {
                used.insert(key.as_str());
            }
        }
        if is_rust(&file.path) || file.path.contains("generated.ts") {
            continue;
        }
        for key in called_keys(&file.text) {
            if !source.contains_key(&key) {
                findings.push(Finding::at(
                    &file.path,
                    format!("`t(\"{key}\")` uses a key that is not in {SOURCE_LOCALE_PATH}."),
                ));
            }
        }
    }
    for key in source.keys().filter(|key| !used.contains(key.as_str())) {
        findings.push(Finding::at(
            SOURCE_LOCALE_PATH,
            format!("`{key}` is never used. Use it in the code or delete it."),
        ));
    }
}

fn is_rust(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .is_some_and(|extension| extension == "rs")
}

/// The text of the keys in `t("key")` and `t("key", …)` calls.
fn called_keys(code: &str) -> Vec<String> {
    let mut keys = Vec::new();
    for (index, _) in code.match_indices("t(") {
        let before = code[..index].chars().next_back();
        if before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.') {
            continue;
        }
        let rest = code[index + 2..].trim_start();
        if let Some(quote) = rest.chars().next().filter(|c| *c == '"' || *c == '\'')
            && let Some(end) = rest[1..].find(quote)
        {
            keys.push(rest[1..=end].to_owned());
        }
    }
    keys
}

/// Every `"…"` (and, for TypeScript, `'…'`) on a single line.
fn string_literals(code: &str, double_only: bool) -> Vec<String> {
    let mut literals = Vec::new();
    for line in code.lines() {
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '"' && (double_only || c != '\'') {
                continue;
            }
            let mut text = String::new();
            let mut closed = false;
            while let Some(next) = chars.next() {
                if next == '\\' {
                    chars.next();
                } else if next == c {
                    closed = true;
                    break;
                } else {
                    text.push(next);
                }
            }
            if closed {
                literals.push(text);
            }
        }
    }
    literals
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(path: &str, text: &str) -> Source {
        Source {
            path: path.to_owned(),
            text: text.to_owned(),
        }
    }

    fn locale(text: &str) -> (String, String) {
        (SOURCE_LOCALE_PATH.to_owned(), text.to_owned())
    }

    fn messages(findings: &[Finding]) -> String {
        findings
            .iter()
            .map(|f| f.message.clone())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn good_example() {
        let locales = [
            locale(
                r#"{"a.title": "Hello", "a.count": "{n, plural, one {# issue} other {# issues}}"}"#,
            ),
            (
                "messages/de.json".to_owned(),
                r#"{"a.title": "Hallo", "a.count": "{n, plural, one {# Fall} other {# Fälle}}"}"#
                    .to_owned(),
            ),
        ];
        let code = [source(
            "web/app/src/A.tsx",
            r#"const k = "a.count"; t("a.title"); t("a.count", { n: 2 });"#,
        )];
        assert!(evaluate(&locales, &code).is_empty());
    }

    #[test]
    fn bad_example() {
        let locales = [
            locale(
                r#"{"a.title": "Hello {name", "a.unused": "x", "Bad Key": "y", "a.count": "{n} items"}"#,
            ),
            (
                "messages/de.json".to_owned(),
                r#"{"a.count": "{m} Fälle", "a.extra": "z"}"#.to_owned(),
            ),
        ];
        let code = [source(
            "web/app/src/A.tsx",
            r#"t("a.count"); t("a.missing"); t("a.title");"#,
        )];
        let text = messages(&evaluate(&locales, &code));
        assert!(text.contains("is never closed"), "{text}");
        assert!(text.contains("`a.unused` is never used"), "{text}");
        assert!(text.contains("`Bad Key` is not a valid key"), "{text}");
        assert!(text.contains("a.missing"), "{text}");
        assert!(text.contains("uses the placeholders"), "{text}");
        assert!(text.contains("`a.extra` is not in"), "{text}");
    }

    #[test]
    fn duplicate_keys_fail() {
        let locales = [locale(r#"{"a.b": "x", "a.b": "y"}"#)];
        let code = [source("web/a.ts", r#"t("a.b")"#)];
        assert!(messages(&evaluate(&locales, &code)).contains("appears twice"));
    }

    #[test]
    fn strings_in_tests_do_not_count_as_use() {
        let locales = [locale(r#"{"a.b": "x"}"#)];
        let code = [source("web/a.test.ts", r#"expect(t("a.b"))"#)];
        assert!(messages(&evaluate(&locales, &code)).contains("never used"));
    }

    #[test]
    fn keys_are_checked_for_shape() {
        assert!(valid_key("issue.title"));
        assert!(valid_key("a.b-c.d2"));
        assert!(!valid_key("title"));
        assert!(!valid_key("Issue.title"));
        assert!(!valid_key("issue..title"));
    }
}
