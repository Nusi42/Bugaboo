//! ARCH-LEAN: every list route supports `fields=`, cursor paging and a page-size cap (PERF-02).
//!
//! Read from the generated OpenAPI document, so a new list route is found without anyone
//! registering it. A list is **any array property** of the object a `GET` answers with, not only
//! one called `items`, so a list cannot dodge the rule by its name. An answer that is itself a
//! bare array is refused: it has no room for `next_cursor`. A list called `items` must
//! take the query parameters `fields` and `cursor` and a `limit` with a `maximum`, and its answer
//! must hold `next_cursor`. A list with another name is only allowed when it is named in
//! `tools/checks/lean-exemptions.toml` with a reason that names the follow-up (`M1-F44`): the
//! lists that are small by nature (the sessions of one person) have a hard cap instead, and their
//! answer must say when the cap cut the list off (`truncated`); an array that is the whole value of
//! one setting (`whole_value`) needs no flag. An exemption that matches no list fails, so the file
//! cannot go stale.
//!
//! Two more rules (PERF-03, PERF-02). Every `GET` documents the `ETag` header of its `200` answer
//! and the `304` answer, so a client can ask again cheaply. A `GET` marked `x-never-stored` (its
//! answer holds personal data and no cache keeps it) has no tag to ask with and is left out. A list names related things by ID
//! (or by a reference schema whose name ends in `Ref`, like `PersonRef`); an item that embeds
//! another object is only allowed when the route declares the query parameter `include` with the
//! closed set of names it may embed, and `include` is only allowed on a route that embeds.

use bb_api_words::marks::NEVER_STORED;
use bb_api_words::paging::{CURSOR, FIELDS, INCLUDE, ITEMS, LIMIT, NEXT_CURSOR, TRUNCATED};
use serde::Deserialize;
use serde_json::Value;

use super::api_spec::{self, resolve};
use super::exemptions::{self, Used};
use super::{Check, Context, Finding};

/// The parameters a list route takes.
const REQUIRED_PARAMETERS: [&str; 3] = [FIELDS, CURSOR, LIMIT];
/// Where the exemptions are written down, each with its reason.
const EXEMPTIONS_PATH: &str = "tools/checks/lean-exemptions.toml";
/// The header every `GET` answer documents, and the status of the answer to a match.
const ETAG_HEADER: &str = "etag";
const NOT_MODIFIED: &str = "304";
/// A schema whose name ends like this is a reference to a thing, not the thing itself.
const REFERENCE_SUFFIX: &str = "Ref";

pub(crate) struct ArchLean;

impl Check for ArchLean {
    fn id(&self) -> &'static str {
        "ARCH-LEAN"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let spec = api_spec::load(context.repo)?;
        let exemptions: Exemptions = exemptions::load(context.repo, EXEMPTIONS_PATH)?;
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

impl exemptions::Exemption for Exemption {
    fn name(&self) -> String {
        format!("{} in {}", self.property, self.route)
    }

    fn reason(&self) -> &str {
        &self.reason
    }

    fn unused(&self) -> String {
        "matches no list in the API description. Remove it.".to_owned()
    }
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
            Some(parameter) if name == LIMIT => {
                let maximum = parameter.pointer("/schema/maximum");
                if !maximum.is_some_and(Value::is_number) {
                    findings.push(Finding::at(
                        place,
                        format!("The parameter `{LIMIT}` must have a `maximum`, so one request cannot ask for everything."),
                    ));
                }
            }
            Some(_) => {}
        }
    }
    if schema
        .pointer(&format!("/properties/{NEXT_CURSOR}"))
        .is_none()
    {
        findings.push(Finding::at(
            place,
            format!("The answer of a list route must hold `{NEXT_CURSOR}`."),
        ));
    }
    findings
}

/// The rule of the entity tag: the `200` answer documents the header, and `304` exists.
fn etag_findings(spec: &Value, operation: &Value, place: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let ok = operation
        .pointer("/responses/200")
        .map(|ok| resolve(spec, ok));
    let documents_header = ok
        .and_then(|ok| ok.get("headers"))
        .and_then(Value::as_object)
        .is_some_and(|headers| {
            headers
                .keys()
                .any(|name| name.eq_ignore_ascii_case(ETAG_HEADER))
        });
    if !documents_header {
        findings.push(Finding::at(
            place,
            "The `200` answer of a GET must document the `ETag` header, so a client can ask again with `If-None-Match` (PERF-03).",
        ));
    }
    if operation
        .pointer(&format!("/responses/{NOT_MODIFIED}"))
        .is_none()
    {
        findings.push(Finding::at(
            place,
            format!("A GET must document the `{NOT_MODIFIED}` answer to a request that already has the current `ETag` (PERF-03)."),
        ));
    }
    findings
}

/// The schemas a property can stand for: itself, what a nullable or combined type is made of, and
/// the items of an array.
fn candidates<'a>(spec: &'a Value, property: &'a Value) -> Vec<&'a Value> {
    let mut found = vec![property];
    for key in ["anyOf", "oneOf", "allOf"] {
        for member in property
            .get(key)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            found.push(member);
        }
    }
    if let Some(items) = resolve(spec, property).get("items") {
        found.push(items);
    }
    found
}

/// Whether a property of an item holds another object instead of its ID.
fn embeds(spec: &Value, property: &Value) -> bool {
    candidates(spec, property).into_iter().any(|candidate| {
        let reference = candidate
            .get("$ref")
            .and_then(Value::as_str)
            .and_then(|reference| reference.rsplit('/').next());
        if reference.is_some_and(|name| name.ends_with(REFERENCE_SUFFIX)) {
            return false;
        }
        let resolved = resolve(spec, candidate);
        let object = resolved.get("type").and_then(Value::as_str) == Some("object")
            || resolved
                .get("type")
                .and_then(Value::as_array)
                .is_some_and(|kinds| kinds.iter().any(|kind| kind.as_str() == Some("object")));
        object && resolved.get("properties").is_some()
    })
}

/// The properties of the items of a paged list that embed another object.
fn embedded_properties(spec: &Value, schema: &Value) -> Vec<String> {
    let Some(item) = schema
        .pointer(&format!("/properties/{ITEMS}/items"))
        .map(|item| resolve(spec, item))
    else {
        return Vec::new();
    };
    item.get("properties")
        .and_then(Value::as_object)
        .map(|properties| {
            properties
                .iter()
                .filter(|(_, property)| embeds(spec, property))
                .map(|(name, _)| name.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// The rule of `include`: declared, with a closed set of names, exactly when the items embed.
fn include_findings(spec: &Value, operation: &Value, schema: &Value, place: &str) -> Vec<Finding> {
    let include = operation
        .get("parameters")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|parameter| resolve(spec, parameter))
        .find(|parameter| {
            parameter.get("name").and_then(Value::as_str) == Some(INCLUDE)
                && parameter.get("in").and_then(Value::as_str) == Some("query")
        });
    let embedded = embedded_properties(spec, schema);
    match (include, embedded.is_empty()) {
        (None, false) => vec![Finding::at(
            place,
            format!(
                "The items embed `{}`. Name related things by ID (or a `{REFERENCE_SUFFIX}` schema), or declare the query parameter `{INCLUDE}` with the names that may be embedded.",
                embedded.join("`, `")
            ),
        )],
        (Some(_), true) => vec![Finding::at(
            place,
            format!("The route declares `{INCLUDE}` but its items embed nothing. Remove it."),
        )],
        (Some(parameter), false) => {
            let closed = ["/schema/enum", "/schema/items/enum"]
                .iter()
                .any(|pointer| parameter.pointer(pointer).is_some_and(Value::is_array));
            if closed {
                Vec::new()
            } else {
                vec![Finding::at(
                    place,
                    format!(
                        "The parameter `{INCLUDE}` must list the names that may be embedded (an `enum`)."
                    ),
                )]
            }
        }
        (None, true) => Vec::new(),
    }
}

fn evaluate(spec: &Value, exemptions: &[Exemption]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let Some(operations) = api_spec::operations(spec) else {
        return vec![api_spec::no_paths()];
    };
    let mut used = Used::default();
    for found in operations.iter().filter(|found| found.method == "GET") {
        let operation = found.body;
        let place = found.place();
        if operation.get(NEVER_STORED) != Some(&Value::Bool(true)) {
            findings.extend(etag_findings(spec, operation, &place));
        }
        let Some(schema) = answer_schema(spec, operation) else {
            continue;
        };
        // An answer that is a bare array has no room for `next_cursor` or `truncated`, so it is
        // never allowed: wrap it in an object with a list called `items`.
        if is_array(spec, schema) {
            findings.push(Finding::at(
                &place,
                format!("The answer is a bare array. Answer with an object that holds the list as `{ITEMS}`, so there is room for `{NEXT_CURSOR}`."),
            ));
            continue;
        }
        for property in array_properties(spec, schema) {
            let exemption = exemptions
                .iter()
                .position(|exemption| exemption.route == place && exemption.property == property);
            if let Some(index) = exemption {
                let exemption = &exemptions[index];
                used.mark(index);
                let flag = schema.pointer(&format!("/properties/{TRUNCATED}/type"));
                if !exemption.whole_value && flag.and_then(Value::as_str) != Some("boolean") {
                    findings.push(Finding::at(
                        &place,
                        format!("A list with a hard cap must say when the cap cut it off: the answer needs a boolean `{TRUNCATED}`."),
                    ));
                }
            } else if property == ITEMS {
                findings.extend(paging_findings(spec, operation, schema, &place));
                findings.extend(include_findings(spec, operation, schema, &place));
            } else {
                findings.push(Finding::at(
                    &place,
                    format!("The answer has the list `{property}`. A list is called `{ITEMS}` and takes `{FIELDS}`, `{CURSOR}` and `{LIMIT}`; or, when it is small by nature, it is named in {EXEMPTIONS_PATH} with a reason."),
                ));
            }
        }
    }
    findings.extend(exemptions::judge(EXEMPTIONS_PATH, exemptions, &used));
    findings.extend(exemptions::follow_up_findings(EXEMPTIONS_PATH, exemptions));
    findings
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
                    "responses": {
                        "200": {"description": "ok", "headers": {"etag": {}}, "content": {"application/json": {"schema": {"$ref": "#/components/schemas/Page"}}}},
                        "304": {"description": "not modified"}
                    }
                }},
                "/api/v1/status": {"get": {"responses": {
                    "200": {"headers": {"ETag": {}}, "content": {"application/json": {"schema": {"type": "object", "properties": {"status": {"type": "string"}}}}}},
                    "304": {"description": "not modified"}
                }}}
            },
            "components": {"schemas": {"Page": schema}}
        })
    }

    fn page(list: &str, with_cursor: bool) -> Value {
        let mut properties = json!({ list: {"type": "array", "items": {}}});
        if with_cursor {
            properties[NEXT_CURSOR] = json!({"type": "string"});
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
            schema["properties"][TRUNCATED] = json!({"type": "boolean"});
        }
        schema
    }

    #[test]
    fn good_example() {
        let good = spec_with(&parameters(&REQUIRED_PARAMETERS, true), &page(ITEMS, true));
        assert!(messages(&good, &[]).is_empty());
    }

    #[test]
    fn bad_example() {
        let bad = spec_with(&parameters(&[LIMIT], false), &page(ITEMS, false));
        let found = messages(&bad, &[]).join("\n");
        assert!(found.contains("`fields`"), "{found}");
        assert!(found.contains("`cursor`"), "{found}");
        assert!(found.contains("must have a `maximum`"), "{found}");
        assert!(found.contains(NEXT_CURSOR), "{found}");
    }

    #[test]
    fn bad_example_a_list_that_is_not_called_items_does_not_dodge_the_rule() {
        let bad = spec_with(&parameters(&[], false), &page("issues", false));
        let found = messages(&bad, &[]).join("\n");
        assert!(found.contains("the list `issues`"), "{found}");
    }

    #[test]
    fn bad_example_a_get_that_answers_with_a_bare_array() {
        let bare = spec_with(
            &parameters(&REQUIRED_PARAMETERS, true),
            &json!({"type": "array", "items": {"type": "string"}}),
        );
        let found = messages(&bare, &[]).join(
            "
",
        );
        assert!(found.contains("bare array"), "{found}");
    }

    #[test]
    fn good_example_a_get_that_answers_with_an_object_around_its_list() {
        let wrapped = spec_with(&parameters(&REQUIRED_PARAMETERS, true), &page(ITEMS, true));
        assert!(messages(&wrapped, &[]).is_empty());
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
    fn routes_that_are_not_lists_are_left_alone() {
        let spec = json!({"paths": {"/api/v1/status": {"get": {"responses": {
            "200": {"headers": {"etag": {}}, "content": {"application/json": {"schema": {"type": "object"}}}},
            "304": {}
        }}}}});
        assert!(messages(&spec, &[]).is_empty());
    }

    #[test]
    fn a_document_without_paths_is_a_finding() {
        assert_eq!(messages(&json!({}), &[]).len(), 1);
    }

    /// The fixtures: small API descriptions in `tools/checks/fixtures/ARCH-LEAN/{good,bad}`.
    fn fixture(kind: &str, name: &str) -> Value {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tools/checks/fixtures/ARCH-LEAN")
            .join(kind)
            .join(name);
        let text = std::fs::read_to_string(&path).unwrap();
        serde_json::from_str(&text).unwrap()
    }

    #[test]
    fn good_example_fixtures_pass() {
        for name in ["references-by-id.json", "embeds-with-include.json"] {
            let found = messages(&fixture("good", name), &[]);
            assert!(found.is_empty(), "{name}: {found:?}");
        }
    }

    #[test]
    fn bad_example_fixtures_fail_each_for_its_own_reason() {
        for (name, expected) in [
            ("no-etag-header.json", "document the `ETag` header"),
            ("no-not-modified-answer.json", "`304` answer"),
            ("embeds-without-include.json", "The items embed `project`"),
            ("include-without-embedding.json", "embed nothing"),
            ("include-without-names.json", "an `enum`"),
        ] {
            let found = messages(&fixture("bad", name), &[]);
            assert_eq!(found.len(), 1, "{name}: {found:?}");
            assert!(found[0].contains(expected), "{name}: {found:?}");
        }
    }

    #[test]
    fn good_example_a_read_that_is_never_stored_has_no_tag_to_document() {
        let found = messages(&fixture("good", "never-stored.json"), &[]);
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_get_without_a_json_answer_still_needs_the_tag_and_the_not_modified_answer() {
        let spec = json!({"paths": {"/api/v1/file": {"get": {"responses": {"200": {"description": "bytes"}}}}}});
        assert_eq!(messages(&spec, &[]).len(), 2);
    }
}
