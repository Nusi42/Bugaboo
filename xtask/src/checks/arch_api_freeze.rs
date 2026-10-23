//! ARCH-API-FREEZE: nothing in `/api/v1` is renamed or removed after the API freeze (ADR-0008,
//! amendment M4).
//!
//! The frozen copy (`tools/checks/api-frozen.json`, the generated API description in compact
//! form) is stored by `cargo xtask freeze`; the lead does it at `m4-done`. From then on every
//! description the code writes is compared with it. A path, an operation, a parameter, an answer, a
//! schema, a property, an enum value (an error code is one) of the frozen copy that the new
//! description does not have fails; what the new description adds passes. A change that
//! must stand needs a decision record and a new version, and then a new frozen copy.
//!
//! The comparison is made on *facts*: one line of text for each name that a client can depend on.
//! The members of `oneOf`, `anyOf` and `allOf` count as the schema that holds them, so a new
//! variant is an addition and a property that moves between variants is not seen.

use std::collections::BTreeSet;

use anyhow::Context as _;
use serde_json::Value;

use super::api_spec::{self, resolve};
use super::{Check, Context, Finding};
use crate::generate::api::SPEC_PATH;
use crate::repo::Repo;
use crate::source::read_text;

/// The frozen copy of the API description, relative to the repository root.
pub(crate) const FROZEN_PATH: &str = "tools/checks/api-frozen.json";

/// The most findings written out, so a rename of a whole family reads as one problem.
const MOST_FINDINGS: usize = 40;

pub(crate) struct ArchApiFreeze;

impl Check for ArchApiFreeze {
    fn id(&self) -> &'static str {
        "ARCH-API-FREEZE"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let current = api_spec::load(context.repo)?;
        let Ok(text) = read_text(context.repo, FROZEN_PATH) else {
            return Ok(vec![Finding::at(
                FROZEN_PATH,
                "There is no frozen copy of the API description. Store it with `cargo xtask freeze` (ADR-0008).",
            )]);
        };
        let frozen = api_spec::parse(&text)?;
        Ok(evaluate(&frozen, &current))
    }
}

/// Stores the API description as it is now as the frozen copy.
pub(crate) fn write_frozen(repo: &Repo) -> anyhow::Result<()> {
    let spec = api_spec::load(repo)?;
    let compact = serde_json::to_string(&spec).context("the API description is not JSON")?;
    std::fs::write(repo.path(FROZEN_PATH), format!("{compact}\n"))
        .with_context(|| format!("could not write {FROZEN_PATH}"))
}

/// What a client can depend on in the description, one line for each.
fn facts(spec: &Value) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    path_facts(spec, &mut found);
    for (name, schema) in api_spec::schemas(spec) {
        found.insert(format!("schema {name}"));
        schema_facts(schema, name, &mut found);
    }
    found
}

fn path_facts(spec: &Value, found: &mut BTreeSet<String>) {
    let Some(paths) = spec.get("paths").and_then(Value::as_object) else {
        return;
    };
    for path in paths.keys() {
        found.insert(format!("path {path}"));
    }
    for operation in api_spec::operations(spec).into_iter().flatten() {
        let place = operation.place();
        found.insert(format!(
            "operation {place} {}",
            operation.text("operationId").unwrap_or_default()
        ));
        if operation.body.get("requestBody").is_some() {
            found.insert(format!("body {place}"));
        }
        for parameter in operation
            .body
            .get("parameters")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let parameter = resolve(spec, parameter);
            let place_of = parameter
                .get("in")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let name = parameter
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            found.insert(format!("parameter {place} {place_of} {name}"));
        }
        for status in operation
            .body
            .get("responses")
            .and_then(Value::as_object)
            .into_iter()
            .flat_map(serde_json::Map::keys)
        {
            found.insert(format!("answer {place} {status}"));
        }
    }
}

/// The properties and the enum values of a schema, below the name `at`.
fn schema_facts(schema: &Value, at: &str, found: &mut BTreeSet<String>) {
    if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
        for (name, property) in properties {
            found.insert(format!("property {at}.{name}"));
            schema_facts(property, &format!("{at}.{name}"), found);
        }
    }
    for key in ["enum", "const"] {
        let values: Vec<&Value> = match schema.get(key) {
            Some(Value::Array(values)) => values.iter().collect(),
            Some(value) => vec![value],
            None => Vec::new(),
        };
        for value in values.into_iter().filter_map(Value::as_str) {
            found.insert(format!("value {at} {value}"));
        }
    }
    if let Some(items) = schema.get("items") {
        schema_facts(items, &format!("{at}[]"), found);
    }
    if let Some(extra) = schema.get("additionalProperties") {
        schema_facts(extra, &format!("{at}{{}}"), found);
    }
    for key in ["oneOf", "anyOf", "allOf"] {
        for member in schema
            .get(key)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            schema_facts(member, at, found);
        }
    }
}

fn evaluate(frozen: &Value, current: &Value) -> Vec<Finding> {
    let now = facts(current);
    let gone: Vec<String> = facts(frozen)
        .into_iter()
        .filter(|fact| !now.contains(fact))
        .collect();
    let mut findings: Vec<Finding> = gone
        .iter()
        .take(MOST_FINDINGS)
        .map(|fact| {
            Finding::at(
                SPEC_PATH,
                format!(
                    "`{fact}` is in the frozen API and is gone. Renaming or removing anything of /api/v1 needs a decision record and a new version; adding is free (ADR-0008)."
                ),
            )
        })
        .collect();
    if gone.len() > MOST_FINDINGS {
        findings.push(Finding::at(
            SPEC_PATH,
            format!("And {} more.", gone.len() - MOST_FINDINGS),
        ));
    }
    findings
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn spec() -> Value {
        json!({
            "paths": {
                "/api/v1/things/{thing}": {
                    "get": {
                        "operationId": "getThing",
                        "parameters": [
                            {"name": "thing", "in": "path"},
                            {"$ref": "#/components/parameters/Fields"}
                        ],
                        "responses": {"200": {"description": "ok"}, "404": {"description": "none"}}
                    }
                }
            },
            "components": {
                "parameters": {"Fields": {"name": "fields", "in": "query"}},
                "schemas": {
                    "Thing": {
                        "type": "object",
                        "properties": {
                            "id": {"type": "string"},
                            "status": {"oneOf": [{"type": "string", "enum": ["open", "done"]}]}
                        }
                    },
                    "Code": {"type": "string", "enum": ["auth.wrong-credentials"]}
                }
            }
        })
    }

    fn messages(frozen: &Value, current: &Value) -> Vec<String> {
        evaluate(frozen, current)
            .into_iter()
            .map(|finding| finding.message)
            .collect()
    }

    #[test]
    fn good_example_an_addition_passes() {
        let mut now = spec();
        now["components"]["schemas"]["Thing"]["properties"]["title"] = json!({"type": "string"});
        now["components"]["schemas"]["Code"]["enum"] = json!(["auth.wrong-credentials", "x.new"]);
        now["components"]["schemas"]["Other"] = json!({"type": "object"});
        now["paths"]["/api/v1/more"] =
            json!({"get": {"operationId": "getMore", "responses": {"200": {"description": "ok"}}}});
        assert!(messages(&spec(), &now).is_empty());
        assert!(messages(&spec(), &spec()).is_empty());
    }

    #[test]
    fn bad_example_a_rename_or_a_removal_fails_for_each_name_that_is_gone() {
        let mut now = spec();
        now["components"]["schemas"]["Thing"]["properties"]
            .as_object_mut()
            .unwrap()
            .remove("id");
        now["components"]["schemas"]["Code"]["enum"] = json!(["auth.wrong_credentials"]);
        now["paths"]["/api/v1/things/{thing}"]["get"]["operationId"] = json!("readThing");
        let found = messages(&spec(), &now).join("\n");
        assert!(found.contains("`property Thing.id`"), "{found}");
        assert!(
            found.contains("`value Code auth.wrong-credentials`"),
            "{found}"
        );
        assert!(
            found.contains("`operation GET /api/v1/things/{thing} getThing`"),
            "{found}"
        );
    }

    #[test]
    fn bad_example_a_removed_route_parameter_answer_and_schema_fail() {
        let mut now = spec();
        now["paths"] = json!({});
        now["components"]["schemas"] = json!({});
        let found = messages(&spec(), &now).join("\n");
        assert!(found.contains("`path /api/v1/things/{thing}`"), "{found}");
        assert!(found.contains("`schema Thing`"), "{found}");
        // A parameter of a route that is still there, and an answer of it.
        let mut now = spec();
        now["paths"]["/api/v1/things/{thing}"]["get"]["parameters"] =
            json!([{"name": "thing", "in": "path"}]);
        now["paths"]["/api/v1/things/{thing}"]["get"]["responses"]
            .as_object_mut()
            .unwrap()
            .remove("404");
        let found = messages(&spec(), &now).join("\n");
        assert!(
            found.contains("`parameter GET /api/v1/things/{thing} query fields`"),
            "{found}"
        );
        assert!(
            found.contains("`answer GET /api/v1/things/{thing} 404`"),
            "{found}"
        );
    }

    #[test]
    fn a_new_variant_is_an_addition_and_a_long_list_of_findings_is_cut() {
        let mut now = spec();
        now["components"]["schemas"]["Thing"]["properties"]["status"]["oneOf"]
            .as_array_mut()
            .unwrap()
            .push(json!({"type": "string", "enum": ["archived"]}));
        assert!(messages(&spec(), &now).is_empty());
        let mut frozen = spec();
        for number in 0..60 {
            frozen["components"]["schemas"][format!("Old{number}")] = json!({"type": "object"});
        }
        let found = messages(&frozen, &spec());
        assert_eq!(found.len(), MOST_FINDINGS + 1);
        assert!(found[MOST_FINDINGS].contains("And 20 more"), "{found:?}");
    }
}
