//! ARCH-LEAN: every list route supports `fields=`, cursor paging and a page-size cap (PERF-02).
//!
//! Read from the generated OpenAPI document, so a new list route is found without anyone
//! registering it. A list route is a `GET` whose successful JSON answer is an object with an
//! `items` list. It must take the query parameters `fields` and `cursor` and a `limit` with a
//! `maximum`, and its answer must hold `next_cursor`.

use anyhow::Context as _;
use serde_json::Value;

use super::{Check, Context, Finding};
use crate::generate::api::SPEC_PATH;

/// The parameters a list route takes.
const REQUIRED_PARAMETERS: [&str; 3] = ["fields", "cursor", "limit"];

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
        Ok(evaluate(&spec))
    }
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

fn is_list(schema: &Value) -> bool {
    schema
        .pointer("/properties/items/type")
        .and_then(Value::as_str)
        == Some("array")
}

fn evaluate(spec: &Value) -> Vec<Finding> {
    let mut findings = Vec::new();
    let Some(paths) = spec.get("paths").and_then(Value::as_object) else {
        return vec![Finding::at(SPEC_PATH, "The document has no `paths`.")];
    };
    for (path, item) in paths {
        let Some(operation) = item.get("get") else {
            continue;
        };
        let Some(schema) = answer_schema(spec, operation).filter(|schema| is_list(schema)) else {
            continue;
        };
        let place = format!("GET {path}");
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
                    &place,
                    format!("A list route must take the query parameter `{name}` (lean answers, paging)."),
                )),
                Some(parameter) if name == "limit" => {
                    let maximum = parameter.pointer("/schema/maximum");
                    if !maximum.is_some_and(Value::is_number) {
                        findings.push(Finding::at(
                            &place,
                            "The parameter `limit` must have a `maximum`, so one request cannot ask for everything.",
                        ));
                    }
                }
                Some(_) => {}
            }
        }
        if schema.pointer("/properties/next_cursor").is_none() {
            findings.push(Finding::at(
                &place,
                "The answer of a list route must hold `next_cursor`.",
            ));
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn spec(parameters: &Value, schema: &Value) -> Value {
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

    fn page(with_cursor: bool) -> Value {
        let mut properties = json!({"items": {"type": "array", "items": {}}});
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

    fn messages(spec: &Value) -> Vec<String> {
        evaluate(spec).into_iter().map(|f| f.message).collect()
    }

    #[test]
    fn good_example() {
        let good = spec(
            &parameters(&["fields", "cursor", "limit"], true),
            &page(true),
        );
        assert!(messages(&good).is_empty());
    }

    #[test]
    fn bad_example() {
        let bad = spec(&parameters(&["limit"], false), &page(false));
        let found = messages(&bad).join("\n");
        assert!(found.contains("`fields`"), "{found}");
        assert!(found.contains("`cursor`"), "{found}");
        assert!(found.contains("must have a `maximum`"), "{found}");
        assert!(found.contains("next_cursor"), "{found}");
    }

    #[test]
    fn routes_that_are_not_lists_are_left_alone() {
        let spec = json!({"paths": {"/api/v1/status": {"get": {"responses": {"200": {"content": {"application/json": {"schema": {"type": "object"}}}}}}}}});
        assert!(messages(&spec).is_empty());
    }

    #[test]
    fn a_document_without_paths_is_a_finding() {
        assert_eq!(messages(&json!({})).len(), 1);
    }
}
