//! ARCH-API-LINT: the API description keeps to one style (M4 WP-16, API changes A1, A3, A7, A10).
//!
//! It reads the generated description, so a new route or schema is found without anyone registering
//! it. Four rules:
//!
//! - **One casing for enum values.** Every value of an `enum` (and every `const`) is written in
//!   words joined by hyphens, with a dot between the parts of a code (`auth.wrong-credentials`).
//! - **No duplicate schemas.** Two schemas with the same members, types and values (the texts
//!   do not count) are one schema written twice; the code keeps one.
//! - **No orphan schemas.** Every schema is reached from a route, through its answers, its
//!   parameters and its bodies and through the schemas they name. A schema that only gives the
//!   browser a vocabulary is named in `tools/checks/api-lint-exemptions.toml` with a reason.
//! - **No `_id` in the name of a path parameter.** A path says `{role}`, not `{role_id}`.
//!
//! Words of the glossary in the names of schemas, properties, parameters and operations are the
//! work of TXT-TERMS, which reads the same description.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;
use serde_json::Value;

use super::api_spec::{self, resolve};
use super::exemptions::{self, Used};
use super::{Check, Context, Finding};
use crate::generate::api::SPEC_PATH;

/// Where the exemptions are written down, each with its reason.
const EXEMPTIONS_PATH: &str = "tools/checks/api-lint-exemptions.toml";

pub(crate) struct ArchApiLint;

impl Check for ArchApiLint {
    fn id(&self) -> &'static str {
        "ARCH-API-LINT"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let spec = api_spec::load(context.repo)?;
        let file: Exemptions = exemptions::load(context.repo, EXEMPTIONS_PATH)?;
        Ok(evaluate(&spec, &file.exempt))
    }
}

/// The file of exemptions.
#[derive(Deserialize)]
struct Exemptions {
    #[serde(default)]
    exempt: Vec<Exemption>,
}

/// One schema that has no route, and why it stands.
#[derive(Deserialize)]
struct Exemption {
    schema: String,
    reason: String,
}

impl exemptions::Exemption for Exemption {
    fn name(&self) -> String {
        self.schema.clone()
    }

    fn reason(&self) -> &str {
        &self.reason
    }

    fn unused(&self) -> String {
        "is not an orphan schema any more (or is not there). Remove it.".to_owned()
    }
}

/// Whether `value` is words joined by hyphens, with dots between the parts of a code.
fn is_kebab(value: &str) -> bool {
    let word = |word: &str| {
        !word.is_empty()
            && word
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    };
    !value.is_empty()
        && value
            .split('.')
            .all(|part| !part.is_empty() && part.split('-').all(word))
}

/// Every enum value and constant under `value`, with the place it is at.
fn casing_findings(value: &Value, at: &str, findings: &mut Vec<Finding>) {
    match value {
        Value::Object(members) => {
            for key in ["enum", "const"] {
                let words: Vec<&Value> = match members.get(key) {
                    Some(Value::Array(words)) => words.iter().collect(),
                    Some(word) => vec![word],
                    None => Vec::new(),
                };
                for word in words.into_iter().filter_map(Value::as_str) {
                    if !is_kebab(word) {
                        findings.push(Finding::at(
                            at,
                            format!("The value `{word}` is not written in words joined by hyphens (`kebab-case`)."),
                        ));
                    }
                }
            }
            for (name, member) in members {
                // Names of properties and of schemas are not values.
                let next = if name == "properties" || name == "schemas" {
                    at.to_owned()
                } else {
                    format!("{at}/{name}")
                };
                casing_findings(member, &next, findings);
            }
        }
        Value::Array(items) => {
            for item in items {
                casing_findings(item, at, findings);
            }
        }
        _ => {}
    }
}

/// The schema without the texts that say what it is for: two schemas that are the same here are the
/// same schema.
fn shape(value: &Value) -> Value {
    match value {
        Value::Object(members) => Value::Object(
            members
                .iter()
                .filter(|(name, _)| !matches!(name.as_str(), "description" | "title" | "example"))
                .map(|(name, member)| (name.clone(), shape(member)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(shape).collect()),
        other => other.clone(),
    }
}

fn duplicate_findings(spec: &Value) -> Vec<Finding> {
    let mut by_shape: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for (name, schema) in api_spec::schemas(spec) {
        by_shape
            .entry(shape(schema).to_string())
            .or_default()
            .push(name);
    }
    by_shape
        .into_values()
        .filter(|names| names.len() > 1)
        .map(|names| {
            Finding::at(
                names.join(", "),
                format!(
                    "{} are the same schema written {} times. Keep one and use it everywhere.",
                    names.join(", "),
                    names.len()
                ),
            )
        })
        .collect()
}

/// The names under `components` that something reaches from a route, `$ref` after `$ref`.
fn reached(spec: &Value) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut pending: Vec<&Value> = spec.get("paths").into_iter().collect();
    while let Some(value) = pending.pop() {
        match value {
            Value::Object(members) => {
                if let Some(reference) = members.get("$ref").and_then(Value::as_str)
                    && let Some(pointer) = reference.strip_prefix('#')
                    && seen.insert(pointer.to_owned())
                    && let Some(target) = spec.pointer(pointer)
                {
                    pending.push(target);
                }
                pending.extend(members.values());
            }
            Value::Array(items) => pending.extend(items.iter()),
            _ => {}
        }
    }
    seen
}

fn orphan_findings(spec: &Value, exemptions: &[Exemption], used: &mut Used) -> Vec<Finding> {
    let reached = reached(spec);
    let mut findings = Vec::new();
    for (name, _) in api_spec::schemas(spec) {
        if reached.contains(&format!("/components/schemas/{name}")) {
            continue;
        }
        match exemptions.iter().position(|each| each.schema == name) {
            Some(index) => used.mark(index),
            None => findings.push(Finding::at(
                name,
                format!(
                    "The schema `{name}` is used by no route. Use it, remove it, or name it in {EXEMPTIONS_PATH} with a reason."
                ),
            )),
        }
    }
    findings
}

fn parameter_findings(spec: &Value) -> Vec<Finding> {
    let mut findings = Vec::new();
    for operation in api_spec::operations(spec).into_iter().flatten() {
        for parameter in operation
            .body
            .get("parameters")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let parameter = resolve(spec, parameter);
            let name = parameter.get("name").and_then(Value::as_str);
            let in_path = parameter.get("in").and_then(Value::as_str) == Some("path");
            if let Some(name) = name.filter(|name| in_path && name.ends_with("_id")) {
                findings.push(Finding::at(
                    operation.place(),
                    format!(
                        "The path parameter `{name}` ends in `_id`; name the thing (`{{{}}}`).",
                        name.trim_end_matches("_id")
                    ),
                ));
            }
        }
    }
    findings
}

fn evaluate(spec: &Value, exemptions: &[Exemption]) -> Vec<Finding> {
    let mut findings = Vec::new();
    casing_findings(spec, SPEC_PATH, &mut findings);
    findings.extend(duplicate_findings(spec));
    let mut used = Used::default();
    findings.extend(orphan_findings(spec, exemptions, &mut used));
    findings.extend(parameter_findings(spec));
    findings.extend(exemptions::judge(EXEMPTIONS_PATH, exemptions, &used));
    findings.extend(exemptions::follow_up_findings(EXEMPTIONS_PATH, exemptions));
    findings
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn good() -> Value {
        json!({
            "paths": {"/api/v1/things/{thing}": {"get": {
                "operationId": "getThing",
                "parameters": [{"name": "thing", "in": "path"}],
                "responses": {"200": {"content": {"application/json": {"schema": {"$ref": "#/components/schemas/Thing"}}}}}
            }}},
            "components": {"schemas": {
                "Thing": {"type": "object", "properties": {
                    "status": {"$ref": "#/components/schemas/Status"},
                    "code": {"enum": ["auth.wrong-credentials", "set-here"]}
                }},
                "Status": {"type": "string", "enum": ["open", "in-progress"]}
            }}
        })
    }

    fn messages(spec: &Value, exemptions: &[Exemption]) -> Vec<String> {
        evaluate(spec, exemptions)
            .into_iter()
            .map(|finding| finding.message)
            .collect()
    }

    #[test]
    fn good_example_one_casing_no_copy_no_orphan_and_no_id_in_a_path() {
        assert!(messages(&good(), &[]).is_empty());
    }

    #[test]
    fn bad_example_a_value_that_is_not_kebab_case() {
        let mut spec = good();
        spec["components"]["schemas"]["Status"]["enum"] = json!(["open", "in_progress", "Done"]);
        let found = messages(&spec, &[]).join("\n");
        assert!(found.contains("`in_progress`"), "{found}");
        assert!(found.contains("`Done`"), "{found}");
    }

    #[test]
    fn bad_example_two_schemas_that_are_the_same() {
        let mut spec = good();
        spec["components"]["schemas"]["Condition"] = json!({
            "type": "string", "enum": ["open", "in-progress"], "description": "Another text."
        });
        spec["components"]["schemas"]["Thing"]["properties"]["condition"] =
            json!({"$ref": "#/components/schemas/Condition"});
        let found = messages(&spec, &[]).join("\n");
        assert!(
            found.contains("Condition") && found.contains("are the same schema"),
            "{found}"
        );
    }

    #[test]
    fn bad_example_a_schema_no_route_uses_and_an_id_in_a_path_parameter() {
        let mut spec = good();
        spec["components"]["schemas"]["Lonely"] = json!({"type": "object"});
        spec["paths"]["/api/v1/things/{thing_id}"] = json!({"get": {
            "operationId": "getOther",
            "parameters": [{"name": "thing_id", "in": "path"}],
            "responses": {}
        }});
        let found = messages(&spec, &[]).join("\n");
        assert!(
            found.contains("The schema `Lonely` is used by no route"),
            "{found}"
        );
        assert!(found.contains("`thing_id` ends in `_id`"), "{found}");
    }

    #[test]
    fn good_example_a_schema_that_only_gives_a_vocabulary_is_named_with_a_reason() {
        let mut spec = good();
        spec["components"]["schemas"]["Vocabulary"] =
            json!({"type": "string", "enum": ["alpha", "beta"]});
        let named = Exemption {
            schema: "Vocabulary".to_owned(),
            reason: "The browser takes its words from it (M4-F173).".to_owned(),
        };
        assert!(messages(&spec, &[named]).is_empty());
        let stale = Exemption {
            schema: "Gone".to_owned(),
            reason: "Nothing uses it (M4-F173).".to_owned(),
        };
        let found = messages(&good(), &[stale]).join("\n");
        assert!(
            found.contains("is not an orphan schema any more"),
            "{found}"
        );
    }

    #[test]
    fn a_schema_reached_through_a_shared_answer_is_not_an_orphan() {
        let spec = json!({
            "paths": {"/api/v1/a": {"get": {"operationId": "getA", "responses": {
                "404": {"$ref": "#/components/responses/NotFound"}
            }}}},
            "components": {
                "responses": {"NotFound": {"content": {"application/problem+json": {"schema": {"$ref": "#/components/schemas/Problem"}}}}},
                "schemas": {"Problem": {"type": "object"}}
            }
        });
        assert!(messages(&spec, &[]).is_empty());
    }

    #[test]
    fn words_are_kebab_and_dots_join_the_parts_of_a_code() {
        for good in [
            "open",
            "in-progress",
            "auth.wrong-credentials",
            "set-here",
            "a1",
        ] {
            assert!(is_kebab(good), "{good}");
        }
        for bad in ["", "Open", "in_progress", "a--b", "a.", "-a", "a b"] {
            assert!(!is_kebab(bad), "{bad}");
        }
    }
}
