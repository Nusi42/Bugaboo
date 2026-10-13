//! `cargo xtask gen messages`: typed message keys from `messages/en.json` (UX-07).

use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as _;

use anyhow::bail;

use super::Generated;
use super::icu::{ArgKind, placeholders};
use super::made_keys::made_keys;
use super::messages_rust;
use crate::repo::Repo;
use crate::source::{Kind, Source, read_sources, read_text};
use crate::web_code::strip_comments;

/// The source catalogue, relative to the repository root.
pub(crate) const SOURCE_LOCALE_PATH: &str = "messages/en.json";

/// The folder of the generated TypeScript (the types of all keys, and one file per part), and a
/// file in it: written once, so every path below follows.
macro_rules! generated_folder {
    () => {
        "web/packages/messages/src/generated/"
    };
    ($file:literal) => {
        concat!(generated_folder!(), $file)
    };
}
const GENERATED_FOLDER: &str = generated_folder!();
/// The types of every key (`MessageKey`, `MessageArguments`); no text, so it costs the browser nothing.
pub(crate) const TYPES_OUTPUT: &str = generated_folder!("types.ts");
/// Loads every part at once, for what runs outside the browser app (tests and end-to-end tests).
pub(crate) const ALL_OUTPUT: &str = generated_folder!("all.ts");

/// The part that the first page loads: the texts that more than one area shows, or that no area
/// names on its own (for instance the ones the server sends by key, or an enum makes).
pub(crate) const SHARED: &str = "shared";

/// How an area names its part when it imports it (the package export of `@bugaboo/messages`).
pub(crate) const PART_IMPORT: &str = "@bugaboo/messages/parts/";

/// The file that is the entry of a feature, which imports the part of that feature.
pub(crate) fn feature_entries(feature: &str) -> [String; 2] {
    [
        format!("{FEATURES_FOLDER}{feature}/index.ts"),
        format!("{FEATURES_FOLDER}{feature}/index.tsx"),
    ]
}

/// Where the app's features are; each is an area that loads on its own (ADR-0009).
pub(crate) const FEATURES_FOLDER: &str = "web/app/src/features/";

/// One part of the browser catalog: a file of texts that a chunk of the app brings along.
struct Part {
    /// `shared`, or the name of a feature folder.
    name: &'static str,
    /// Where its generated file is.
    path: &'static str,
}

/// Names a part and works out where its file is, so the name is written once.
macro_rules! part {
    ($name:literal) => {
        Part {
            name: $name,
            path: concat!(generated_folder!(), $name, ".ts"),
        }
    };
}

/// Every part, in the order of the files: `shared` and one per feature folder. A test compares the
/// list with the folders, so a new feature cannot be left out.
const PARTS: [Part; 25] = [
    part!("access"),
    part!("account"),
    part!("admin"),
    part!("bulk"),
    part!("fields"),
    part!("follow"),
    part!("history"),
    part!("home"),
    part!("invitation"),
    part!("issue-types"),
    part!("issues"),
    part!("labels"),
    part!("mentions"),
    part!("my-settings"),
    part!("people"),
    part!("projects"),
    part!("relation-kinds"),
    part!("relations"),
    part!("roles"),
    part!("settings"),
    part!("setup"),
    part!("shared"),
    part!("shell"),
    part!("sign-in"),
    part!("sign-up"),
];

/// The areas (first part of a key) that the browser shows: an allow-list, so a new area ships to the
/// browser only when it is added here (M1-F12). A key outside these areas is for the server and its
/// command line. A `t("…")` of the app with a key from another area is a type error.
const BROWSER_AREAS: [&str; 84] = [
    "access",
    "access-keys",
    "app",
    "appearance",
    "assignments",
    "backup",
    "bulk",
    "bulk-skip",
    "bulk-status",
    "check-access",
    "choice-color",
    "code-app",
    "comments",
    "common",
    "conflict",
    "data-folder",
    "editing",
    "error",
    "field",
    "field-example",
    "field-for",
    "field-level",
    "field-look",
    "field-permissions",
    "field-type",
    "fields",
    "first-run",
    "focus",
    "followers",
    "groups",
    "history",
    "home",
    "invitation",
    "invitations",
    "issue-field",
    "issue-icon",
    "issue-id-mode",
    "issue-ref",
    "issue-type",
    "issue-types",
    "issues",
    "labels",
    "list",
    "markdown",
    "mentions",
    "nav",
    "own",
    "page",
    "panel",
    "passkeys",
    "people",
    "people-kind",
    "people-level",
    "people-visibility",
    "permission",
    "permission-topic",
    "person",
    "priority",
    "projects",
    "related-issues",
    "relation-kinds",
    "roles",
    "save",
    "search",
    "second-step",
    "security",
    "security-log",
    "server-status",
    "sessions",
    "set-up-second-step",
    "setting",
    "setup",
    "shell",
    "shortcut",
    "shortcuts",
    "sign-in",
    "sign-up",
    "sign-up-policy",
    "starting-role",
    "status",
    "status-group",
    "sub-issues",
    "theme",
    "validation",
];

/// Whether a message is shown by the server only, and so is left out of the browser's catalog.
pub(crate) fn is_server_only(key: &str) -> bool {
    key.split_once('.')
        .is_none_or(|(area, _)| !BROWSER_AREAS.contains(&area))
}

/// The line width of oxfmt (its default).
const PRINT_WIDTH: usize = 100;

const TS_HEADER: &str =
    "// Generated by `cargo xtask gen messages` from messages/en.json. Do not edit by hand.
";

/// The text of the catalog on disk.
pub(crate) fn source_text(repo: &Repo) -> anyhow::Result<String> {
    read_text(repo, SOURCE_LOCALE_PATH)
}

/// The messages of the catalog on disk, in file order.
pub(crate) fn source_pairs(repo: &Repo) -> anyhow::Result<Vec<(String, String)>> {
    parse_pairs(&source_text(repo)?)
}

/// The paths [`generate`] writes.
pub(crate) fn output_paths() -> Vec<&'static str> {
    let mut paths = vec![TYPES_OUTPUT, ALL_OUTPUT, messages_rust::OUTPUT];
    paths.extend(PARTS.iter().map(|part| part.path));
    paths
}

/// Whether `path` is a file of the generated TypeScript catalog. Its texts are the catalog itself,
/// so they never count as a use of a key.
pub(crate) fn is_generated_catalog(path: &str) -> bool {
    path.starts_with(GENERATED_FOLDER)
}

/// Whether a file is a test or generated, so that its strings say nothing about what the product
/// shows.
pub(crate) fn is_test_or_generated(path: &str) -> bool {
    const MARKERS: [&str; 5] = [".test.", ".spec.", "/e2e/", "/tests/", "generated."];
    is_generated_catalog(path) || MARKERS.iter().any(|marker| path.contains(marker))
}

/// The feature a source file of the app belongs to, if it is inside one.
fn feature_of(path: &str) -> Option<&'static str> {
    let (name, _) = path.strip_prefix(FEATURES_FOLDER)?.split_once('/')?;
    PARTS
        .iter()
        .map(|part| part.name)
        .find(|part| *part == name && *part != SHARED)
}

/// The feature folders among `paths` that have no part, so their texts would stay on the first
/// page. A new feature is added to [`PARTS`].
pub(crate) fn features_without_part<'a>(paths: impl Iterator<Item = &'a str>) -> Vec<&'a str> {
    let mut missing: Vec<&str> = paths
        .filter_map(|path| path.strip_prefix(FEATURES_FOLDER)?.split_once('/'))
        .map(|(name, _)| name)
        .filter(|name| !PARTS.iter().any(|part| part.name == *name))
        .collect();
    missing.sort_unstable();
    missing.dedup();
    missing
}

/// Who names a key in the code.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Users {
    /// Only this feature does.
    Feature(&'static str),
    /// Code outside the features, or more than one feature, or a feature the list lacks.
    Several,
}

/// The part of every key of the browser catalog, from one rule: a key goes to the part of a
/// feature when that feature is the only code that names the key, and to `shared` otherwise. Code
/// names a key as a string, or builds it from a template (`` `issue-field.${code}` ``): then it
/// names every key that starts with the text of the template. A key that no code names is
/// `shared` too: the server sends it by key, or a word of an enum makes it, and then any page may
/// show it. `made` are the keys that the words of the enums make: the server sends them and shared
/// code looks them up, so they are `shared` whatever the code says. The first page loads `shared`
/// only, and a feature brings its own part along with its chunk.
pub(crate) fn parts_of<'a>(
    keys: impl IntoIterator<Item = &'a str>,
    sources: &[Source],
    made: &[&str],
) -> BTreeMap<&'a str, &'static str> {
    let keys: Vec<&str> = keys.into_iter().collect();
    let known: HashSet<&str> = keys.iter().copied().collect();
    let mut users: BTreeMap<&str, Users> = BTreeMap::new();
    let mut name = |key: &'a str, here: Users| {
        let merged = match users.get(key) {
            Some(before) if *before != here => Users::Several,
            _ => here,
        };
        users.insert(key, merged);
    };
    for (path, text) in sources {
        if is_test_or_generated(path) {
            continue;
        }
        let here = feature_of(path).map_or(Users::Several, Users::Feature);
        let code = strip_comments(text);
        for literal in string_literals(&code, false) {
            if let Some(key) = known.get(literal.as_str()) {
                name(key, here);
            }
        }
        for prefix in template_prefixes(&code) {
            for key in keys.iter().filter(|key| key.starts_with(prefix.as_str())) {
                name(key, here);
            }
        }
    }
    keys.into_iter()
        .map(|key| {
            let part = match users.get(key) {
                Some(Users::Feature(feature)) if !made.contains(&key) => feature,
                _ => SHARED,
            };
            (key, part)
        })
        .collect()
}

/// The beginnings of keys that the code builds at run time: `` `prefix.${…}` `` and
/// `"prefix." + …`. A prefix ends with a dot and is made of the letters of a key.
pub(crate) fn template_prefixes(code: &str) -> Vec<String> {
    let of_a_key = |text: &str| {
        text.ends_with('.')
            && text
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.')
    };
    let mut prefixes = Vec::new();
    for (at, _) in code.match_indices("${") {
        let before = &code[..at];
        if let Some(start) = before.rfind('`')
            && of_a_key(&before[start + 1..])
        {
            prefixes.push(before[start + 1..].to_owned());
        }
    }
    for quote in ['"', '\''] {
        for (at, _) in code.match_indices(&format!(".{quote}")) {
            let after = code[at + 2..].trim_start_matches([' ', '\t']);
            let before = &code[..=at];
            if after.starts_with('+')
                && let Some(start) = before.rfind(quote)
                && of_a_key(&before[start + 1..])
            {
                prefixes.push(before[start + 1..].to_owned());
            }
        }
    }
    prefixes
}

/// Every `"…"` (and, for TypeScript, `'…'`) on a single line.
pub(crate) fn string_literals(code: &str, double_only: bool) -> Vec<String> {
    let mut literals = Vec::new();
    for line in code.lines() {
        let mut chars = line.chars();
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

/// The TypeScript and the Rust output, from the catalog on disk and the code that names its keys.
pub(crate) fn outputs(repo: &Repo) -> anyhow::Result<Vec<Generated>> {
    generate(repo, &source_text(repo)?)
}

/// The TypeScript and the Rust output from the one catalog.
pub(crate) fn generate(repo: &Repo, catalog_text: &str) -> anyhow::Result<Vec<Generated>> {
    let sources = read_sources(repo, None, Kind::Web)?;
    generate_from(repo, catalog_text, &sources)
}

/// Like [`generate`], with the code that names the keys given.
fn generate_from(
    repo: &Repo,
    catalog_text: &str,
    sources: &[Source],
) -> anyhow::Result<Vec<Generated>> {
    let all_messages: Vec<(String, String)> = read_catalog(catalog_text)?;
    // The browser gets only what it shows (M1-F12): the server's own words never ship to it, and
    // a `t("config.…")` in the app is a type error.
    let messages: Vec<(String, String)> = all_messages
        .iter()
        .filter(|(key, _)| !is_server_only(key))
        .cloned()
        .collect();
    let made = made_keys();
    let made: Vec<&str> = made.iter().map(|made| made.key.as_str()).collect();
    let part_of = parts_of(messages.iter().map(|(key, _)| key.as_str()), sources, &made);
    let mut generated = vec![
        Generated::new(TYPES_OUTPUT, types_file(&messages)?),
        Generated::new(ALL_OUTPUT, all_file()),
        Generated::new(
            messages_rust::OUTPUT,
            messages_rust::generate(repo, &all_messages)?,
        ),
    ];
    for part in &PARTS {
        let own: Vec<&(String, String)> = messages
            .iter()
            .filter(|(key, _)| part_of.get(key.as_str()) == Some(&part.name))
            .collect();
        generated.push(Generated::new(part.path, part_file(&own)?));
    }
    Ok(generated)
}

/// The name of a part as a TypeScript identifier: `issue-types` is `issueTypes`.
fn identifier(part: &str) -> String {
    let mut out = String::new();
    let mut upper = false;
    for character in part.chars() {
        if character == '-' {
            upper = true;
        } else if upper {
            out.extend(character.to_uppercase());
            upper = false;
        } else {
            out.push(character);
        }
    }
    out
}

/// A file of one part: its texts, registered when the file is loaded.
fn part_file(messages: &[&(String, String)]) -> anyhow::Result<String> {
    let mut out = String::from(TS_HEADER);
    out.push_str("\nimport { register } from \"../registry.ts\";\n\n");
    if messages.is_empty() {
        out.push_str("export const messages = {} as const;\n");
    } else {
        out.push_str("export const messages = {\n");
        for (key, text) in messages {
            let (key, text) = (json_string(key)?, json_string(text)?);
            // The formatter (oxfmt) moves a value that does not fit the line to the next one; the
            // generated file must already look like that, or FMT-TS and DOC-GENERATED disagree.
            let line = format!("  {key}: {text},");
            if line.chars().count() > PRINT_WIDTH {
                let _ = writeln!(
                    out,
                    "  {key}:
    {text},"
                );
            } else {
                let _ = writeln!(out, "{line}");
            }
        }
        out.push_str("} as const;\n");
    }
    out.push_str("\nregister(messages);\n");
    Ok(out)
}

/// The file that loads every part, and names each one's texts.
fn all_file() -> String {
    let mut out = String::from(TS_HEADER);
    out.push('\n');
    for part in &PARTS {
        let _ = writeln!(
            out,
            "export {{ messages as {} }} from \"./{}.ts\";",
            identifier(part.name),
            part.name
        );
    }
    out
}

/// The types of all keys, from the parts: `MessageKey` and the values each message needs.
fn types_file(messages: &[(String, String)]) -> anyhow::Result<String> {
    let mut out = String::from(TS_HEADER);
    out.push('\n');
    for part in &PARTS {
        let _ = writeln!(
            out,
            "import type {{ messages as {} }} from \"./{}.ts\";",
            identifier(part.name),
            part.name
        );
    }
    out.push_str("\nexport type MessageKey =\n");
    for (index, part) in PARTS.iter().enumerate() {
        let end = if index + 1 == PARTS.len() { ";" } else { "" };
        let _ = writeln!(out, "  | keyof typeof {}{end}", identifier(part.name));
    }
    out.push('\n');
    let mut with_values = BTreeMap::new();
    for (key, text) in messages {
        let found = placeholders(text).map_err(|problem| anyhow::anyhow!("{key}: {problem}"))?;
        if !found.is_empty() {
            with_values.insert(key.clone(), found);
        }
    }
    if with_values.is_empty() {
        out.push_str("export type MessageArguments = Record<never, never>;\n");
    } else {
        out.push_str("export interface MessageArguments {\n");
        for (key, found) in &with_values {
            let fields: Vec<String> = found
                .iter()
                .map(|(name, kind)| format!("{name}: {}", ts_type(*kind)))
                .collect();
            let key = json_string(key)?;
            let line = format!("  {key}: {{ {} }};", fields.join("; "));
            if line.chars().count() > PRINT_WIDTH {
                let _ = writeln!(out, "  {key}: {{");
                for field in &fields {
                    let _ = writeln!(out, "    {field};");
                }
                out.push_str("  };\n");
            } else {
                let _ = writeln!(out, "{line}");
            }
        }
        out.push_str("}\n");
    }
    Ok(out)
}

/// The catalogue as ordered key and text pairs. Duplicate keys are an error here too.
pub(crate) fn read_catalog(text: &str) -> anyhow::Result<Vec<(String, String)>> {
    let pairs = parse_pairs(text)?;
    let mut seen = HashSet::new();
    for (key, _) in &pairs {
        if !seen.insert(key.as_str()) {
            bail!("the key `{key}` appears twice");
        }
    }
    Ok(pairs)
}

/// Reads a flat JSON object of text values, keeping order and duplicates.
pub(crate) fn parse_pairs(text: &str) -> anyhow::Result<Vec<(String, String)>> {
    struct Pairs(Vec<(String, String)>);
    struct PairsVisitor;
    impl<'de> serde::de::Visitor<'de> for PairsVisitor {
        type Value = Pairs;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("an object whose values are all text")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Pairs, A::Error> {
            let mut pairs = Vec::new();
            while let Some(entry) = map.next_entry::<String, String>()? {
                pairs.push(entry);
            }
            Ok(Pairs(pairs))
        }
    }
    impl<'de> serde::Deserialize<'de> for Pairs {
        fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            deserializer.deserialize_map(PairsVisitor)
        }
    }
    Ok(serde_json::from_str::<Pairs>(text)?.0)
}

fn ts_type(kind: ArgKind) -> &'static str {
    match kind {
        ArgKind::Text => "string | number",
        ArgKind::Select => "string",
        ArgKind::Number => "number",
        ArgKind::Date => "Date | number",
    }
}

/// The text as the formatter writes a string: in double quotes, or in single quotes when it holds
/// more double quotes than single ones (oxfmt follows Prettier here), so the generated file is
/// already what the formatter would make of it.
fn json_string(text: &str) -> anyhow::Result<String> {
    const SINGLE: char = '\'';
    const BACKSLASH: char = '\\';
    let double = serde_json::to_string(text)?;
    if text.matches('"').count() <= text.matches(SINGLE).count() {
        return Ok(double);
    }
    let inner = double
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .ok_or_else(|| anyhow::anyhow!("a JSON string is not in quotes"))?;
    let mut single = String::from(SINGLE);
    let mut chars = inner.chars();
    while let Some(character) = chars.next() {
        match character {
            BACKSLASH => match chars.next() {
                Some('"') => single.push('"'),
                Some(escaped) => {
                    single.push(BACKSLASH);
                    single.push(escaped);
                }
                None => single.push(BACKSLASH),
            },
            SINGLE => {
                single.push(BACKSLASH);
                single.push(SINGLE);
            }
            other => single.push(other),
        }
    }
    single.push(SINGLE);
    Ok(single)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_string_with_more_double_than_single_quotes_is_written_in_single_quotes() {
        assert_eq!(json_string("plain").unwrap(), "\"plain\"");
        assert_eq!(json_string("it's").unwrap(), "\"it's\"");
        assert_eq!(
            json_string("\"… is editing\"").unwrap(),
            "'\"… is editing\"'"
        );
        assert_eq!(
            json_string("\"a\" it's \"b\"").unwrap(),
            "'\"a\" it\\'s \"b\"'"
        );
        assert_eq!(json_string("\"a\\b\"").unwrap(), "'\"a\\\\b\"'");
    }

    #[test]
    fn generated_file_is_up_to_date() {
        let repo = Repo::discover().unwrap();
        let text = std::fs::read_to_string(repo.path(SOURCE_LOCALE_PATH)).unwrap();
        for output in generate(&repo, &text).unwrap() {
            let on_disk = std::fs::read_to_string(repo.path(output.path)).unwrap();
            assert_eq!(
                crate::source::normalize(&on_disk),
                output.content,
                "{} is stale; run `cargo xtask gen messages`",
                output.path
            );
        }
    }

    /// The text of the generated file at `path`.
    fn file<'a>(output: &'a [Generated], path: &str) -> &'a str {
        &output
            .iter()
            .find(|generated| generated.path == path)
            .unwrap_or_else(|| panic!("{path} is not generated"))
            .content
    }

    /// The text of every part of the browser catalog together.
    fn all_parts(output: &[Generated]) -> String {
        PARTS
            .iter()
            .map(|part| file(output, part.path))
            .collect::<Vec<_>>()
            .join(
                "
",
            )
    }

    fn code(path: &str, text: &str) -> Source {
        (path.to_owned(), text.to_owned())
    }

    #[test]
    fn placeholders_become_typed_arguments() {
        let repo = Repo::discover().unwrap();
        let output = generate(&repo, r#"{"home.b": "Hi {name}", "home.c": "Plain"}"#).unwrap();
        let types = file(&output, TYPES_OUTPUT);
        assert!(types.contains("\"home.b\": { name: string | number };"));
        assert!(!types.contains("\"home.c\": {"));
        assert!(
            file(&output, messages_rust::OUTPUT)
                .contains("pub fn home_b(name: impl std::fmt::Display)")
        );
    }

    #[test]
    fn the_browser_catalog_leaves_out_what_only_the_server_shows() {
        let repo = Repo::discover().unwrap();
        let text = r#"{"error.a.b": "Oops", "config.x.y": "Server", "server.listening": "Up {address}", "home.title": "Hi"}"#;
        let output = generate(&repo, text).unwrap();
        let browser = format!("{}{}", all_parts(&output), file(&output, TYPES_OUTPUT));
        assert!(browser.contains("\"error.a.b\""));
        assert!(browser.contains("\"home.title\""));
        assert!(!browser.contains("config.x.y"));
        assert!(!browser.contains("server.listening"));
        let rust = file(&output, messages_rust::OUTPUT);
        assert!(rust.contains("CONFIG_X_Y"));
        assert!(rust.contains("server_listening"));
    }

    #[test]
    fn a_new_area_stays_out_of_the_browser_until_it_is_listed() {
        let repo = Repo::discover().unwrap();
        let text = r#"{"brand-new.title": "New", "home.title": "Hi"}"#;
        let output = generate(&repo, text).unwrap();
        let browser = all_parts(&output);
        assert!(!browser.contains("brand-new.title"));
        assert!(browser.contains("home.title"));
        assert!(file(&output, messages_rust::OUTPUT).contains("BRAND_NEW_TITLE"));
    }

    #[test]
    fn a_key_that_one_feature_alone_names_goes_to_the_part_of_that_feature() {
        let sources = [
            code(
                "web/app/src/features/fields/FieldsPage.tsx",
                r#"t("fields.title"); t("save.label"); t("nav.fields")"#,
            ),
            code(
                "web/app/src/features/people/List.tsx",
                r#"t("save.label"); t("people.title")"#,
            ),
            code("web/app/src/shell.tsx", r#"t("nav.fields")"#),
        ];
        let parts = parts_of(
            ["fields.title", "save.label", "nav.fields", "people.title"],
            &sources,
            &[],
        );
        assert_eq!(parts["fields.title"], "fields");
        assert_eq!(parts["people.title"], "people");
        // Two features, and a feature with code outside, both need the text on the first page.
        assert_eq!(parts["save.label"], "shared");
        assert_eq!(parts["nav.fields"], "shared");
    }

    #[test]
    fn a_key_that_no_code_names_is_on_the_first_page() {
        let sources = [code(
            "web/app/src/features/fields/A.tsx",
            r#"t("fields.a")"#,
        )];
        let parts = parts_of(["fields.a", "error.resource.not-found"], &sources, &[]);
        assert_eq!(parts["error.resource.not-found"], "shared");
    }

    #[test]
    fn tests_and_the_generated_catalog_do_not_decide_where_a_key_goes() {
        let sources = [
            code("web/app/src/features/fields/A.tsx", r#"t("fields.a")"#),
            code("web/app/src/shared/A.test.ts", r#"t("fields.a")"#),
            code("web/app/e2e/a.spec.ts", r#"t("fields.a")"#),
            code(generated_folder!("shared.ts"), r#""fields.a": "A","#),
        ];
        assert_eq!(parts_of(["fields.a"], &sources, &[])["fields.a"], "fields");
    }

    #[test]
    fn a_folder_that_the_list_of_features_lacks_counts_as_shared_code() {
        let sources = [code(
            "web/app/src/features/brand-new/A.tsx",
            r#"t("fields.a")"#,
        )];
        assert_eq!(parts_of(["fields.a"], &sources, &[])["fields.a"], "shared");
    }

    #[test]
    fn a_key_that_a_word_of_an_enum_makes_is_shared_even_if_one_feature_names_it() {
        let sources = [code(
            "web/app/src/features/issues/A.tsx",
            r#"t("issue-field.title"); t("issues.list")"#,
        )];
        let parts = parts_of(
            ["issue-field.title", "issues.list"],
            &sources,
            &["issue-field.title"],
        );
        assert_eq!(parts["issue-field.title"], "shared");
        assert_eq!(parts["issues.list"], "issues");
    }

    #[test]
    fn the_keys_of_the_enums_are_shared_in_the_repository() {
        let repo = Repo::discover().unwrap();
        let text = read_text(&repo, SOURCE_LOCALE_PATH).unwrap();
        let output = generate(&repo, &text).unwrap();
        let shared = file(&output, generated_folder!("shared.ts"));
        for made in made_keys() {
            let quoted = json_string(made.key.as_str()).unwrap();
            if !is_server_only(made.key.as_str()) {
                assert!(shared.contains(&format!("  {quoted}:")), "{}", made.key);
            }
        }
    }

    #[test]
    fn a_template_of_shared_code_puts_every_key_it_can_build_in_shared() {
        let sources = [
            code(
                "web/app/src/shared/names.ts",
                "t(`issue-field.${code}`); const k = \"markdown.\" + name;",
            ),
            code(
                "web/app/src/features/issues/A.tsx",
                r#"t("issue-field.title")"#,
            ),
            code("web/app/src/features/people/A.tsx", r#"t("markdown.bold")"#),
        ];
        let keys = ["issue-field.title", "issue-field.priority", "markdown.bold"];
        let parts = parts_of(keys, &sources, &[]);
        assert_eq!(parts["issue-field.title"], "shared");
        assert_eq!(parts["issue-field.priority"], "shared");
        assert_eq!(parts["markdown.bold"], "shared");
    }

    #[test]
    fn a_template_inside_one_feature_leaves_its_keys_in_that_feature() {
        let sources = [
            code(
                "web/app/src/features/fields/A.tsx",
                "t(`field-type.${kind}`); t('field-type.text')",
            ),
            code("web/app/src/features/people/A.tsx", r#"t("people.title")"#),
        ];
        let parts = parts_of(
            ["field-type.text", "field-type.date", "people.title"],
            &sources,
            &[],
        );
        assert_eq!(parts["field-type.text"], "fields");
        assert_eq!(parts["field-type.date"], "fields");
        assert_eq!(parts["people.title"], "people");
        // The same template in a second feature makes them shared.
        let more = [
            code(
                "web/app/src/features/fields/A.tsx",
                "t(`field-type.${kind}`)",
            ),
            code(
                "web/app/src/features/people/B.tsx",
                "t(`field-type.${kind}`)",
            ),
        ];
        assert_eq!(
            parts_of(["field-type.date"], &more, &[])["field-type.date"],
            "shared"
        );
    }

    #[test]
    fn template_prefixes_are_the_text_before_the_dollar_brace_or_the_plus() {
        let found = template_prefixes(
            "t(`a.b-c.${x}`); k(\"d.\" + y); k('e.f.' +z); `${x}.no`; `plain ${x}`; \"g.h\" + 1",
        );
        assert_eq!(found, ["a.b-c.", "d.", "e.f."]);
    }

    #[test]
    fn a_key_in_a_comment_is_not_a_use() {
        let sources = [
            code(
                "web/app/src/features/fields/A.tsx",
                "t(\"fields.a\"); // t(\"people.b\")\n/* t(`people.${c}`)\n t('people.b') */",
            ),
            code("web/app/src/features/people/A.tsx", r#"t("people.b")"#),
        ];
        let parts = parts_of(["fields.a", "people.b"], &sources, &[]);
        assert_eq!(parts["people.b"], "people");
    }

    #[test]
    fn every_key_is_in_exactly_one_part() {
        let repo = Repo::discover().unwrap();
        let text = read_text(&repo, SOURCE_LOCALE_PATH).unwrap();
        let output = generate(&repo, &text).unwrap();
        let keys: Vec<String> = read_catalog(&text)
            .unwrap()
            .into_iter()
            .map(|(key, _)| key)
            .filter(|key| !is_server_only(key))
            .collect();
        assert!(!keys.is_empty());
        for key in keys {
            let quoted = json_string(&key).unwrap();
            let homes: Vec<&str> = PARTS
                .iter()
                .filter(|part| {
                    file(&output, part.path)
                        .lines()
                        .any(|line| line.trim_start().starts_with(&format!("{quoted}:")))
                })
                .map(|part| part.name)
                .collect();
            assert_eq!(homes.len(), 1, "{key} is in {homes:?}");
        }
    }

    #[test]
    fn there_is_a_part_for_every_feature_folder_and_for_nothing_else() {
        let repo = Repo::discover().unwrap();
        let mut folders: Vec<String> = std::fs::read_dir(repo.path(FEATURES_FOLDER))
            .unwrap()
            .map(|entry| entry.unwrap())
            .filter(|entry| entry.path().is_dir())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        folders.push(SHARED.to_owned());
        folders.sort();
        let parts: Vec<&str> = PARTS.iter().map(|part| part.name).collect();
        assert_eq!(
            folders, parts,
            "PARTS must list `shared` and every folder of {FEATURES_FOLDER}, in this order"
        );
    }

    #[test]
    fn every_part_has_its_file_in_the_generated_folder() {
        for part in &PARTS {
            assert_eq!(
                part.path,
                format!("{GENERATED_FOLDER}{}.ts", part.name),
                "{}",
                part.name
            );
        }
    }

    #[test]
    fn duplicate_keys_are_rejected() {
        assert!(generate(&Repo::discover().unwrap(), r#"{"a.b": "x", "a.b": "y"}"#).is_err());
    }
}
