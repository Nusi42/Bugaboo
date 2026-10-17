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

use super::{Check, Context, Finding};
use crate::docs::text::has_extension;
use crate::generate::icu::{Placeholders, placeholders};
use crate::generate::made_keys::{Made, made_keys};
use crate::generate::messages::{
    FEATURES_FOLDER, PART_IMPORT, Quotes, SHARED, SOURCE_LOCALE_PATH, feature_entries,
    features_without_part, is_generated_catalog, is_server_only, is_test_or_generated, parse_pairs,
    parts_of, string_literals,
};
use crate::generate::messages_rust::{constant_name, function_name};
use crate::rust_source::{strip_comments, without_tests};
use crate::source::read_text;
use crate::web_code::strip_comments as strip_web_comments;

const MESSAGES_FOLDER: &str = "messages/";

/// One piece of code: where it is and what it says.
#[derive(Clone)]
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
    check_parts(source, sources, made, &mut findings);
    findings
}

/// The browser catalog is split into parts (PERF-04, ADR-0009): the first page loads `shared`, and
/// an area brings its own part along with its chunk. `t()` fails for a text whose part is not
/// loaded, so the entry of every feature that has a part imports it, and nothing else does.
fn check_parts(
    source: &BTreeMap<String, Placeholders>,
    sources: &[Source],
    made: &[Made],
    findings: &mut Vec<Finding>,
) {
    let keys = source
        .keys()
        .map(String::as_str)
        .filter(|key| !is_server_only(key));
    let code: Vec<crate::source::Source> = sources
        .iter()
        .map(|file| (file.path.clone(), file.text.clone()))
        .collect();
    let made: Vec<&str> = made.iter().map(|made| made.key.as_str()).collect();
    let filled: HashSet<&str> = parts_of(keys, &code, &made).into_values().collect();
    for feature in features_without_part(sources.iter().map(|file| file.path.as_str())) {
        findings.push(Finding::at(
            format!("{FEATURES_FOLDER}{feature}"),
            "This feature has no message part, so its texts would stay on the first page. Add `part!(\"…\")` with its folder name to PARTS in xtask/src/gen/messages.rs, run `cargo xtask gen messages`, and import the part in its index.ts.",
        ));
    }
    let import_of = |part: &str| format!("{PART_IMPORT}{part}");
    for part in filled.iter().filter(|part| **part != SHARED) {
        let entries = feature_entries(part);
        let imported = sources.iter().any(|file| {
            entries.contains(&file.path) && starts_with_import(&file.text, &import_of(part))
        });
        if !imported {
            findings.push(Finding::at(
                &entries[0],
                format!(
                    "Its texts are in their own part. Make `import \"{}\";` the first import of this file (in code, not in a comment), so they are loaded with this area before anything of it runs; without it `t()` fails.",
                    import_of(part)
                ),
            ));
        }
    }
    for file in sources
        .iter()
        .filter(|file| !is_generated_catalog(&file.path))
    {
        for literal in string_literals(&file.text, Quotes::DoubleAndSingle) {
            let Some(part) = literal.strip_prefix(PART_IMPORT) else {
                continue;
            };
            if !feature_entries(part).contains(&file.path) {
                findings.push(Finding::at(
                    &file.path,
                    format!(
                        "Only the entry of the feature `{part}` (`index.ts`) imports its message part. `shared` is loaded by the messages package itself."
                    ),
                ));
            }
        }
    }
}

/// Whether the first statement of the code of `text` (comments do not count) is the side-effect
/// import of `module`: the part must be evaluated before anything else of the area is.
fn starts_with_import(text: &str, module: &str) -> bool {
    let code = strip_web_comments(text);
    let first = code.trim_start().split(';').next().unwrap_or_default();
    first
        .strip_prefix("import")
        .is_some_and(|rest| rest.trim_start().starts_with(['"', '\'']))
        && string_literals(first, Quotes::DoubleAndSingle) == [module]
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
        let is_test = is_test_or_generated(&file.path);
        let text = if is_rust(&file.path) {
            product_code(&file.text)
        } else {
            strip_web_comments(&file.text)
        };
        for literal in string_literals(
            &text,
            if is_rust(&file.path) {
                Quotes::Double
            } else {
                Quotes::DoubleAndSingle
            },
        ) {
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
        if is_rust(&file.path) || is_generated_catalog(&file.path) {
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

#[cfg(test)]
mod tests {
    use bb_core::ErrorCode;

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
    fn an_area_must_import_its_message_part() {
        let locales = [locale(r#"{"fields.list": "Fields", "nav.home": "Home"}"#)];
        let page = source(
            "web/app/src/features/fields/Page.tsx",
            r#"t("fields.list"); t("nav.home")"#,
        );
        let shell = source("web/app/src/routes/Root.tsx", r#"t("nav.home")"#);
        let without = source("web/app/src/features/fields/index.ts", "export {};");
        let text = messages(&evaluate(
            &locales,
            &[page.clone(), shell.clone(), without],
            &[],
        ));
        assert!(text.contains("@bugaboo/messages/parts/fields"), "{text}");
        let with = source(
            "web/app/src/features/fields/index.ts",
            r#"import "@bugaboo/messages/parts/fields";"#,
        );
        assert!(evaluate(&locales, &[page, shell, with], &[]).is_empty());
    }

    #[test]
    fn the_part_must_be_the_first_import_of_the_entry_and_in_code() {
        let locales = [locale(r#"{"fields.list": "Fields"}"#)];
        let page = source(
            "web/app/src/features/fields/Page.tsx",
            r#"t("fields.list")"#,
        );
        let entry = |text: &str| source("web/app/src/features/fields/index.ts", text);
        let part = r#"import "@bugaboo/messages/parts/fields";"#;
        let places =
            |entry: Source| -> usize { evaluate(&locales, &[page.clone(), entry], &[]).len() };
        assert_eq!(places(entry(part)), 0);
        assert_eq!(
            places(entry(&format!(
                "// the texts\n/* more */\n{part}\nexport {{}};"
            ))),
            0
        );
        // After another import, the part is evaluated too late.
        let late = format!("import {{ A }} from \"./A.tsx\";\n{part}");
        assert_eq!(places(entry(&late)), 1);
        // In a comment, it is not there.
        let commented = format!("// {part}\nexport {{}};");
        assert_eq!(places(entry(&commented)), 1);
        let block = format!("/*\n{part}\n*/\nexport {{}};");
        assert_eq!(places(entry(&block)), 1);
        // A named import of the module is no side-effect import.
        assert_eq!(
            places(entry(
                "import { x } from \"@bugaboo/messages/parts/fields\";"
            )),
            1
        );
    }

    #[test]
    fn typescript_comments_are_not_code() {
        let locales = [locale(r#"{"a.used": "x", "a.only-commented": "y"}"#)];
        let code = [source(
            "web/app/src/A.tsx",
            "t(\"a.used\"); // t(\"a.only-commented\") t(\"a.missing\")\n/* t(\"a.also-missing\") */",
        )];
        let text = messages(&evaluate(&locales, &code, &[]));
        assert!(text.contains("`a.only-commented` is never used"), "{text}");
        assert!(!text.contains("missing"), "{text}");
    }

    #[test]
    fn a_feature_folder_without_a_part_is_found() {
        let locales = [locale(r#"{"fields.list": "Fields"}"#)];
        let code = [source(
            "web/app/src/features/brand-new/Page.tsx",
            r#"t("fields.list")"#,
        )];
        let places: Vec<String> = evaluate(&locales, &code, &[])
            .into_iter()
            .filter_map(|finding| finding.location)
            .collect();
        assert_eq!(places, ["web/app/src/features/brand-new"]);
    }

    #[test]
    fn only_the_entry_of_a_feature_imports_its_part() {
        let locales = [locale(r#"{"fields.list": "Fields"}"#)];
        let entry = source(
            "web/app/src/features/fields/index.ts",
            r#"import "@bugaboo/messages/parts/fields";"#,
        );
        let page = source(
            "web/app/src/features/fields/Page.tsx",
            r#"import "@bugaboo/messages/parts/fields"; t("fields.list")"#,
        );
        let other = source(
            "web/app/src/features/people/index.ts",
            r#"import "@bugaboo/messages/parts/fields";"#,
        );
        let shared = source(
            "web/app/src/routes/Root.tsx",
            r#"import "@bugaboo/messages/parts/shared";"#,
        );
        let places: Vec<String> = evaluate(&locales, &[entry, page, other, shared], &[])
            .into_iter()
            .filter_map(|finding| finding.location)
            .collect();
        assert_eq!(
            places,
            [
                "web/app/src/features/fields/Page.tsx",
                "web/app/src/features/people/index.ts",
                "web/app/src/routes/Root.tsx",
            ]
        );
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
