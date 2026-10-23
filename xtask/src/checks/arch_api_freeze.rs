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
//! The comparison is made on *facts*: one line of text for each name, type, answer, header, media type
//! and rule of requiredness that a client can depend on (an answer member stays always there, a
//! request member stays optional, a limit of a request is never tightened). `cargo xtask freeze`
//! refuses a copy that lacks a fact of the one at the tag `m4-done`, or at the merge base with main.
//! The members of `oneOf`, `anyOf` and `allOf` count as the schema that holds them, so a new
//! variant is an addition and a property that moves between variants is not seen.

use std::collections::{BTreeMap, BTreeSet};

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
/// The copy that a new frozen copy must hold: the one at the tag `m4-done` when it exists, else the
/// one at the merge base with `main`. `None` when there is none yet.
fn copy_to_keep(repo: &Repo) -> Option<Value> {
    let git = |arguments: &[&str]| {
        let output = std::process::Command::new("git")
            .args(arguments)
            .current_dir(repo.root())
            .output()
            .ok()?;
        output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
    };
    let reference = if git(&["rev-parse", "--verify", "--quiet", "refs/tags/m4-done"]).is_some() {
        "m4-done".to_owned()
    } else {
        let main = ["origin/main", "main"]
            .into_iter()
            .find(|name| git(&["rev-parse", "--verify", "--quiet", name]).is_some())?;
        git(&["merge-base", "HEAD", main])?
    };
    let text = git(&["show", &format!("{reference}:{FROZEN_PATH}")])?;
    serde_json::from_str(&text).ok()
}

pub(crate) fn write_frozen(repo: &Repo) -> anyhow::Result<()> {
    let spec = api_spec::load(repo)?;
    if let Some(kept) = copy_to_keep(repo) {
        let gone = broken(&kept, &spec);
        if !gone.is_empty() {
            anyhow::bail!(
                "a new frozen copy must hold everything of the one at m4-done (or at the merge base with main); {} facts are gone, for example: {}",
                gone.len(),
                gone.iter().take(5).cloned().collect::<Vec<_>>().join("; ")
            );
        }
    }
    let compact = serde_json::to_string(&spec).context("the API description is not JSON")?;
    std::fs::write(repo.path(FROZEN_PATH), format!("{compact}\n"))
        .with_context(|| format!("could not write {FROZEN_PATH}"))
}

/// What a client can depend on in the description.
///
/// `names` holds one line of text for each fact that must stay (a frozen fact that the new
/// description lacks is a break). `upper` and `lower` hold the limits of what a request may hold: a
/// limit may be loosened and never tightened.
#[derive(Default)]
struct Facts {
    names: BTreeSet<String>,
    upper: BTreeMap<String, f64>,
    lower: BTreeMap<String, f64>,
}

/// The upper and lower limits of validation.
const UPPER: [&str; 3] = ["maximum", "maxLength", "maxItems"];
const LOWER: [&str; 3] = ["minimum", "minLength", "minItems"];

/// Which side of the API reaches a schema: what a client sends, and what it reads.
#[derive(Clone, Copy, Default)]
struct Side {
    request: bool,
    answer: bool,
}

/// The name of the schema a `$ref` points to.
fn ref_name(reference: &str) -> &str {
    reference.rsplit('/').next().unwrap_or(reference)
}

/// What a schema is made of, as words: the schemas it refers to and the types it has (`null`
/// included), `oneOf`, `anyOf` and `allOf` seen through, an array as `[]` and what its items are.
fn tokens(schema: &Value) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        found.insert(ref_name(reference).to_owned());
    }
    match schema.get("type") {
        Some(Value::String(kind)) => {
            found.insert(kind.clone());
        }
        Some(Value::Array(kinds)) => {
            found.extend(kinds.iter().filter_map(Value::as_str).map(str::to_owned));
        }
        _ => {}
    }
    for key in ["oneOf", "anyOf", "allOf"] {
        for member in schema
            .get(key)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            found.extend(tokens(member));
        }
    }
    if let Some(items) = schema.get("items") {
        found.extend(tokens(items).into_iter().map(|token| format!("[]{token}")));
    }
    found
}

/// The names of the schemas that `roots` reach, `$ref` after `$ref`.
fn reach(spec: &Value, roots: &[&Value]) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut pending: Vec<&Value> = roots.to_vec();
    while let Some(value) = pending.pop() {
        match value {
            Value::Object(members) => {
                if let Some(reference) = members.get("$ref").and_then(Value::as_str)
                    && let Some(pointer) = reference.strip_prefix('#')
                    && seen.insert(ref_name(reference).to_owned())
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

/// The schemas of the bodies of a request, and of its parameters.
fn request_roots<'a>(spec: &'a Value, operation: &'a Value) -> Vec<&'a Value> {
    let mut roots: Vec<&Value> = operation
        .get("parameters")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|parameter| resolve(spec, parameter))
        .collect();
    if let Some(body) = operation.get("requestBody") {
        roots.push(resolve(spec, body));
    }
    roots
}

/// The answers of an operation, `$ref` followed.
fn answers<'a>(spec: &'a Value, operation: &'a Value) -> Vec<(&'a str, &'a Value)> {
    operation
        .get("responses")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .map(|(status, answer)| (status.as_str(), resolve(spec, answer)))
        .collect()
}

fn facts(spec: &Value) -> Facts {
    let mut found = Facts::default();
    let mut requested = Vec::new();
    let mut answered = Vec::new();
    for operation in api_spec::operations(spec).into_iter().flatten() {
        requested.extend(request_roots(spec, operation.body));
        answered.extend(answers(spec, operation.body).into_iter().map(|(_, a)| a));
    }
    path_facts(spec, &mut found);
    let requests = reach(spec, &requested);
    let replies = reach(spec, &answered);
    for (name, schema) in api_spec::schemas(spec) {
        found.names.insert(format!("schema {name}"));
        let side = Side {
            request: requests.contains(name),
            answer: replies.contains(name),
        };
        schema_facts(schema, name, side, &mut found);
    }
    found
}

fn limits(schema: &Value, at: &str, found: &mut Facts) {
    for key in UPPER {
        if let Some(value) = schema.get(key).and_then(Value::as_f64) {
            found.upper.insert(format!("{at} {key}"), value);
        }
    }
    for key in LOWER {
        if let Some(value) = schema.get(key).and_then(Value::as_f64) {
            found.lower.insert(format!("{at} {key}"), value);
        }
    }
}

fn path_facts(spec: &Value, found: &mut Facts) {
    let Some(paths) = spec.get("paths").and_then(Value::as_object) else {
        return;
    };
    for path in paths.keys() {
        found.names.insert(format!("path {path}"));
    }
    for operation in api_spec::operations(spec).into_iter().flatten() {
        let place = operation.place();
        found.names.insert(format!(
            "operation {place} {}",
            operation.text("operationId").unwrap_or_default()
        ));
        if let Some(body) = operation.body.get("requestBody") {
            found.names.insert(format!("body {place}"));
            let body = resolve(spec, body);
            for (media, content) in body
                .get("content")
                .and_then(Value::as_object)
                .into_iter()
                .flatten()
            {
                found.names.insert(format!("request-media {place} {media}"));
                if let Some(schema) = content.get("schema") {
                    for token in tokens(schema) {
                        found
                            .names
                            .insert(format!("request {place} {media} {token}"));
                    }
                }
            }
        }
        for parameter in operation
            .body
            .get("parameters")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let parameter = resolve(spec, parameter);
            let kind = parameter
                .get("in")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let name = parameter
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let at = format!("{place} {kind} {name}");
            found.names.insert(format!("parameter {at}"));
            if parameter.get("required").and_then(Value::as_bool) != Some(true) {
                found.names.insert(format!("optional-parameter {at}"));
            }
            if let Some(schema) = parameter.get("schema") {
                for token in tokens(schema) {
                    found.names.insert(format!("parameter-type {at} {token}"));
                }
                limits(schema, &format!("parameter {at}"), found);
            }
        }
        for (status, answer) in answers(spec, operation.body) {
            found.names.insert(format!("answer {place} {status}"));
            for header in answer
                .get("headers")
                .and_then(Value::as_object)
                .into_iter()
                .flat_map(serde_json::Map::keys)
            {
                found
                    .names
                    .insert(format!("header {place} {status} {header}"));
            }
            for (media, content) in answer
                .get("content")
                .and_then(Value::as_object)
                .into_iter()
                .flatten()
            {
                found
                    .names
                    .insert(format!("media {place} {status} {media}"));
                if let Some(schema) = content.get("schema") {
                    for token in tokens(schema) {
                        found
                            .names
                            .insert(format!("answer-schema {place} {status} {media} {token}"));
                    }
                }
            }
        }
    }
}

/// The properties, types, requiredness, limits and enum values of a schema, below the name `at`.
fn schema_facts(schema: &Value, at: &str, side: Side, found: &mut Facts) {
    let required: BTreeSet<&str> = schema
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
        for (name, property) in properties {
            let here = format!("{at}.{name}");
            found.names.insert(format!("property {here}"));
            let kinds = tokens(property);
            for kind in &kinds {
                found.names.insert(format!("type {here} {kind}"));
            }
            // What a reader can count on is always there; what a writer may leave out stays optional.
            if side.answer && required.contains(name.as_str()) && !kinds.contains("null") {
                found.names.insert(format!("always {here}"));
            }
            if side.request && !required.contains(name.as_str()) {
                found.names.insert(format!("optional {here}"));
            }
            schema_facts(property, &here, side, found);
        }
    }
    if side.request {
        limits(schema, at, found);
    }
    for key in ["enum", "const"] {
        let values: Vec<&Value> = match schema.get(key) {
            Some(Value::Array(values)) => values.iter().collect(),
            Some(value) => vec![value],
            None => Vec::new(),
        };
        for value in values.into_iter().filter_map(Value::as_str) {
            found.names.insert(format!("value {at} {value}"));
        }
    }
    if let Some(items) = schema.get("items") {
        schema_facts(items, &format!("{at}[]"), side, found);
    }
    if let Some(extra) = schema.get("additionalProperties") {
        schema_facts(extra, &format!("{at}{{}}"), side, found);
    }
    for key in ["oneOf", "anyOf", "allOf"] {
        for member in schema
            .get(key)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            schema_facts(member, at, side, found);
        }
    }
}

/// What the frozen description has and the current one lacks or tightened.
fn broken(frozen: &Value, current: &Value) -> Vec<String> {
    let (old, now) = (facts(frozen), facts(current));
    let mut gone: Vec<String> = old
        .names
        .iter()
        .filter(|fact| !now.names.contains(*fact))
        .cloned()
        .collect();
    for (key, was) in &old.upper {
        if now.upper.get(key).is_some_and(|is| is < was) {
            gone.push(format!(
                "limit {key} {was} (tightened to {})",
                now.upper[key]
            ));
        }
    }
    for (key, was) in &old.lower {
        if now.lower.get(key).is_some_and(|is| is > was) {
            gone.push(format!(
                "limit {key} {was} (tightened to {})",
                now.lower[key]
            ));
        }
    }
    gone
}

fn evaluate(frozen: &Value, current: &Value) -> Vec<Finding> {
    let gone = broken(frozen, current);
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

    fn typed() -> Value {
        json!({
            "paths": {
                "/api/v1/things": {
                    "post": {
                        "operationId": "createThing",
                        "parameters": [{"name": "limit", "in": "query", "schema": {"type": "integer", "maximum": 100}}],
                        "requestBody": {"content": {"application/json": {"schema": {"$ref": "#/components/schemas/NewThing"}}}},
                        "responses": {"200": {
                            "headers": {"etag": {"schema": {"type": "string"}}},
                            "content": {"application/json": {"schema": {"$ref": "#/components/schemas/Thing"}}}
                        }}
                    }
                }
            },
            "components": {"schemas": {
                "NewThing": {"type": "object", "required": ["name"], "properties": {
                    "name": {"type": "string"}, "note": {"type": "string"}
                }},
                "Thing": {"type": "object", "required": ["id", "title"], "properties": {
                    "id": {"type": "string"}, "title": {"type": "string"}
                }}
            }}
        })
    }

    #[test]
    fn good_example_an_optional_request_property_a_new_answer_member_and_a_new_operation() {
        let mut now = typed();
        now["components"]["schemas"]["NewThing"]["properties"]["tag"] = json!({"type": "string"});
        now["components"]["schemas"]["Thing"]["properties"]["extra"] = json!({"type": "string"});
        now["components"]["schemas"]["NewThing"]["properties"]["note"]["maxLength"] = json!(500);
        now["paths"]["/api/v1/other"] = json!({"get": {"operationId": "getOther", "responses": {"200": {"description": "ok"}}}});
        assert!(messages(&typed(), &now).is_empty());
        // A limit that is loosened passes.
        now["paths"]["/api/v1/things"]["post"]["parameters"][0]["schema"]["maximum"] = json!(200);
        assert!(messages(&typed(), &now).is_empty());
    }

    #[test]
    fn bad_example_a_type_a_required_member_and_an_answer_that_are_not_what_they_were() {
        let found = |change: &dyn Fn(&mut Value)| {
            let mut now = typed();
            change(&mut now);
            messages(&typed(), &now).join("\n")
        };
        // A property of another type.
        let text = found(&|now| {
            now["components"]["schemas"]["Thing"]["properties"]["id"] = json!({"type": "integer"});
        });
        assert!(text.contains("`type Thing.id string`"), "{text}");
        // A request property that became required.
        let text = found(&|now| {
            now["components"]["schemas"]["NewThing"]["required"] = json!(["name", "note"]);
        });
        assert!(text.contains("`optional NewThing.note`"), "{text}");
        // An answer member that may be missing or null now.
        let text = found(&|now| {
            now["components"]["schemas"]["Thing"]["required"] = json!(["id"]);
        });
        assert!(text.contains("`always Thing.title`"), "{text}");
        let text = found(&|now| {
            now["components"]["schemas"]["Thing"]["properties"]["title"] =
                json!({"type": ["string", "null"]});
        });
        assert!(text.contains("`always Thing.title`"), "{text}");
        // Another request schema, answer schema, header and media type.
        let text = found(&|now| {
            let post = &mut now["paths"]["/api/v1/things"]["post"];
            post["requestBody"]["content"]["application/json"]["schema"] =
                json!({"$ref": "#/components/schemas/Thing"});
            post["responses"]["200"]["content"]["application/json"]["schema"] =
                json!({"$ref": "#/components/schemas/NewThing"});
            post["responses"]["200"]["headers"] = json!({});
            post["responses"]["200"]["content"] =
                json!({"text/plain": {"schema": {"type": "string"}}});
        });
        assert!(
            text.contains("`request POST /api/v1/things application/json NewThing`"),
            "{text}"
        );
        assert!(
            text.contains("answer-schema POST /api/v1/things 200 application/json Thing"),
            "{text}"
        );
        assert!(
            text.contains("header POST /api/v1/things 200 etag"),
            "{text}"
        );
        assert!(
            text.contains("media POST /api/v1/things 200 application/json"),
            "{text}"
        );
    }

    #[test]
    fn bad_example_a_tightened_limit_and_a_parameter_that_became_required() {
        let mut now = typed();
        now["paths"]["/api/v1/things"]["post"]["parameters"][0]["schema"]["maximum"] = json!(50);
        now["paths"]["/api/v1/things"]["post"]["parameters"][0]["required"] = json!(true);
        let text = messages(&typed(), &now).join("\n");
        assert!(text.contains("maximum 100 (tightened to 50)"), "{text}");
        assert!(
            text.contains("optional-parameter POST /api/v1/things query limit"),
            "{text}"
        );
        // A request property with an upper limit that is lowered.
        let mut frozen = typed();
        frozen["components"]["schemas"]["NewThing"]["properties"]["note"]["maxLength"] = json!(500);
        let mut now = frozen.clone();
        now["components"]["schemas"]["NewThing"]["properties"]["note"]["maxLength"] = json!(100);
        let text = messages(&frozen, &now).join("\n");
        assert!(text.contains("maxLength 500 (tightened to 100)"), "{text}");
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
