//! ARCH-LEAN: every list route supports `fields=`, cursor paging and a page-size cap (PERF-02).
//!
//! Read from the generated OpenAPI document, so a new list route is found without anyone
//! registering it. A list is **any array property** of the object a `GET` answers with, not only
//! one called `items`, so a list cannot dodge the rule by its name. A list called `items` must
//! take the query parameters `fields` and `cursor` and a `limit` with a `maximum`, and its answer
//! must hold `next_cursor`. A list with another name is only allowed when it is named in
//! `tools/checks/lean-exemptions.toml` with a reason that names the follow-up (`M1-F44`): the
//! lists that are small by nature (the sessions of one person) have a hard cap instead, and their
//! answer must say when the cap cut the list off (`truncated`); an array that is the whole value of
//! one setting (`whole_value`) needs no flag. An exemption that matches no list fails, so the file
//! cannot go stale.

use std::collections::BTreeSet;

use anyhow::Context as _;
use serde::Deserialize;
use serde_json::Value;

use super::{Check, Context, Finding};
use crate::generate::api::SPEC_PATH;

/// The parameters a list route takes.
const REQUIRED_PARAMETERS: [&str; 3] = ["fields", "cursor", "limit"];
/// The only name of a list that is paged.
const PAGED_LIST: &str = "items";
/// Where the exemptions are written down, each with its reason.
const EXEMPTIONS_PATH: &str = "tools/checks/lean-exemptions.toml";
/// What the answer of an exempt list must say.
const CAP_FLAG: &str = "truncated";

pub(crate) struct ArchLean;

impl Check for ArchLean {
    fn id(&self) -> &'static str {
        "ARCH-LEAN"
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
        Ok(evaluate(&spec, &exemptions.exempt))
    }
}

/// The file of exemptions.
#[derive(Deserialize)]
struct Exemptions {
    #[serde(default)]
    exempt: Vec<Exemption>,
}

/// One list that is allowed to be a plain array: the route (`GET /api/v1/...`), the name of the
/// array property in the answer, and why. `whole_value` says that the array is the value of one
/// setting, read and written as a whole and bounded when it is written, not a list of records that
/// grows; it needs no `truncated` flag.
#[derive(Deserialize)]
struct Exemption {
    route: String,
    property: String,
    reason: String,
    #[serde(default)]
    whole_value: bool,
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

/// The object schema of the successful JSON answer.
fn answer_schema<'a>(spec: &'a Value, operation: &'a Value) -> Option<&'a Value> {
    let ok = operation.pointer("/responses/200")?;
    let schema = resolve(spec, ok)
        .pointer("/content/application~1json/schema")
        .map(|schema| resolve(spec, schema))?;
    Some(schema)
}

fn is_array(spec: &Value, property: &Value) -> bool {
    let kind = resolve(spec, property).get("type");
    kind.and_then(Value::as_str) == Some("array")
        || kind
            .and_then(Value::as_array)
            .is_some_and(|kinds| kinds.iter().any(|kind| kind.as_str() == Some("array")))
}

/// The names of the properties that are arrays.
fn array_properties(spec: &Value, schema: &Value) -> Vec<String> {
    schema
        .get("properties")
        .and_then(Value::as_object)
        .map(|properties| {
            properties
                .iter()
                .filter(|(_, property)| is_array(spec, property))
                .map(|(name, _)| name.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// The rules of a paged list: parameters, a maximum for `limit`, and `next_cursor`.
fn paging_findings(spec: &Value, operation: &Value, schema: &Value, place: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let parameters: Vec<&Value> = operation
        .get("parameters")
        .and_then(Value::as_array)
        .map(|list| list.iter().map(|p| resolve(spec, p)).collect())
        .unwrap_or_default();
    for name in REQUIRED_PARAMETERS {
        let found = parameters.iter().find(|parameter| {
            parameter.get("name").and_then(Value::as_str) == Some(name)
                && parameter.get("in").and_then(Value::as_str) == Some("query")
        });
        match found {
            None => findings.push(Finding::at(
                place,
                format!(
                    "A list route must take the query parameter `{name}` (lean answers, paging)."
                ),
            )),
            Some(parameter) if name == "limit" => {
                let maximum = parameter.pointer("/schema/maximum");
                if !maximum.is_some_and(Value::is_number) {
                    findings.push(Finding::at(
                        place,
                        "The parameter `limit` must have a `maximum`, so one request cannot ask for everything.",
                    ));
                }
            }
            Some(_) => {}
        }
    }
    if schema.pointer("/properties/next_cursor").is_none() {
        findings.push(Finding::at(
            place,
            "The answer of a list route must hold `next_cursor`.",
        ));
    }
    findings
}

fn evaluate(spec: &Value, exemptions: &[Exemption]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let Some(paths) = spec.get("paths").and_then(Value::as_object) else {
        return vec![Finding::at(SPEC_PATH, "The document has no `paths`.")];
    };
    let mut used: BTreeSet<(String, String)> = BTreeSet::new();
    for (path, item) in paths {
        let Some(operation) = item.get("get") else {
            continue;
        };
        let Some(schema) = answer_schema(spec, operation) else {
            continue;
        };
        let place = format!("GET {path}");
        for property in array_properties(spec, schema) {
            let exemption = exemptions
                .iter()
                .find(|exemption| exemption.route == place && exemption.property == property);
            if let Some(exemption) = exemption {
                used.insert((place.clone(), property.clone()));
                if !names_a_follow_up(&exemption.reason) {
                    findings.push(Finding::at(
                        EXEMPTIONS_PATH,
                        format!("The exemption of `{property}` in {place} must give a reason that names the follow-up that tracks it (like M1-F44)."),
                    ));
                }
                let flag = schema.pointer(&format!("/properties/{CAP_FLAG}/type"));
                if !exemption.whole_value && flag.and_then(Value::as_str) != Some("boolean") {
                    findings.push(Finding::at(
                        &place,
                        format!("A list with a hard cap must say when the cap cut it off: the answer needs a boolean `{CAP_FLAG}`."),
                    ));
                }
            } else if property == PAGED_LIST {
                findings.extend(paging_findings(spec, operation, schema, &place));
            } else {
                findings.push(Finding::at(
                    &place,
                    format!("The answer has the list `{property}`. A list is called `{PAGED_LIST}` and takes `fields`, `cursor` and `limit`; or, when it is small by nature, it is named in {EXEMPTIONS_PATH} with a reason."),
                ));
            }
        }
    }
    for exemption in exemptions {
        if !used.contains(&(exemption.route.clone(), exemption.property.clone())) {
            findings.push(Finding::at(
                EXEMPTIONS_PATH,
                format!(
                    "The exemption of `{}` in {} matches no list in the API description. Remove it.",
                    exemption.property, exemption.route
                ),
            ));
        }
    }
    findings
}

/// Whether the reason names a follow-up like `M1-F44`.
fn names_a_follow_up(reason: &str) -> bool {
    reason
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
        .any(|word| {
            let mut parts = word.split('-');
            let (Some(milestone), Some(number), None) = (parts.next(), parts.next(), parts.next())
            else {
                return false;
            };
            let milestone_ok = milestone.len() >= 2
                && milestone.starts_with(['M', 'V'])
                && milestone[1..].chars().all(|c| c.is_ascii_digit());
            let number_ok = number.len() >= 2
                && number.starts_with('F')
                && number[1..].chars().all(|c| c.is_ascii_digit());
            milestone_ok && number_ok
        })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn spec_with(parameters: &Value, schema: &Value) -> Value {
        json!({
            "paths": {
                "/api/v1/issues": {"get": {
                    "parameters": parameters,
                    "responses": {"200": {"description": "ok", "content": {"application/json": {"schema": {"$ref": "#/components/schemas/Page"}}}}}
                }},
                "/api/v1/status": {"get": {"responses": {"200": {"content": {"application/json": {"schema": {"type": "object", "properties": {"status": {"type": "string"}}}}}}}}}
            },
            "components": {"schemas": {"Page": schema}}
        })
    }

    fn page(list: &str, with_cursor: bool) -> Value {
        let mut properties = json!({ list: {"type": "array", "items": {}}});
        if with_cursor {
            properties["next_cursor"] = json!({"type": "string"});
        }
        json!({"type": "object", "properties": properties})
    }

    fn parameters(names: &[&str], limit_maximum: bool) -> Value {
        Value::Array(
            names
                .iter()
                .map(|name| {
                    let mut schema = json!({"type": "integer"});
                    if *name == "limit" && limit_maximum {
                        schema["maximum"] = json!(100);
                    }
                    json!({"name": name, "in": "query", "schema": schema})
                })
                .collect(),
        )
    }

    fn messages(spec: &Value, exemptions: &[Exemption]) -> Vec<String> {
        evaluate(spec, exemptions)
            .into_iter()
            .map(|f| f.message)
            .collect()
    }

    fn exemption(route: &str, property: &str, reason: &str) -> Exemption {
        Exemption {
            route: route.to_owned(),
            property: property.to_owned(),
            reason: reason.to_owned(),
            whole_value: false,
        }
    }

    /// A list with a hard cap: an array and, if `flag`, the flag that says the cap cut it off.
    fn capped(flag: bool) -> Value {
        let mut schema = page("issues", false);
        if flag {
            schema["properties"]["truncated"] = json!({"type": "boolean"});
        }
        schema
    }

    #[test]
    fn good_example() {
        let good = spec_with(
            &parameters(&["fields", "cursor", "limit"], true),
            &page("items", true),
        );
        assert!(messages(&good, &[]).is_empty());
    }

    #[test]
    fn bad_example() {
        let bad = spec_with(&parameters(&["limit"], false), &page("items", false));
        let found = messages(&bad, &[]).join("\n");
        assert!(found.contains("`fields`"), "{found}");
        assert!(found.contains("`cursor`"), "{found}");
        assert!(found.contains("must have a `maximum`"), "{found}");
        assert!(found.contains("next_cursor"), "{found}");
    }

    #[test]
    fn bad_example_a_list_that_is_not_called_items_does_not_dodge_the_rule() {
        let bad = spec_with(&parameters(&[], false), &page("issues", false));
        let found = messages(&bad, &[]).join("\n");
        assert!(found.contains("the list `issues`"), "{found}");
    }

    #[test]
    fn good_example_a_small_list_with_a_cap_is_exempt_with_a_reason_and_says_when_cut() {
        let good = spec_with(&parameters(&[], false), &capped(true));
        let exemptions = [exemption(
            "GET /api/v1/issues",
            "issues",
            "Small by nature; hard cap (M1-F44).",
        )];
        assert!(messages(&good, &exemptions).is_empty());
    }

    #[test]
    fn good_example_the_whole_value_of_a_setting_needs_no_cap_flag() {
        let spec = spec_with(&parameters(&[], false), &capped(false));
        let whole = Exemption {
            whole_value: true,
            ..exemption("GET /api/v1/issues", "issues", "One setting (M1-F44).")
        };
        assert!(messages(&spec, &[whole]).is_empty());
    }

    #[test]
    fn bad_example_an_exemption_needs_the_flag_a_follow_up_and_a_list_to_match() {
        let route = "GET /api/v1/issues";
        let no_flag = spec_with(&parameters(&[], false), &capped(false));
        let found = messages(
            &no_flag,
            &[exemption(route, "issues", "Small by nature (M1-F44).")],
        )
        .join("\n");
        assert!(found.contains("`truncated`"), "{found}");
        let flagged = spec_with(&parameters(&[], false), &capped(true));
        let found = messages(&flagged, &[exemption(route, "issues", "Because.")]).join("\n");
        assert!(found.contains("names the follow-up"), "{found}");
        let found = messages(
            &flagged,
            &[
                exemption(route, "issues", "Small by nature (M1-F44)."),
                exemption("GET /api/v1/gone", "x", "Old (M1-F44)."),
            ],
        )
        .join("\n");
        assert!(found.contains("matches no list"), "{found}");
    }

    #[test]
    fn follow_ups_are_recognised_in_a_reason() {
        assert!(names_a_follow_up("See M1-F44."));
        assert!(names_a_follow_up("tracked as V1-F12, later"));
        for bad in ["", "M1", "F44", "M1-44", "see the follow-up list"] {
            assert!(!names_a_follow_up(bad), "{bad}");
        }
    }

    #[test]
    fn routes_that_are_not_lists_are_left_alone() {
        let spec = json!({"paths": {"/api/v1/status": {"get": {"responses": {"200": {"content": {"application/json": {"schema": {"type": "object"}}}}}}}}});
        assert!(messages(&spec, &[]).is_empty());
    }

    #[test]
    fn a_document_without_paths_is_a_finding() {
        assert_eq!(messages(&json!({}), &[]).len(), 1);
    }
}
