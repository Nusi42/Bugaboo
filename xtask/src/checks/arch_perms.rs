//! ARCH-PERMS: every route declares the permission it needs; every permission is used and
//! explained (SEC-01, ADR-0004, ADR-0025 decision 1, architecture §7 *The permission catalogue*).
//!
//! Read from the generated API description (`x-permission` on every operation), the catalogue in
//! `bb-policy` (the enum `Permission` itself) and the message catalog:
//!
//! - every operation has `x-permission`, and its value is a key of the catalogue or one of
//!   `public`, `signed-in`, `own-account`;
//! - every permission of the catalogue is asked by a route, or is named in
//!   `tools/checks/permission-exemptions.toml` as used by the engine only, with a reason that
//!   names the follow-up that tracks it (a permission that field levels or a use case check
//!   without a route of its own yet). An entry for a permission that a route asks, or for one the
//!   catalogue does not have, fails, so the file cannot go stale;
//! - every permission has its sentence (`permission.<key>`) and its topic has its name
//!   (`permission-topic.<topic>`) in the message catalog; both keys are made from the words by the
//!   enums, so the check holds the made keys against the catalog.

use std::collections::BTreeSet;

use anyhow::Context as _;
use serde::Deserialize;
use serde_json::Value;

use bb_api_words::marks::PERMISSION as MARK;
use bb_api_words::needs::WITHOUT_CATALOGUE as WORDS;
use bb_policy::Permission;

use super::api_spec;
use super::exemptions;
use super::{Check, Context, Finding};
use crate::generate::messages::SOURCE_LOCALE_PATH;

/// Where the permissions are declared; only shown in messages.
const CATALOGUE_PATH: &str = "crates/bb-policy/src/catalogue.rs";
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
        let spec = api_spec::load(context.repo)?;
        let messages: Value = serde_json::from_str(&read(SOURCE_LOCALE_PATH, "")?)
            .with_context(|| format!("{SOURCE_LOCALE_PATH} is not valid JSON"))?;
        let message_keys: BTreeSet<String> = messages
            .as_object()
            .map(|object| object.keys().cloned().collect())
            .unwrap_or_default();
        let exemptions: Exemptions = exemptions::load(context.repo, EXEMPTIONS_PATH)?;
        Ok(evaluate(
            &spec,
            &catalogue(),
            &message_keys,
            &exemptions.engine_only,
        ))
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

impl exemptions::Exemption for EngineOnly {
    fn name(&self) -> String {
        self.permission.clone()
    }

    fn reason(&self) -> &str {
        &self.reason
    }
}

/// One permission as the catalogue declares it: its key and the two messages that explain it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Declared {
    pub(super) key: String,
    /// The sentence on the role page, and the name of the topic it is listed under.
    pub(super) messages: [String; 2],
}

/// The permissions, as the enum declares them.
fn catalogue() -> Vec<Declared> {
    Permission::ALL
        .iter()
        .map(|permission| Declared {
            key: permission.code().to_owned(),
            messages: [
                permission.sentence().as_str().to_owned(),
                permission.topic().name().as_str().to_owned(),
            ],
        })
        .collect()
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
            "No permission was found in the catalogue.",
        ));
    }
    let known: BTreeSet<&str> = catalogue
        .iter()
        .map(|declared| declared.key.as_str())
        .collect();
    let Some(operations) = api_spec::operations(spec) else {
        findings.push(api_spec::no_paths());
        return findings;
    };
    let mut asked: BTreeSet<&str> = BTreeSet::new();
    for operation in operations {
        let place = operation.place();
        match operation.text(MARK) {
            None => findings.push(Finding::at(
                place,
                format!(
                    "The route declares no permission. Add it to the table of routes in `bb-api` (`permissions.rs`): a key of the catalogue, or one of {}.",
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
        for message in &declared.messages {
            if !messages.contains(message) {
                findings.push(Finding::at(
                    format!("{CATALOGUE_PATH}: {}", declared.key),
                    format!("The message `{message}` is missing in {SOURCE_LOCALE_PATH}: every permission is explained on the role page."),
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
    let mut findings = exemptions::reason_findings(EXEMPTIONS_PATH, exemptions);
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
    }
    findings.extend(exemptions::follow_up_findings(EXEMPTIONS_PATH, exemptions));
    findings
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn declared(key: &str, topic: &str) -> Declared {
        Declared {
            key: key.to_owned(),
            messages: [
                format!("permission.{key}"),
                format!("permission-topic.{topic}"),
            ],
        }
    }

    fn two_permissions() -> Vec<Declared> {
        vec![
            declared("see-projects", "projects"),
            declared("manage-roles", "people"),
        ]
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
        evaluate(spec, &two_permissions(), &messages(), exemptions)
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
    fn the_catalogue_is_the_enum_with_the_messages_it_makes_from_the_words() {
        let all = catalogue();
        assert_eq!(all.len(), Permission::ALL.len());
        let first = &all[0];
        assert_eq!(first.key, Permission::ALL[0].code());
        assert_eq!(first.messages[0], format!("permission.{}", first.key));
        assert!(first.messages[1].starts_with("permission-topic."));
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
        let found: Vec<String> = evaluate(&asks_both, &two_permissions(), &messages, &[])
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

    #[test]
    fn the_permissions_of_this_repository_have_their_messages() {
        let repo = crate::repo::Repo::discover().unwrap();
        let text = std::fs::read_to_string(repo.path(SOURCE_LOCALE_PATH)).unwrap();
        let object: Value = serde_json::from_str(&text).unwrap();
        let keys: BTreeSet<String> = object.as_object().unwrap().keys().cloned().collect();
        for permission in catalogue() {
            for message in &permission.messages {
                assert!(keys.contains(message), "{message}");
            }
        }
    }
}
