//! Reading the API description (`web/packages/api-client/openapi.json`) for the checks that look
//! at the routes and their answers. Every check asks here; none reads or walks the document by
//! itself.

use anyhow::Context as _;
use serde_json::Value;

use super::Finding;
use crate::generate::api::SPEC_PATH;
use crate::repo::Repo;
use crate::source::read_text;

/// The methods an OpenAPI path item can hold an operation under.
const METHODS: [&str; 8] = [
    "get", "put", "post", "delete", "options", "head", "patch", "trace",
];

/// Reads the API description of the repository.
pub(super) fn load(repo: &Repo) -> anyhow::Result<Value> {
    let text = read_text(repo, SPEC_PATH)
        .with_context(|| format!("could not read {SPEC_PATH}; run `cargo xtask gen api`"))?;
    parse(&text)
}

/// Parses the text of an API description.
pub(super) fn parse(text: &str) -> anyhow::Result<Value> {
    serde_json::from_str(text).with_context(|| format!("{SPEC_PATH} is not valid JSON"))
}

/// Follows a `$ref` that points into the document itself.
pub(super) fn resolve<'a>(spec: &'a Value, value: &'a Value) -> &'a Value {
    value
        .get("$ref")
        .and_then(Value::as_str)
        .and_then(|reference| reference.strip_prefix('#'))
        .and_then(|pointer| spec.pointer(pointer))
        .unwrap_or(value)
}

/// One operation of the description: a route with a method.
pub(super) struct Operation<'a> {
    /// The method in capitals (`GET`).
    pub(super) method: String,
    /// The path as the description writes it (`/api/v1/projects/{id}`).
    pub(super) path: &'a str,
    /// The operation object itself.
    pub(super) body: &'a Value,
}

impl<'a> Operation<'a> {
    /// The route as a person names it: `GET /api/v1/projects`.
    pub(super) fn place(&self) -> String {
        format!("{} {}", self.method, self.path)
    }

    /// The text field `key` of the operation (`operationId`, an `x-` mark).
    pub(super) fn text(&self, key: &str) -> Option<&'a str> {
        self.body.get(key).and_then(Value::as_str)
    }
}

/// The paths of the description with their items, in the order of the document. `None` when the
/// document has no `paths` at all.
pub(super) fn paths(spec: &Value) -> Option<Vec<(&str, &Value)>> {
    let paths = spec.get("paths").and_then(Value::as_object)?;
    Some(
        paths
            .iter()
            .map(|(path, item)| (path.as_str(), item))
            .collect(),
    )
}

/// Every operation of the description, in the order of the document. `None` when the document has
/// no `paths` at all.
pub(super) fn operations(spec: &Value) -> Option<Vec<Operation<'_>>> {
    let mut found = Vec::new();
    for (path, item) in paths(spec)? {
        for method in METHODS {
            if let Some(body) = item.get(method) {
                found.push(Operation {
                    method: method.to_uppercase(),
                    path,
                    body,
                });
            }
        }
    }
    Some(found)
}

/// The finding for a description that has no `paths`.
pub(super) fn no_paths() -> Finding {
    Finding::at(SPEC_PATH, "The document has no `paths`.")
}

/// The schemas that the responses of an operation name, `$ref` followed, in any media type.
pub(super) fn answer_schemas<'a>(spec: &'a Value, operation: &'a Value) -> Vec<&'a Value> {
    operation
        .get("responses")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(serde_json::Map::values)
        .flat_map(|response| {
            resolve(spec, response)
                .get("content")
                .and_then(Value::as_object)
                .into_iter()
                .flat_map(serde_json::Map::values)
        })
        .filter_map(|media| media.get("schema"))
        .collect()
}

/// The schemas under `components.schemas`, each with its name.
pub(super) fn schemas(spec: &Value) -> Vec<(&str, &Value)> {
    spec.pointer("/components/schemas")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .map(|(name, schema)| (name.as_str(), schema))
        .collect()
}

/// The names under `components.schemas`.
pub(super) fn schema_names(spec: &Value) -> Vec<&str> {
    schemas(spec).into_iter().map(|(name, _)| name).collect()
}

/// The parameters under `components.parameters`.
pub(super) fn shared_parameters(spec: &Value) -> Vec<&Value> {
    spec.pointer("/components/parameters")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(serde_json::Map::values)
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn spec() -> Value {
        json!({
            "paths": {
                "/a": {"get": {"operationId": "getA", "responses": {
                    "200": {"$ref": "#/components/responses/Ok"}
                }}, "parameters": []},
                "/b": {"post": {"operationId": "postB"}, "patch": {"operationId": "patchB"}}
            },
            "components": {
                "responses": {"Ok": {"content": {"application/json": {"schema": {"$ref": "#/components/schemas/A"}}}}},
                "schemas": {"A": {"type": "object"}}
            }
        })
    }

    #[test]
    fn operations_are_the_methods_of_every_path_and_not_its_other_keys() {
        let spec = spec();
        let places: Vec<String> = operations(&spec)
            .unwrap()
            .iter()
            .map(Operation::place)
            .collect();
        assert_eq!(places, ["GET /a", "POST /b", "PATCH /b"]);
    }

    #[test]
    fn a_document_without_paths_has_no_operations() {
        assert!(operations(&json!({})).is_none());
    }

    #[test]
    fn answers_follow_the_references_of_responses_and_schemas() {
        let spec = spec();
        let operations = operations(&spec).unwrap();
        let schemas = answer_schemas(&spec, operations[0].body);
        assert_eq!(schemas.len(), 1);
        assert_eq!(
            resolve(&spec, schemas[0]).get("type"),
            Some(&json!("object"))
        );
        assert_eq!(schema_names(&spec), ["A"]);
    }

    #[test]
    fn a_text_that_is_not_json_names_the_description() {
        let error = parse("{").unwrap_err();
        assert!(format!("{error:#}").contains("openapi.json is not valid JSON"));
    }
}
