//! ARCH-PERMS: every route declares the permission it needs; every permission is used and
//! explained (SEC-01, ADR-0004, ADR-0025 decision 1, architecture §7 *The permission catalogue*).
//!
//! Read from the generated API description (`x-permission` on every operation), the catalogue in
//! `bb-policy` (one line per permission, read from the source of the enum) and the message
//! catalog:
//!
//! - every operation has `x-permission`, and its value is a key of the catalogue or one of
//!   `public`, `signed-in`, `own-account`;
//! - every permission of the catalogue is asked by a route, or is named in
//!   `tools/checks/permission-exemptions.toml` as used by the engine only, with a reason that
//!   names the follow-up that tracks it (a permission that field levels or a use case check
//!   without a route of its own yet). An entry for a permission that a route asks, or for one the
//!   catalogue does not have, fails, so the file cannot go stale;
//! - every permission has its sentence (`permission.<key>`) and its topic has its name
//!   (`permission-topic.<topic>`) in the message catalog.

use std::collections::BTreeSet;

use anyhow::Context as _;
use serde::Deserialize;
use serde_json::Value;

use super::arch_lean::names_a_follow_up;
use super::{Check, Context, Finding};
use crate::generate::api::SPEC_PATH;

/// The mark of an operation that says what it needs.
const MARK: &str = "x-permission";
/// What a route may declare besides a permission of the catalogue.
const WORDS: [&str; 3] = ["public", "signed-in", "own-account"];
/// The methods of an item of `paths` that are operations.
const METHODS: [&str; 8] = [
    "get", "put", "post", "delete", "options", "head", "patch", "trace",
];
/// Where the permissions are declared.
const CATALOGUE_PATH: &str = "crates/bb-policy/src/catalogue.rs";
/// The message catalog.
const MESSAGES_PATH: &str = "messages/en.json";
/// Where the permissions that only the engine uses are named, each with its reason.
const EXEMPTIONS_PATH: &str = "tools/checks/permission-exemptions.toml";

pub(crate) struct ArchPerms;

impl Check for ArchPerms {
    fn id(&self) -> &'static str {
        "ARCH-PERMS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let read = |path: &str, hint: &str| {
            std::fs::read_to_string(context.repo.path(path))
                .with_context(|| format!("could not read {path}{hint}"))
        };
        let spec: Value = serde_json::from_str(&read(SPEC_PATH, "; run `cargo xtask gen api`")?)
            .with_context(|| format!("{SPEC_PATH} is not valid JSON"))?;
        let source = read(CATALOGUE_PATH, "")?.replace("\r\n", "\n");
        let catalogue = read_catalogue(&source);
        let messages: Value = serde_json::from_str(&read(MESSAGES_PATH, "")?)
            .with_context(|| format!("{MESSAGES_PATH} is not valid JSON"))?;
        let message_keys: BTreeSet<String> = messages
            .as_object()
            .map(|object| object.keys().cloned().collect())
            .unwrap_or_default();
        let exemptions: Exemptions = toml::from_str(&read(EXEMPTIONS_PATH, "")?)
            .with_context(|| format!("{EXEMPTIONS_PATH} is not valid"))?;
        let mut findings = reading_findings(&source, &catalogue);
        findings.extend(evaluate(
            &spec,
            &catalogue,
            &message_keys,
            &exemptions.engine_only,
        ));
        Ok(findings)
    }
}

/// The file of permissions that only the engine uses.
#[derive(Deserialize)]
struct Exemptions {
    #[serde(default, rename = "engine-only")]
    engine_only: Vec<EngineOnly>,
}

/// A permission that no route asks for, and why.
#[derive(Deserialize)]
struct EngineOnly {
    permission: String,
    reason: String,
}

/// One permission as the catalogue declares it: its key and the name of its topic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Declared {
    pub(super) key: String,
    pub(super) topic: String,
}

/// The permissions declared in the source of the enum: one line `Variant => ("key", Topic, …)`
/// each. Comment lines do not count.
pub(super) fn read_catalogue(source: &str) -> Vec<Declared> {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .filter_map(|line| {
            let rest = line.split_once("=> (\"")?.1;
            let (key, rest) = rest.split_once('"')?;
            let topic = rest.strip_prefix(',')?.split(',').next()?.trim();
            (!key.is_empty() && !topic.is_empty()).then(|| Declared {
                key: key.to_owned(),
                topic: topic.to_owned(),
            })
        })
        .collect()
}

/// How many variants the source of the enum declares: the lines that start with a name and `=>`
/// (the line of a variant whose declaration is broken over lines starts so too, and has no key on
/// it).
fn variant_lines(source: &str) -> usize {
    source
        .lines()
        .map(str::trim_start)
        .filter(|line| !line.starts_with("//"))
        .filter(|line| {
            let name: String = line
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            name.chars().next().is_some_and(char::is_uppercase)
                && line[name.len()..].trim_start().starts_with("=>")
        })
        .count()
}

/// A variant that the check could not read is a permission it would not hold to the rules: the
/// number of variants must be the number of permissions it read.
fn reading_findings(source: &str, catalogue: &[Declared]) -> Vec<Finding> {
    let variants = variant_lines(source);
    if variants == catalogue.len() {
        return Vec::new();
    }
    vec![Finding::at(
        CATALOGUE_PATH,
        format!(
            "The enum declares {variants} permissions but the check read {}: write each as one \
             line `Variant => (\"key\", Topic, …)`, so none is left out of the rules.",
            catalogue.len()
        ),
    )]
}

/// The name of a topic as the message catalog spells it (`Projects` is `projects`).
fn topic_message(topic: &str) -> String {
    format!("permission-topic.{}", topic.to_lowercase())
}

fn evaluate(
    spec: &Value,
    catalogue: &[Declared],
    messages: &BTreeSet<String>,
    exemptions: &[EngineOnly],
) -> Vec<Finding> {
    let mut findings = Vec::new();
    if catalogue.is_empty() {
        findings.push(Finding::at(
            CATALOGUE_PATH,
            "No permission was found in the catalogue: the check reads one line \
             `Variant => (\"key\", Topic, …)` for each.",
        ));
    }
    let known: BTreeSet<&str> = catalogue
        .iter()
        .map(|declared| declared.key.as_str())
        .collect();
    let Some(paths) = spec.get("paths").and_then(Value::as_object) else {
        findings.push(Finding::at(SPEC_PATH, "The document has no `paths`."));
        return findings;
    };
    let mut asked: BTreeSet<&str> = BTreeSet::new();
    for (path, item) in paths {
        for method in METHODS {
            let Some(operation) = item.get(method) else {
                continue;
            };
            let place = format!("{} {path}", method.to_uppercase());
            match operation.get(MARK).and_then(Value::as_str) {
                None => findings.push(Finding::at(
                    place,
                    format!(
                        "The route declares no permission. Add it to the table of routes in \
                         `bb-api` (`permissions.rs`): a key of the catalogue, or one of {}.",
                        WORDS.join(", ")
                    ),
                )),
                Some(value) if known.contains(value) => {
                    asked.insert(value);
                }
                Some(value) if WORDS.contains(&value) => {}
                Some(value) => findings.push(Finding::at(
                    place,
                    format!(
                        "`{value}` is neither a permission of the catalogue nor one of {}.",
                        WORDS.join(", ")
                    ),
                )),
            }
        }
    }
    for declared in catalogue {
        if !asked.contains(declared.key.as_str())
            && !exemptions
                .iter()
                .any(|exemption| exemption.permission == declared.key)
        {
            findings.push(Finding::at(
                format!("{CATALOGUE_PATH}: {}", declared.key),
                format!(
                    "No route asks for this permission. Declare it on the route that needs it, or \
                     name it in {EXEMPTIONS_PATH} as used by the engine only, with a reason that \
                     names the follow-up that tracks it."
                ),
            ));
        }
        for message in [
            format!("permission.{}", declared.key),
            topic_message(&declared.topic),
        ] {
            if !messages.contains(&message) {
                findings.push(Finding::at(
                    format!("{CATALOGUE_PATH}: {}", declared.key),
                    format!("The message `{message}` is missing in {MESSAGES_PATH}: every permission is explained on the role page."),
                ));
            }
        }
    }
    findings.extend(exemption_findings(exemptions, &known, &asked));
    findings
}

/// The rules of the file of engine-only permissions: each names a permission of the catalogue that
/// no route asks for, once, with a reason that names a follow-up.
fn exemption_findings(
    exemptions: &[EngineOnly],
    known: &BTreeSet<&str>,
    asked: &BTreeSet<&str>,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut seen = BTreeSet::new();
    for exemption in exemptions {
        let key = exemption.permission.as_str();
        let mut stale = |message: String| findings.push(Finding::at(EXEMPTIONS_PATH, message));
        if !known.contains(key) {
            stale(format!(
                "`{key}` is no permission of the catalogue. Remove the entry."
            ));
        } else if asked.contains(key) {
            stale(format!(
                "A route asks for `{key}` now, so it is no longer used by the engine only. Remove \
                 the entry."
            ));
        }
        if !seen.insert(key) {
            stale(format!("`{key}` is named twice."));
        }
        if !names_a_follow_up(&exemption.reason) {
            stale(format!(
                "The entry of `{key}` must give a reason that names the follow-up that tracks it \
                 (like M2-F60)."
            ));
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn catalogue() -> Vec<Declared> {
        read_catalogue(
            "/// A comment about `X => (\"nope\", Projects, a)`.\n\
             SeeProjects => (\"see-projects\", Projects, Project, See, message::A),\n\
             ManageRoles => (\"manage-roles\", People, Organization, Manage, message::B),\n\
             ($variant:ident => ($key:literal, $topic:ident)),\n",
        )
    }

    fn messages() -> BTreeSet<String> {
        [
            "permission.see-projects",
            "permission.manage-roles",
            "permission-topic.projects",
            "permission-topic.people",
        ]
        .iter()
        .map(|key| (*key).to_owned())
        .collect()
    }

    fn engine_only(permission: &str, reason: &str) -> EngineOnly {
        EngineOnly {
            permission: permission.to_owned(),
            reason: reason.to_owned(),
        }
    }

    fn found(spec: &Value, exemptions: &[EngineOnly]) -> Vec<String> {
        evaluate(spec, &catalogue(), &messages(), exemptions)
            .into_iter()
            .map(|finding| finding.message)
            .collect()
    }

    /// The small API descriptions in `tools/checks/fixtures/ARCH-PERMS/{good,bad}`.
    fn fixture(kind: &str, name: &str) -> Value {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tools/checks/fixtures/ARCH-PERMS")
            .join(kind)
            .join(name);
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn the_catalogue_is_read_from_the_lines_of_the_enum_and_not_from_comments_or_the_macro() {
        let keys: Vec<(String, String)> = catalogue()
            .into_iter()
            .map(|declared| (declared.key, declared.topic))
            .collect();
        assert_eq!(
            keys,
            [
                ("see-projects".to_owned(), "Projects".to_owned()),
                ("manage-roles".to_owned(), "People".to_owned())
            ]
        );
    }

    #[test]
    fn bad_example_a_permission_declared_over_several_lines_is_not_read_and_says_so() {
        let one_line = "A => (\"a\", Projects, Project, See, message::A),\nB => (\"b\", People, Organization, Manage, message::B),\n";
        let broken = "A => (\"a\", Projects, Project, See, message::A),\nB =>\n    (\"b\", People, Organization, Manage, message::B),\n";
        assert!(reading_findings(one_line, &read_catalogue(one_line)).is_empty());
        let found = reading_findings(broken, &read_catalogue(broken));
        assert_eq!(found.len(), 1);
        assert!(found[0].message.contains("declares 2 permissions"));
        // The macro pattern and the comments are no variants.
        let pattern = "$($variant:ident => ($key:literal),+)\n// C => (\"c\", X, Y, Z, m)\n";
        assert_eq!(variant_lines(pattern), 0);
    }

    #[test]
    fn good_example_fixtures_pass() {
        let found = found(&fixture("good", "every-route-declares-one.json"), &[]);
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn bad_example_a_route_without_x_permission() {
        let found = found(&fixture("bad", "route-without-x-permission.json"), &[]);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("declares no permission"), "{found:?}");
    }

    #[test]
    fn bad_example_a_declaration_that_is_no_permission_and_no_word() {
        let found = found(&fixture("bad", "unknown-permission.json"), &[]);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("`see-everything`"), "{found:?}");
    }

    #[test]
    fn bad_example_a_permission_nobody_uses() {
        // Only words are declared: both permissions of the catalogue have no route.
        let only_words = json!({"paths": {"/a": {"get": {MARK: "public"}}}});
        let found = found(&only_words, &[]);
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(
            found
                .iter()
                .all(|message| message.contains("No route asks"))
        );
    }

    #[test]
    fn good_example_a_permission_of_the_engine_only_is_named_with_a_follow_up() {
        let only_words = json!({"paths": {"/a": {"get": {MARK: "public"}}}});
        let exemptions = [
            engine_only(
                "see-projects",
                "The engine decides it for the tree (M2-F60).",
            ),
            engine_only("manage-roles", "Its screen comes with WP-11 (M2-F60)."),
        ];
        let found = found(&only_words, &exemptions);
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn bad_example_an_entry_that_is_stale_unknown_twice_or_without_a_follow_up() {
        let asks_one = json!({"paths": {"/a": {"get": {MARK: "see-projects"}}}});
        let found = found(
            &asks_one,
            &[
                // A route asks for it now.
                engine_only("see-projects", "Was engine only (M2-F60)."),
                // No such permission.
                engine_only("make-tea", "Nope (M2-F60)."),
                // Named twice, and no follow-up.
                engine_only("manage-roles", "Soon."),
                engine_only("manage-roles", "Soon (M2-F60)."),
            ],
        );
        for expected in [
            "A route asks for `see-projects`",
            "`make-tea` is no permission",
            "`manage-roles` is named twice",
            "names the follow-up",
        ] {
            assert!(
                found.iter().any(|message| message.contains(expected)),
                "{expected}: {found:?}"
            );
        }
    }

    #[test]
    fn bad_example_a_permission_without_its_sentence_or_its_topic_name() {
        let asks_both = json!({"paths": {"/a": {
            "get": {MARK: "see-projects"},
            "post": {MARK: "manage-roles"}
        }}});
        let mut messages = messages();
        messages.remove("permission.manage-roles");
        messages.remove("permission-topic.projects");
        let found: Vec<String> = evaluate(&asks_both, &catalogue(), &messages, &[])
            .into_iter()
            .map(|finding| finding.message)
            .collect();
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(
            found
                .iter()
                .any(|m| m.contains("`permission.manage-roles`"))
        );
        assert!(
            found
                .iter()
                .any(|m| m.contains("`permission-topic.projects`"))
        );
    }

    #[test]
    fn a_document_without_paths_or_a_catalogue_is_a_finding() {
        assert_eq!(found(&json!({}), &[]).len(), 1);
        let none = evaluate(&json!({"paths": {}}), &[], &messages(), &[]);
        assert_eq!(none.len(), 1);
    }
}
