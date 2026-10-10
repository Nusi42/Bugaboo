//! TXT-CATALOG: the message catalog has no missing, unused or duplicate keys, and the
//! placeholders of every language match (UX-07).
//!
//! Code shows text only through `t("key")` (browser) or the generated constants and functions of
//! `bb-core` (Rust). A key counts as used when its exact text appears as a string in non-test code
//! under `web/` or `crates/`, or when Rust code names its constant or function
//! (`SERVER_NOT_FOUND`, `server_port_busy`); a `t("…")` call with an unknown key is missing. The
//! messages that the words of the enums make (an error code, a permission and its topic, a starting
//! role, a kind of people information, a field) count as used. That each of them is in the catalog
//! is the build's job: a made key that is not there does not compile (`bb-core`, `derived_key!`).

use std::collections::{BTreeMap, HashSet};

use bb_core::ErrorCode;
use bb_core::access::{FieldLevel, PeopleLevel};
use bb_core::fields::Field;
use bb_core::issue::{IssueIcon, Priority, Status, StatusGroup};
use bb_core::issue_key::IdMode;
use bb_core::message::MessageKey;
use bb_core::own_fields::{ChoiceColor, FieldFor, FieldType, Look};
use bb_core::people_kinds::PeopleKind;
use bb_policy::{Permission, StartingRole, Topic};

use super::{Check, Context, Finding};
use crate::docs::text::has_extension;
use crate::generate::icu::{Placeholders, placeholders};
use crate::generate::messages::{SOURCE_LOCALE_PATH, parse_pairs};
use crate::generate::messages_rust::{constant_name, function_name};
use crate::rust_source::{strip_comments, without_tests};
use crate::source::read_text;

const MESSAGES_FOLDER: &str = "messages/";

/// Test files check behavior; their strings must not keep a key alive.
const TEST_MARKERS: [&str; 6] = [
    ".test.",
    ".spec.",
    "/e2e/",
    "/tests/",
    "generated.ts",
    "generated.rs",
];

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
        // Whatever changed, the whole repository is read: one unused key depends on all code.
        let mut locales = Vec::new();
        let mut sources = Vec::new();
        for path in context.everything().selected(self.id())? {
            let text = read_text(context.repo, &path)?;
            if path.starts_with(MESSAGES_FOLDER) {
                if has_extension(&path, "json") {
                    locales.push((path, text));
                }
            } else {
                sources.push(Source { path, text });
            }
        }
        Ok(evaluate(&locales, &sources, &made_keys()))
    }
}

/// A message that the word of an enum makes, and the word.
struct Made {
    key: MessageKey,
    word: String,
}

/// Every message that the words of the enums make (`derived_key!` in `bb-core`): the code
/// does not write these keys, so the check finds them here.
fn made_keys() -> Vec<Made> {
    fn made(kind: &str, word: &str, key: MessageKey) -> Made {
        Made {
            key,
            word: format!("{kind} `{word}`"),
        }
    }
    let mut keys = Vec::new();
    for code in ErrorCode::ALL {
        keys.push(made("the error code", code.code(), code.message()));
    }
    for permission in Permission::ALL {
        keys.push(made(
            "the permission",
            permission.code(),
            permission.sentence(),
        ));
    }
    for topic in Topic::ALL {
        keys.push(made("the topic", topic.code(), topic.name()));
    }
    for role in StartingRole::ALL {
        keys.push(made("the starting role", role.code(), role.name()));
    }
    for kind in PeopleKind::ALL {
        keys.push(made(
            "the kind of people information",
            kind.code(),
            kind.name(),
        ));
    }
    for field in Field::ALL {
        keys.push(made("the field", field.code(), field.name()));
    }
    for status in Status::ALL {
        keys.push(made("the status", status.code(), status.name()));
    }
    for group in StatusGroup::ALL {
        keys.push(made("the status group", group.code(), group.name()));
    }
    for priority in Priority::ALL {
        keys.push(made("the priority", priority.code(), priority.name()));
    }
    for icon in IssueIcon::ALL {
        keys.push(made("the icon of an issue type", icon.code(), icon.name()));
    }
    for mode in IdMode::ALL {
        keys.push(made("the mode of issue IDs", mode.code(), mode.name()));
    }
    for level in FieldLevel::ALL {
        keys.push(made("the field level", level.code(), level.name()));
    }
    for kind in FieldType::ALL {
        keys.push(made("the example of the kind", kind.code(), kind.example()));
        keys.push(made("the kind of an own field", kind.code(), kind.name()));
    }
    for what in FieldFor::ALL {
        keys.push(made("what an own field is for", what.code(), what.name()));
    }
    for color in ChoiceColor::ALL {
        keys.push(made("the color of a choice", color.code(), color.name()));
    }
    for look in Look::ALL {
        keys.push(made("the look of an own field", look.code(), look.name()));
    }
    for level in PeopleLevel::ALL {
        keys.push(made("the people level", level.code(), level.name()));
    }
    keys
}

fn evaluate(locales: &[(String, String)], sources: &[Source], made: &[Made]) -> Vec<Finding> {
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
    check_usage(source, sources, made, &mut findings);
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
    made: &[Made],
    findings: &mut Vec<Finding>,
) {
    let mut used: HashSet<&str> = HashSet::new();
    for made in made {
        match source.get_key_value(made.key.as_str()) {
            Some((key, _)) => {
                used.insert(key.as_str());
            }
            None => findings.push(Finding::at(
                SOURCE_LOCALE_PATH,
                format!(
                    "`{}` is the message of {} but is not in the catalog; add it.",
                    made.key, made.word
                ),
            )),
        }
    }
    for file in sources {
        let is_test = TEST_MARKERS.iter().any(|marker| file.path.contains(marker));
        let text = if is_rust(&file.path) {
            product_code(&file.text)
        } else {
            file.text.clone()
        };
        for literal in string_literals(&text, is_rust(&file.path)) {
            if !is_test && let Some((key, _)) = source.get_key_value(literal.as_str()) {
                used.insert(key.as_str());
            }
        }
        if is_rust(&file.path) && !is_test {
            let names = identifiers(&text);
            for key in source.keys() {
                if names.contains(constant_name(key).as_str())
                    || names.contains(function_name(key).as_str())
                {
                    used.insert(key.as_str());
                }
            }
        }
        if is_rust(&file.path) || file.path.contains("generated.ts") {
            continue;
        }
        for key in called_keys(&text) {
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

/// The Rust code that ships: without the unit tests at the end of the file and without comment
/// lines, so a key that only a test or a comment mentions still counts as unused.
fn product_code(text: &str) -> String {
    strip_comments(&without_tests(text))
}

/// The words of Rust code: everything made of letters, digits and underscores.
fn identifiers(code: &str) -> HashSet<&str> {
    code.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|word| !word.is_empty())
        .collect()
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
        assert!(evaluate(&locales, &code, &[]).is_empty());
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
        let text = messages(&evaluate(&locales, &code, &[]));
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
        assert!(messages(&evaluate(&locales, &code, &[])).contains("appears twice"));
    }

    #[test]
    fn strings_in_tests_do_not_count_as_use() {
        let locales = [locale(r#"{"a.b": "x"}"#)];
        let code = [source("web/a.test.ts", r#"expect(t("a.b"))"#)];
        assert!(messages(&evaluate(&locales, &code, &[])).contains("never used"));
    }

    #[test]
    fn rust_code_uses_a_key_by_naming_its_constant_or_function() {
        let locales = [locale(
            r#"{"a.not-found": "x", "a.port": "Port {n}", "a.lonely": "y"}"#,
        )];
        let code = [
            source(
                "crates/bb-api/src/a.rs",
                "let a = message::A_NOT_FOUND; let b = message::a_port(5);",
            ),
            source(
                "crates/bb-core/src/message/generated.rs",
                "pub const A_LONELY: MessageKey = MessageKey::new(\"a.lonely\");",
            ),
        ];
        let text = messages(&evaluate(&locales, &code, &[]));
        assert!(!text.contains("`a.not-found`"), "{text}");
        assert!(!text.contains("`a.port`"), "{text}");
        assert!(text.contains("`a.lonely` is never used"), "{text}");
    }

    #[test]
    fn rust_tests_and_comments_do_not_keep_a_key_alive() {
        let locales = [locale(
            r#"{"a.in-test": "x", "a.in-comment": "y", "a.real": "z"}"#,
        )];
        let code = [source(
            "crates/bb-api/src/a.rs",
            "// A_IN_COMMENT\nlet a = message::A_REAL;\n#[cfg(test)]\nmod tests { fn t() { message::A_IN_TEST; } }\n",
        )];
        let text = messages(&evaluate(&locales, &code, &[]));
        assert!(text.contains("`a.in-test` is never used"), "{text}");
        assert!(text.contains("`a.in-comment` is never used"), "{text}");
        assert!(!text.contains("`a.real`"), "{text}");
    }

    #[test]
    fn keys_are_checked_for_shape() {
        assert!(valid_key("issue.title"));
        assert!(valid_key("a.b-c.d2"));
        assert!(!valid_key("title"));
        assert!(!valid_key("Issue.title"));
        assert!(!valid_key("issue..title"));
    }

    #[test]
    fn a_message_that_a_word_makes_counts_as_used_and_must_be_in_the_catalog() {
        let made = [Made {
            key: ErrorCode::NotFound.message(),
            word: "the error code `resource.not_found`".to_owned(),
        }];
        let present = [locale(r#"{"error.resource.not-found": "Not here."}"#)];
        assert!(evaluate(&present, &[], &made).is_empty());
        let absent = [locale(r#"{"a.other": "x"}"#)];
        let text = messages(&evaluate(&absent, &[], &made));
        assert!(
            text.contains("`error.resource.not-found` is the message of the error code"),
            "{text}"
        );
    }

    #[test]
    fn every_word_of_this_repository_has_its_message() {
        let repo = crate::repo::Repo::discover().unwrap();
        let text = read_text(&repo, SOURCE_LOCALE_PATH).unwrap();
        let locales = [locale(&text)];
        let found = messages(
            &evaluate(&locales, &[], &made_keys())
                .into_iter()
                .filter(|finding| finding.message.contains("is the message of"))
                .collect::<Vec<_>>(),
        );
        assert!(found.is_empty(), "{found}");
    }
}
