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
const PARTS: [Part; 28] = [
    part!("access"),
    part!("account"),
    part!("admin"),
    part!("fields"),
    part!("files"),
    part!("follow"),
    part!("history"),
    part!("home"),
    part!("invitation"),
    part!("issue-types"),
    part!("issues"),
    part!("labels"),
    part!("layouts"),
    part!("many-changes"),
    part!("my-settings"),
    part!("notifications"),
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
    part!("workflows"),
];

/// The areas (first part of a key) that the browser shows: an allow-list, so a new area ships to the
/// browser only when it is added here (M1-F12). A key outside these areas is for the server and its
/// command line. A `t("…")` of the app with a key from another area is a type error.
const BROWSER_AREAS: [&str; 93] = [
    "access",
    "access-keys",
    "app",
    "appearance",
    "assignments",
    "backup",
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
    "files",
    "first-run",
    "focus",
    "followers",
    "groups",
    "handle",
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
    "layout-kind",
    "layouts",
    "list",
    "mail-server",
    "many-changes",
    "many-changes-skip",
    "many-changes-status",
    "markdown",
    "nav",
    "notification",
    "notifications",
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
    "preview",
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
    "window",
    "workflows",
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
    let mut paths = vec![
        TYPES_OUTPUT,
        ALL_OUTPUT,
        messages_rust::OUTPUT,
        messages_rust::SEARCH_OUTPUT,
    ];
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
        for literal in string_literals(&code, Quotes::DoubleAndSingle) {
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

/// Which quotes make a string literal.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Quotes {
    /// `"…"` only: Rust.
    Double,
    /// `"…"` and `'…'`: TypeScript.
    DoubleAndSingle,
}

/// Every string literal on a single line.
pub(crate) fn string_literals(code: &str, quotes: Quotes) -> Vec<String> {
    let mut literals = Vec::new();
    for line in code.lines() {
        let mut chars = line.chars();
        while let Some(c) = chars.next() {
            if c != '"' && (quotes == Quotes::Double || c != '\'') {
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
        Generated::new(
            messages_rust::SEARCH_OUTPUT,
            messages_rust::generate_search_keys(repo, &all_messages)?,
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
mod tests;
