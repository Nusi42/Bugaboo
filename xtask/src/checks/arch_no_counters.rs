//! ARCH-NO-COUNTERS: no public identifier counts up, and no answer holds a number that counts
//! more than the viewer may see (ADR-0021, PRIV-13, architecture §7 *No counts that leak*).
//!
//! Two rules:
//!
//! - **The API description.** A property of an answer that is a count (`total`, `count`,
//!   `…_count`, `number_of_…`, or a whole number whose description says it counts) fails unless
//!   `tools/checks/counter-exemptions.toml` names the route and the property with a reason that
//!   says why the number holds only what the viewer may see. Lists page with `next_cursor`; they
//!   never answer a total. An entry that matches no property of the description fails, so the
//!   file cannot go stale.
//! - **The code.** What makes an identifier or a label count up fails in product code: a column
//!   or a sequence the database counts for (`AUTOINCREMENT`, `SERIAL`, `IDENTITY`,
//!   `CREATE SEQUENCE`, `nextval`, `auto_increment()`, `last_insert_rowid`) and a number made by
//!   adding one to a count (`count + 1`, `counter + 1`, `.count() + 1`, `next_number`). Counting
//!   issue IDs (ADR-0021, M3) is the one place that may, and it is named in the same file with
//!   its folder. IDs, access-key prefixes and the labels of removed people are random.
//!
//! Comments, string and character literals do not count in code, and neither does code behind
//! `#[cfg(test)]`; the SQL words are read in the strings of migrations, because that is where
//! they are written.

use std::collections::BTreeSet;

use anyhow::Context as _;
use serde::Deserialize;
use serde_json::Value;

use super::{Check, Context, Finding};
use crate::generate::api::SPEC_PATH;
use crate::rust_source::{Source, read_product_sources, squeezed, strip, without_tests};

/// Where the exceptions are written down, each with its reason.
const EXEMPTIONS_PATH: &str = "tools/checks/counter-exemptions.toml";
/// The words of a property name that make it a count.
const COUNT_WORDS: [&str; 6] = ["total", "totals", "count", "counts", "tally", "sum"];
/// The words of the description of a whole number that say the number counts, and the phrases.
/// (`Counts up with every change`, a version, is no count of what a viewer may see.)
const COUNT_DESCRIPTION_WORDS: [&str; 4] = ["counted", "counting", "total", "totals"];
const COUNT_PHRASES: [&str; 3] = ["how many", "number of", "the count"];
/// How deep an answer schema is followed.
const DEPTH_MAX: usize = 12;
/// The SQL words that make the database count for a column (matched in capitals, as written).
const SQL_COUNTERS: [&str; 8] = [
    "AUTOINCREMENT",
    "AUTO_INCREMENT",
    "BIGSERIAL",
    "SMALLSERIAL",
    " SERIAL ",
    "CREATE SEQUENCE",
    "AS IDENTITY",
    "NEXTVAL(",
];
/// What in code makes a number count up (matched in the code with its white space gone).
const CODE_COUNTERS: [&str; 6] = [
    "count+1",
    "counter+1",
    ".count()+1",
    "next_number",
    ".auto_increment(",
    "last_insert_rowid",
];

pub(crate) struct ArchNoCounters;

impl Check for ArchNoCounters {
    fn id(&self) -> &'static str {
        "ARCH-NO-COUNTERS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let text = std::fs::read_to_string(context.repo.path(SPEC_PATH))
            .with_context(|| format!("could not read {SPEC_PATH}; run `cargo xtask gen api`"))?;
        let spec: Value = serde_json::from_str(&text)
            .with_context(|| format!("{SPEC_PATH} is not valid JSON"))?;
        let exemptions = std::fs::read_to_string(context.repo.path(EXEMPTIONS_PATH))
            .with_context(|| format!("could not read {EXEMPTIONS_PATH}"))?;
        let exemptions: Exemptions = toml::from_str(&exemptions)
            .with_context(|| format!("{EXEMPTIONS_PATH} is not valid"))?;
        let sources = read_product_sources(context.repo)?;
        let mut findings = evaluate_spec(&spec, &exemptions.exempt);
        findings.extend(evaluate_code(&sources, &exemptions.counting));
        Ok(findings)
    }
}

/// The file of exceptions.
#[derive(Deserialize)]
struct Exemptions {
    /// Numbers in answers that count only what the viewer may see.
    #[serde(default)]
    exempt: Vec<Exemption>,
    /// Folders of code that may count: the counting issue IDs.
    #[serde(default)]
    counting: Vec<Counting>,
}

/// One number of an answer: the route (`GET /api/v1/...`), the property and why it is safe.
#[derive(Deserialize)]
struct Exemption {
    route: String,
    property: String,
    reason: String,
}

/// A folder of code that may count, and why.
#[derive(Deserialize)]
struct Counting {
    folder: String,
    reason: String,
}

/// Follows `$ref`s that point into the document itself.
fn resolve<'a>(spec: &'a Value, value: &'a Value) -> &'a Value {
    value
        .get("$ref")
        .and_then(Value::as_str)
        .and_then(|reference| reference.strip_prefix('#'))
        .and_then(|pointer| spec.pointer(pointer))
        .unwrap_or(value)
}

/// Whether a name is a count: one of its words (split at `_`, `-` and capitals) says so, or it
/// starts with `number of`.
fn is_count_name(name: &str) -> bool {
    let mut words: Vec<String> = Vec::new();
    let mut word = String::new();
    for character in name.chars() {
        if character == '_' || character == '-' {
            words.push(std::mem::take(&mut word));
        } else if character.is_uppercase() && !word.is_empty() {
            words.push(std::mem::take(&mut word));
            word.extend(character.to_lowercase());
        } else {
            word.extend(character.to_lowercase());
        }
    }
    words.push(word);
    words
        .iter()
        .any(|word| COUNT_WORDS.contains(&word.as_str()))
        || words.first().is_some_and(|word| word == "number")
            && words.get(1).is_some_and(|word| word == "of")
}

/// Whether the schema of a property is a whole number whose description says it counts.
fn describes_a_count(property: &Value) -> bool {
    let kind = property.get("type");
    let is_number = |kind: &Value| matches!(kind.as_str(), Some("integer" | "number"));
    let numeric = kind.is_some_and(|kind| {
        is_number(kind)
            || kind
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(is_number))
    });
    let description = property
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_lowercase();
    let says_so = description
        .split(|c: char| !c.is_alphabetic())
        .any(|word| COUNT_DESCRIPTION_WORDS.contains(&word))
        || COUNT_PHRASES
            .iter()
            .any(|phrase| description.contains(phrase));
    numeric && says_so
}

/// Every property of an answer, at any depth (through `$ref`s, items, combined schemas), with its
/// schema; a schema is looked at once.
fn properties<'a>(
    spec: &'a Value,
    schema: &'a Value,
    seen: &mut BTreeSet<String>,
    depth: usize,
    found: &mut Vec<(String, &'a Value)>,
) {
    if depth > DEPTH_MAX {
        return;
    }
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str)
        && !seen.insert(reference.to_owned())
    {
        return;
    }
    let schema = resolve(spec, schema);
    if let Some(members) = schema.get("properties").and_then(Value::as_object) {
        for (name, property) in members {
            found.push((name.clone(), resolve(spec, property)));
            properties(spec, property, seen, depth + 1, found);
        }
    }
    for key in ["items", "additionalProperties"] {
        if let Some(inner) = schema.get(key).filter(|inner| inner.is_object()) {
            properties(spec, inner, seen, depth + 1, found);
        }
    }
    for key in ["anyOf", "oneOf", "allOf"] {
        for member in schema
            .get(key)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            properties(spec, member, seen, depth + 1, found);
        }
    }
}

fn evaluate_spec(spec: &Value, exemptions: &[Exemption]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let Some(paths) = spec.get("paths").and_then(Value::as_object) else {
        return vec![Finding::at(SPEC_PATH, "The document has no `paths`.")];
    };
    let mut present: BTreeSet<(String, String)> = BTreeSet::new();
    let mut used: BTreeSet<(String, String)> = BTreeSet::new();
    for (path, item) in paths {
        let Some(item) = item.as_object() else {
            continue;
        };
        for (method, operation) in item {
            let Some(responses) = operation.get("responses").and_then(Value::as_object) else {
                continue;
            };
            let route = format!("{} {path}", method.to_uppercase());
            let mut found = Vec::new();
            let mut seen = BTreeSet::new();
            for response in responses.values() {
                let response = resolve(spec, response);
                for media in response
                    .get("content")
                    .and_then(Value::as_object)
                    .into_iter()
                    .flat_map(|content| content.values())
                {
                    if let Some(schema) = media.get("schema") {
                        properties(spec, schema, &mut seen, 0, &mut found);
                    }
                }
            }
            let mut reported = BTreeSet::new();
            for (name, property) in found {
                present.insert((route.clone(), name.clone()));
                if !(is_count_name(&name) || describes_a_count(property)) {
                    continue;
                }
                let key = (route.clone(), name.clone());
                if exemptions
                    .iter()
                    .any(|exemption| exemption.route == route && exemption.property == name)
                {
                    used.insert(key);
                } else if reported.insert(name.clone()) {
                    findings.push(Finding::at(
                        &route,
                        format!(
                            "The answer holds a count, `{name}`. A number that counts can include \
                             what the viewer may not see: page with `next_cursor` instead of \
                             answering a total, or name the property in {EXEMPTIONS_PATH} with a \
                             reason that says why it counts only what the viewer may see."
                        ),
                    ));
                }
            }
        }
    }
    for exemption in exemptions {
        let key = (exemption.route.clone(), exemption.property.clone());
        if !present.contains(&key) {
            findings.push(Finding::at(
                EXEMPTIONS_PATH,
                format!(
                    "The entry `{}` in {} matches no property of an answer. Remove it.",
                    exemption.property, exemption.route
                ),
            ));
        }
        if !exemption.reason.to_lowercase().contains("see") || exemption.reason.trim().len() < 30 {
            findings.push(Finding::at(
                EXEMPTIONS_PATH,
                format!(
                    "The entry `{}` in {} needs a reason that says why the number holds only \
                     what the viewer may see.",
                    exemption.property, exemption.route
                ),
            ));
        }
    }
    findings
}

fn evaluate_code(sources: &[Source], counting: &[Counting]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut used: BTreeSet<usize> = BTreeSet::new();
    for (path, text) in sources {
        let allowing = counting
            .iter()
            .position(|entry| path.starts_with(&entry.folder));
        let code = strip(&without_tests(text));
        // The words of SQL are written in strings, which `strip` blanks: read them from the text
        // with the comments and tests taken away only.
        let sql = without_tests(text);
        let (squeezed, lines) = squeezed(&code);
        let mut hits: Vec<(usize, &str)> = Vec::new();
        for pattern in CODE_COUNTERS {
            for (at, _) in squeezed.match_indices(pattern) {
                hits.push((lines.get(at).copied().unwrap_or(1), pattern));
            }
        }
        for (index, line) in sql.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            let upper = line.to_uppercase();
            for pattern in SQL_COUNTERS {
                if upper.contains(pattern) {
                    hits.push((index + 1, pattern));
                }
            }
        }
        for (line, pattern) in hits {
            if let Some(index) = allowing {
                used.insert(index);
                continue;
            }
            findings.push(Finding::at(
                format!("{path}:{line}"),
                format!(
                    "`{pattern}` makes a number that counts up. A public identifier or label \
                     that counts tells how many there are and how fast it grows (ADR-0021, \
                     PRIV-13): use a random one (`IdSource`). Only the counting issue IDs of a \
                     project may count, and their folder is named in {EXEMPTIONS_PATH}."
                ),
            ));
        }
    }
    for (index, entry) in counting.iter().enumerate() {
        if !used.contains(&index) {
            findings.push(Finding::at(
                EXEMPTIONS_PATH,
                format!(
                    "The folder `{}` has nothing that counts any more. Remove the entry.",
                    entry.folder
                ),
            ));
        }
        if entry.reason.trim().len() < 20 {
            findings.push(Finding::at(
                EXEMPTIONS_PATH,
                format!("The folder `{}` needs a reason.", entry.folder),
            ));
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn exemption(route: &str, property: &str, reason: &str) -> Exemption {
        Exemption {
            route: route.to_owned(),
            property: property.to_owned(),
            reason: reason.to_owned(),
        }
    }

    fn messages(spec: &Value, exemptions: &[Exemption]) -> Vec<String> {
        evaluate_spec(spec, exemptions)
            .into_iter()
            .map(|finding| finding.message)
            .collect()
    }

    /// The small API descriptions in `tools/checks/fixtures/ARCH-NO-COUNTERS/{good,bad}`.
    fn fixture(kind: &str, name: &str) -> Value {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tools/checks/fixtures/ARCH-NO-COUNTERS")
            .join(kind)
            .join(name);
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    const GAP_ROUTE: &str = "GET /api/v1/roles/{role_id}/second-step-gap";
    const GAP_REASON: &str = "Only people the viewer may see are counted: whoever holds Manage roles sees names everywhere in Organization settings (M2-Q3).";

    #[test]
    fn good_example_a_paged_list_and_an_answer_without_a_count() {
        let found = messages(&fixture("good", "paged-list.json"), &[]);
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn bad_example_a_list_answer_with_a_total() {
        let found = messages(&fixture("bad", "list-with-total.json"), &[]);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("`total`"), "{found:?}");
    }

    #[test]
    fn bad_example_a_count_inside_an_item_or_with_another_name_is_found_too() {
        let found = messages(
            &fixture("bad", "counts-in-items-and-by-description.json"),
            &[],
        );
        assert_eq!(found.len(), 3, "{found:?}");
        for name in [
            "`member_count`",
            "`sub_projects_count`",
            "`still_to_set_up`",
        ] {
            assert!(
                found.iter().any(|message| message.contains(name)),
                "{name}: {found:?}"
            );
        }
    }

    #[test]
    fn good_example_a_count_of_what_the_viewer_may_see_is_named_with_its_reason() {
        let spec = fixture("good", "second-step-gap.json");
        assert!(
            messages(
                &spec,
                &[exemption(GAP_ROUTE, "still_to_set_up", GAP_REASON)]
            )
            .is_empty()
        );
        // Without the entry the same answer fails: the mechanism is the entry.
        assert_eq!(messages(&spec, &[]).len(), 1);
    }

    #[test]
    fn bad_example_an_entry_that_matches_nothing_or_has_no_reason() {
        let spec = fixture("good", "paged-list.json");
        let found = messages(&spec, &[exemption("GET /api/v1/gone", "total", GAP_REASON)]);
        assert!(
            found.iter().any(|m| m.contains("matches no property")),
            "{found:?}"
        );
        let spec = fixture("good", "second-step-gap.json");
        let found = messages(
            &spec,
            &[exemption(GAP_ROUTE, "still_to_set_up", "It is fine.")],
        );
        assert!(
            found.iter().any(|m| m.contains("needs a reason")),
            "{found:?}"
        );
    }

    #[test]
    fn names_that_count_and_names_that_only_contain_the_letters() {
        for name in [
            "total",
            "count",
            "member_count",
            "totalCount",
            "number_of_people",
            "sub-projects-count",
            "counts",
        ] {
            assert!(is_count_name(name), "{name}");
        }
        for name in [
            "account",
            "county",
            "discount",
            "phone_number",
            "next_cursor",
            "accounting",
            "summary",
            "limit",
            "version",
        ] {
            assert!(!is_count_name(name), "{name}");
        }
    }

    fn source(path: &str, text: &str) -> Vec<Source> {
        vec![(path.to_owned(), text.to_owned())]
    }

    #[test]
    fn bad_example_a_number_made_by_counting_or_a_column_the_database_counts() {
        for text in [
            "fn label(count: u32) -> u32 { count + 1 }\n",
            "fn n(list: &[u8]) -> usize { list.iter().count() + 1 }\n",
            "fn n(counter: u32) -> u32 {\n    counter\n        + 1\n}\n",
            "fn n() { let next_number = 3; }\n",
            "const SQL: &str = \"CREATE TABLE t (id INTEGER PRIMARY KEY AUTOINCREMENT)\";\n",
            "const SQL: &str = \"create sequence labels\";\n",
            "const SQL: &str = \"id BIGSERIAL PRIMARY KEY\";\n",
            "const SQL: &str = \"SELECT nextval('labels')\";\n",
            "fn c() { ColumnDef::new(Id).big_integer().auto_increment(); }\n",
        ] {
            let found = evaluate_code(&source("crates/bb-store/src/x.rs", text), &[]);
            assert!(!found.is_empty(), "{text}");
        }
    }

    #[test]
    fn good_example_paging_versions_random_labels_comments_texts_and_tests() {
        for text in [
            "fn page(limit: usize) -> usize { limit + 1 }\n",
            "fn v(version: i64) -> i64 { version + 1 }\n",
            "fn n(seq: i64) -> i64 { seq + 1 }\n",
            "// count + 1 and AUTOINCREMENT in a comment\n",
            "const T: &str = \"count + 1\";\n",
            "fn label(random: [u8; 4]) -> String { String::new() }\n",
            "#[cfg(test)]\nmod tests {\n    fn t(count: u32) -> u32 { count + 1 }\n}\n",
        ] {
            let found = evaluate_code(&source("crates/bb-store/src/x.rs", text), &[]);
            assert!(
                found.is_empty(),
                "{text}: {:?}",
                found.iter().map(|f| &f.message).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn good_example_the_folder_of_counting_issue_ids_may_count_and_must_still_do_so() {
        let counting = [Counting {
            folder: "crates/bb-app/src/issues/".to_owned(),
            reason: "Counting issue IDs of a project (ADR-0021, M3).".to_owned(),
        }];
        let text = "fn next(count: u32) -> u32 { count + 1 }\n";
        assert!(
            evaluate_code(&source("crates/bb-app/src/issues/ids.rs", text), &counting).is_empty()
        );
        // Elsewhere it still fails, and an entry with nothing to allow fails.
        assert!(!evaluate_code(&source("crates/bb-app/src/x.rs", text), &counting).is_empty());
        let found = evaluate_code(&source("crates/bb-app/src/x.rs", "fn f() {}\n"), &counting);
        assert!(
            found
                .iter()
                .any(|f| f.message.contains("nothing that counts"))
        );
    }

    #[test]
    fn a_document_without_paths_is_a_finding() {
        assert_eq!(messages(&json!({}), &[]).len(), 1);
    }
}
