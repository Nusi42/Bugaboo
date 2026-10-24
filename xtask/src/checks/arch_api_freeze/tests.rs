//! The tests of ARCH-API-FREEZE: what the recursive comparison finds.

use serde_json::json;

use super::*;

/// A description with two operations: `POST /api/v1/things` takes `request` and `GET
/// /api/v1/things/{id}` answers `answer`; `schemas` are the components.
fn api(request: Value, answer: Value, schemas: Value) -> Value {
    let mut spec = json!({
        "paths": {
            "/api/v1/things": {"post": {
                "operationId": "makeThing",
                "requestBody": {"content": {"application/json": {}}},
                "responses": {"200": {"description": "ok"}}
            }},
            "/api/v1/things/{id}": {"get": {
                "operationId": "getThing",
                "parameters": [{"name": "id", "in": "path", "required": true, "schema": {"type": "string"}}],
                "responses": {"200": {"description": "ok", "headers": {"ETag": {}},
                    "content": {"application/json": {}}}}
            }}
        },
        "components": {}
    });
    spec["paths"]["/api/v1/things"]["post"]["requestBody"]["content"]["application/json"]["schema"] =
        request;
    spec["paths"]["/api/v1/things/{id}"]["get"]["responses"]["200"]["content"]["application/json"]
        ["schema"] = answer;
    spec["components"]["schemas"] = schemas;
    spec
}

fn req(schema: Value) -> Value {
    api(schema, json!({}), json!({}))
}

fn ans(schema: Value) -> Value {
    api(json!({}), schema, json!({}))
}

fn object(properties: Value, required: &[&str]) -> Value {
    let mut schema = json!({"type": "object", "required": required});
    schema["properties"] = properties;
    schema
}

fn found(frozen: &Value, current: &Value) -> String {
    evaluate(frozen, current)
        .into_iter()
        .map(|finding| finding.message)
        .collect::<Vec<_>>()
        .join("\n")
}

fn broken_by(frozen: &Value, current: &Value, needle: &str) {
    let text = found(frozen, current);
    assert!(text.contains(needle), "wanted `{needle}` in:\n{text}");
}

fn passes(frozen: &Value, current: &Value) {
    let text = found(frozen, current);
    assert!(text.is_empty(), "no finding wanted:\n{text}");
}

/// The schema of the request body of `POST /api/v1/things`.
const BODY: &str = "/paths/~1api~1v1~1things/post/requestBody/content/application~1json/schema";

/// The parameters of `GET /api/v1/things/{id}`.
const PARAMETERS: &str = "/paths/~1api~1v1~1things~1{id}/get/parameters";

// --- the same description, additions, structure ---

#[test]
fn good_example_the_same_description_and_an_addition_pass() {
    let old = api(
        object(json!({"id": {"type": "string"}}), &[]),
        object(json!({"id": {"type": "string"}}), &["id"]),
        json!({"Code": {"type": "string", "enum": ["a.b"]}}),
    );
    passes(&old, &old);
    let mut now = old.clone();
    now["components"]["schemas"]["Code"]["enum"] = json!(["a.b", "c.d"]);
    now["components"]["schemas"]["More"] = json!({"type": "object"});
    now["paths"]["/api/v1/more"] =
        json!({"get": {"operationId": "getMore", "responses": {"200": {"description": "ok"}}}});
    now["paths"]["/api/v1/things/{id}"]["get"]["responses"]["404"] = json!({"description": "none"});
    passes(&old, &now);
}

#[test]
fn bad_example_a_route_an_operation_a_parameter_an_answer_a_header_and_a_schema_are_gone() {
    let old = api(json!({}), json!({}), json!({"Thing": {"type": "object"}}));
    let mut now = old.clone();
    now["paths"] = json!({});
    broken_by(&old, &now, "the path is gone");
    let mut now = old.clone();
    now["paths"]["/api/v1/things/{id}"] = json!({});
    broken_by(&old, &now, "GET /api/v1/things/{id}: the operation is gone");
    let mut now = old.clone();
    now["paths"]["/api/v1/things/{id}"]["get"]["operationId"] = json!("readThing");
    broken_by(&old, &now, "the operationId changed");
    let mut now = old.clone();
    now["paths"]["/api/v1/things/{id}"]["get"]["parameters"] = json!([]);
    broken_by(&old, &now, "parameter path id: the parameter is gone");
    let mut now = old.clone();
    now["paths"]["/api/v1/things/{id}"]["get"]["responses"] = json!({});
    broken_by(&old, &now, "answer 200: the answer is gone");
    let mut now = old.clone();
    now["paths"]["/api/v1/things/{id}"]["get"]["responses"]["200"]["headers"] = json!({});
    broken_by(&old, &now, "the header ETag is gone");
    let mut now = old.clone();
    now["paths"]["/api/v1/things/{id}"]["get"]["responses"]["200"]["content"] = json!({});
    broken_by(&old, &now, "application/json: the media type is gone");
    let mut now = old.clone();
    now["components"]["schemas"] = json!({});
    broken_by(&old, &now, "schema Thing: the schema is gone");
}

#[test]
fn bad_example_a_schema_no_operation_reaches_loses_a_member() {
    let old = api(
        json!({}),
        json!({}),
        json!({"Lone": object(json!({"id": {"type": "string"}}), &[])}),
    );
    let mut now = old.clone();
    now["components"]["schemas"]["Lone"]["properties"] = json!({});
    broken_by(&old, &now, "schema Lone > id: the member is gone");
}

#[test]
fn good_example_a_parameter_that_is_new_and_optional_with_a_limit() {
    let old = api(json!({}), json!({}), json!({}));
    let mut now = old.clone();
    now.pointer_mut(PARAMETERS)
        .unwrap()
        .as_array_mut()
        .unwrap()
        .push(
            json!({"name": "limit", "in": "query", "schema": {"type": "integer", "maximum": 50}}),
        );
    passes(&old, &now);
}

#[test]
fn bad_example_a_parameter_that_is_new_and_required_or_becomes_required() {
    let old = api(json!({}), json!({}), json!({}));
    let mut now = old.clone();
    now.pointer_mut(PARAMETERS)
        .unwrap()
        .as_array_mut()
        .unwrap()
        .push(json!({"name": "q", "in": "query", "required": true, "schema": {"type": "string"}}));
    broken_by(&old, &now, "parameter query q: a new parameter is required");
    let mut was = old.clone();
    *was.pointer_mut(PARAMETERS).unwrap() =
        json!([{"name": "q", "in": "query", "schema": {"type": "string", "enum": ["a", "b"]}}]);
    let mut now = was.clone();
    now.pointer_mut(PARAMETERS).unwrap()[0]["required"] = json!(true);
    broken_by(&was, &now, "the parameter became required");
    let mut now = was.clone();
    now.pointer_mut(PARAMETERS).unwrap()[0]["schema"]["enum"] = json!(["a"]);
    broken_by(&was, &now, "the value \"b\" is gone");
}

#[test]
fn bad_example_a_request_body_that_goes_or_becomes_required() {
    let old = req(json!({"type": "object"}));
    let mut now = old.clone();
    now["paths"]["/api/v1/things"]["post"]["requestBody"]["required"] = json!(true);
    broken_by(&old, &now, "the request body became required");
    let mut now = old.clone();
    now["paths"]["/api/v1/things"]["post"]
        .as_object_mut()
        .unwrap()
        .remove("requestBody");
    broken_by(&old, &now, "the request body is gone");
}

// --- the request side ---

#[test]
fn bad_example_a_request_member_becomes_required_or_a_required_one_is_new() {
    let old = req(object(json!({"a": {"type": "string"}}), &[]));
    let now = req(object(json!({"a": {"type": "string"}}), &["a"]));
    broken_by(
        &old,
        &now,
        "requestBody application/json > a: the member became required",
    );
    let now = req(object(
        json!({"a": {"type": "string"}, "b": {"type": "string"}}),
        &["b"],
    ));
    broken_by(&old, &now, "> b: a new member is required");
}

fn many_change(move_requires: &[&str]) -> Value {
    api(
        json!({"$ref": "#/components/schemas/ManyChangeActionBody"}),
        json!({}),
        json!({
            "ManyChangeActionBody": {"oneOf": [
                {"$ref": "#/components/schemas/MoveAction"},
                {"$ref": "#/components/schemas/CloseAction"}
            ]},
            "MoveAction": object(json!({
                "kind": {"type": "string", "enum": ["move"]},
                "to": {"type": "string"},
                "reason": {"type": "string"}
            }), move_requires),
            "CloseAction": object(json!({"kind": {"type": "string", "enum": ["close"]}}), &["kind"])
        }),
    )
}

#[test]
fn bad_example_a_required_member_in_an_existing_variant() {
    let old = many_change(&["kind", "to"]);
    let now = many_change(&["kind", "to", "reason"]);
    broken_by(
        &old,
        &now,
        "variant(kind=move) > reason: the member became required",
    );
}

#[test]
fn good_example_a_new_variant_and_a_new_optional_member_in_a_variant() {
    let old = many_change(&["kind", "to"]);
    let mut now = old.clone();
    now["components"]["schemas"]["ManyChangeActionBody"]["oneOf"]
        .as_array_mut()
        .unwrap()
        .push(json!({"$ref": "#/components/schemas/DropAction"}));
    now["components"]["schemas"]["DropAction"] = object(
        json!({"kind": {"type": "string", "enum": ["drop"]}, "why": {"type": "string"}}),
        &["kind", "why"],
    );
    now["components"]["schemas"]["MoveAction"]["properties"]["note"] =
        json!({"type": "string", "maxLength": 80});
    passes(&old, &now);
}

fn value_body(integer_format: &str, string: Value, with_integer: bool) -> Value {
    let mut variants = vec![string, json!({"type": "boolean"})];
    if with_integer {
        variants.push(json!({"type": "integer", "format": integer_format}));
    }
    api(
        json!({"$ref": "#/components/schemas/ValueBody"}),
        json!({}),
        json!({"ValueBody": {"oneOf": variants}}),
    )
}

#[test]
fn bad_example_a_variant_loses_its_integer_changes_its_format_or_gets_a_rule() {
    let text = || json!({"type": "string"});
    let old = value_body("int64", text(), true);
    broken_by(
        &old,
        &value_body("int64", text(), false),
        "variant(integer): the variant is gone",
    );
    broken_by(
        &old,
        &value_body("int32", text(), true),
        "variant(integer): the format is now",
    );
    broken_by(
        &old,
        &value_body("int64", json!({"type": "string", "enum": ["a"]}), true),
        "variant(string): the request now takes a list of values only",
    );
    broken_by(
        &old,
        &value_body("int64", json!({"type": "string", "pattern": "^a"}), true),
        "variant(string): the pattern",
    );
}

#[test]
fn bad_example_a_binary_upload_becomes_base64() {
    let old = req(json!({"type": "string", "format": "binary"}));
    let now = req(json!({"type": "string", "format": "byte"}));
    broken_by(&old, &now, "the format is now");
}

#[test]
fn bad_example_a_free_request_text_gets_a_rule() {
    let old = req(object(json!({"a": {"type": "string"}}), &[]));
    for (key, value, needle) in [
        ("default", json!("x"), "the default changed"),
        ("pattern", json!("^x"), "the pattern"),
        ("enum", json!(["x"]), "takes a list of values only"),
        ("maxLength", json!(10), "maxLength 10 is new or lower"),
        ("minLength", json!(1), "minLength 1 is new or higher"),
        ("format", json!("uuid"), "the format is now"),
    ] {
        let mut now = old.clone();
        now.pointer_mut(BODY).unwrap()["properties"]["a"][key] = value;
        broken_by(&old, &now, needle);
    }
}

#[test]
fn bad_example_a_request_limit_is_new_or_tighter() {
    let old = req(object(
        json!({
            "n": {"type": "integer", "maximum": 100, "minimum": 1},
            "l": {"type": "array", "items": {"type": "string"}}
        }),
        &[],
    ));
    for (member, key, value, needle) in [
        ("n", "maximum", json!(50), "maximum 50"),
        ("n", "minimum", json!(2), "minimum 2"),
        ("n", "multipleOf", json!(5), "multipleOf"),
        ("l", "maxItems", json!(5), "maxItems 5"),
        ("l", "minItems", json!(1), "minItems 1"),
        ("l", "uniqueItems", json!(true), "uniqueItems"),
    ] {
        let mut now = old.clone();
        now.pointer_mut(BODY).unwrap()["properties"][member][key] = value;
        broken_by(&old, &now, needle);
    }
}

#[test]
fn good_example_a_request_loosens() {
    let old = req(object(
        json!({
            "n": {"type": "integer", "maximum": 100, "enum": [1, 2]},
            "s": {"type": "string", "pattern": "^a"},
            "r": {"type": "string"}
        }),
        &["r"],
    ));
    let new = req(object(
        json!({
            "n": {"type": ["integer", "null"], "maximum": 200, "enum": [1, 2, 3]},
            "s": {"type": "string"},
            "r": {"type": "string"}
        }),
        &[],
    ));
    passes(&old, &new);
}

#[test]
fn bad_example_an_integer_enum_value_of_a_request_goes() {
    let old = req(json!({"type": "integer", "enum": [1, 2, 3]}));
    let now = req(json!({"type": "integer", "enum": [1, 3]}));
    broken_by(&old, &now, "the value 2 is gone");
}

#[test]
fn bad_example_a_request_is_narrowed_by_type_null_or_a_closed_object() {
    let old = req(object(json!({"a": {"type": ["string", "null"]}}), &[]));
    let now = req(object(json!({"a": {"type": "string"}}), &[]));
    broken_by(&old, &now, "null is no longer accepted");
    let old = req(object(json!({"a": {"type": ["string", "integer"]}}), &[]));
    let now = req(object(json!({"a": {"type": "string"}}), &[]));
    broken_by(&old, &now, "the type changed");
    let old = req(object(json!({}), &[]));
    let mut closed = old.clone();
    closed.pointer_mut(BODY).unwrap()["additionalProperties"] = json!(false);
    broken_by(&old, &closed, "additionalProperties: false");
}

#[test]
fn bad_example_a_value_schema_on_an_open_map() {
    let old = req(object(
        json!({"fields": {"type": "object", "additionalProperties": true}}),
        &[],
    ));
    let mut now = req(object(
        json!({"fields": {"type": "object", "additionalProperties": {"$ref": "#/components/schemas/Value"}}}),
        &[],
    ));
    now["components"]["schemas"]["Value"] = json!({"type": "string"});
    broken_by(
        &old,
        &now,
        "fields{}: the values of an open map get a schema",
    );
}

#[test]
fn good_example_a_required_member_in_a_new_optional_object() {
    let old = req(object(json!({"a": {"type": "string"}}), &[]));
    let now = req(object(
        json!({
            "a": {"type": "string"},
            "extra": object(json!({"x": {"type": "string", "maxLength": 3}}), &["x"])
        }),
        &[],
    ));
    passes(&old, &now);
}

// --- the answer side ---

#[test]
fn bad_example_an_answer_loses_a_member_or_its_always_there() {
    let two = json!({"a": {"type": "string"}, "b": {"type": "string"}});
    let old = ans(object(two.clone(), &["a", "b"]));
    let now = ans(object(json!({"a": {"type": "string"}}), &["a"]));
    broken_by(&old, &now, "> b: the member is gone");
    let now = ans(object(two, &["a"]));
    broken_by(&old, &now, "> b: the member is no longer always there");
}

#[test]
fn bad_example_an_answer_gets_null_a_wider_type_or_another_format() {
    let old = ans(object(json!({"a": {"type": "string"}}), &["a"]));
    let now = ans(object(json!({"a": {"type": ["string", "null"]}}), &["a"]));
    broken_by(&old, &now, "null is now a value of the answer");
    let now = ans(object(
        json!({"a": {"type": ["string", "integer"]}}),
        &["a"],
    ));
    broken_by(&old, &now, "the type changed");
    let was = ans(object(
        json!({"a": {"type": "string", "format": "date-time"}}),
        &["a"],
    ));
    let now = ans(object(
        json!({"a": {"type": "string", "format": "date"}}),
        &["a"],
    ));
    broken_by(&was, &now, "the format changed");
}

fn person_ref(display_name_in_user: bool) -> Value {
    let kind = |name: &str| json!({"type": "string", "enum": [name]});
    let user = if display_name_in_user {
        object(
            json!({"kind": kind("user"), "display_name": {"type": "string"}}),
            &["kind", "display_name"],
        )
    } else {
        object(json!({"kind": kind("user")}), &["kind"])
    };
    let mut spec = ans(json!({"$ref": "#/components/schemas/PersonRef"}));
    spec["components"]["schemas"] = json!({
        "PersonRef": {"oneOf": [
            {"$ref": "#/components/schemas/UserRef"}, {"$ref": "#/components/schemas/GroupRef"}
        ]},
        "UserRef": user,
        "GroupRef": object(
            json!({"kind": kind("group"), "display_name": {"type": "string"}}),
            &["kind", "display_name"]
        )
    });
    spec
}

#[test]
fn bad_example_one_variant_of_an_answer_loses_a_member_another_keeps() {
    broken_by(
        &person_ref(true),
        &person_ref(false),
        "variant(kind=user) > display_name: the member is gone",
    );
}

#[test]
fn bad_example_an_answer_variant_goes() {
    let old = person_ref(true);
    let mut now = old.clone();
    now["components"]["schemas"]["PersonRef"]["oneOf"] =
        json!([{"$ref": "#/components/schemas/UserRef"}]);
    broken_by(&old, &now, "variant(kind=group): the variant is gone");
}

fn draft_target(both: bool) -> Value {
    let mut spec = ans(json!({"$ref": "#/components/schemas/DraftTargetBody"}));
    let mut variants = vec![json!({"$ref": "#/components/schemas/IssueTarget"})];
    if both {
        variants.push(json!({"$ref": "#/components/schemas/ProjectTarget"}));
    }
    spec["components"]["schemas"] = json!({
        "DraftTargetBody": {"oneOf": variants},
        "IssueTarget": object(json!({"issue": {"type": "string"}}), &["issue"]),
        "ProjectTarget": object(json!({"project": {"type": "string"}}), &["project"])
    });
    spec
}

#[test]
fn bad_example_an_answer_value_goes_and_a_ref_variant_goes() {
    let old = ans(object(
        json!({"state": {"type": "string", "enum": ["open", "done"]}}),
        &["state"],
    ));
    let now = ans(object(
        json!({"state": {"type": "string", "enum": ["open"]}}),
        &["state"],
    ));
    broken_by(&old, &now, "the value \"done\" is gone");
    broken_by(
        &draft_target(true),
        &draft_target(false),
        "variant(ref ProjectTarget): the variant is gone",
    );
}

#[test]
fn good_example_an_answer_grows() {
    let old = ans(object(
        json!({"a": {"type": "string", "enum": ["x"]}, "u": {"type": ["string", "null"]}}),
        &["a"],
    ));
    let now = ans(object(
        json!({
            "a": {"type": "string", "enum": ["x", "y"]},
            "u": {"type": "string"},
            "b": {"type": "string"}
        }),
        &["a", "u"],
    ));
    passes(&old, &now);
    passes(&draft_target(false), &draft_target(true));
}

#[test]
fn bad_example_a_schema_used_by_a_request_and_an_answer_is_judged_by_both() {
    let both = |required: &[&str]| {
        api(
            json!({"$ref": "#/components/schemas/Both"}),
            json!({"$ref": "#/components/schemas/Both"}),
            json!({"Both": object(json!({"a": {"type": "string"}}), required)}),
        )
    };
    // The request side finds that it became required.
    broken_by(&both(&[]), &both(&["a"]), "the member became required");
    // The answer side finds that it is no longer always there.
    broken_by(
        &both(&["a"]),
        &both(&[]),
        "the member is no longer always there",
    );
}

// --- the cut ---

#[test]
fn a_long_list_of_findings_is_cut() {
    let members = |count: usize| {
        let mut map = serde_json::Map::new();
        for index in 0..count {
            map.insert(format!("m{index}"), json!({"type": "string"}));
        }
        Value::Object(map)
    };
    let old = ans(object(members(60), &[]));
    let now = ans(object(members(0), &[]));
    let all = evaluate(&old, &now);
    assert_eq!(all.len(), MOST_FINDINGS + 1);
    assert!(all.last().unwrap().message.contains("And 20 more"));
}

// --- null judged for each use, merged allOf parts, names of variants, schemas no operation reaches ---

fn nullable(reference: &str) -> Value {
    json!({"oneOf": [{"$ref": reference}, {"type": "null"}]})
}

#[test]
fn bad_example_null_is_judged_for_each_use_of_the_same_ref() {
    let x = "#/components/schemas/X";
    let with = |members: Value| {
        let mut spec = ans(object(members, &[]));
        spec["components"]["schemas"] = json!({"X": {"type": "string"}});
        spec
    };
    // Two answer members use X; the second becomes nullable after the first was compared.
    let old = with(json!({"a": {"$ref": x}, "b": {"$ref": x}}));
    let now = with(json!({"a": {"$ref": x}, "b": nullable(x)}));
    broken_by(&old, &now, "> b: null is now a value of the answer");
    // Two request members take null; the first stops taking it.
    let mut old = req(object(json!({"a": nullable(x), "b": nullable(x)}), &[]));
    old["components"]["schemas"] = json!({"X": {"type": "string"}});
    let mut now = req(object(json!({"a": {"$ref": x}, "b": nullable(x)}), &[]));
    now["components"]["schemas"] = json!({"X": {"type": "string"}});
    broken_by(&old, &now, "> a: null is no longer accepted");
    passes(&old, &old);
}

fn tried(button: bool, to_required: bool) -> Value {
    let mut spec = ans(json!({"$ref": "#/components/schemas/TriedChangeBody"}));
    let mut properties = json!({"to": {"type": "string"}});
    let mut required = vec![];
    if to_required {
        required.push("to");
    }
    if button {
        properties["button"] = json!({"type": "string"});
        required.push("button");
    }
    spec["components"]["schemas"] = json!({
        "TriedVerdictBody": {"oneOf": [
            object(json!({"verdict": {"type": "string", "enum": ["allowed"]}}), &["verdict"]),
            object(json!({"verdict": {"type": "string", "enum": ["refused"]}}), &["verdict"])
        ]},
        "TriedChangeBody": {"allOf": [
            {"$ref": "#/components/schemas/TriedVerdictBody"},
            object(properties, &required)
        ]}
    });
    spec
}

#[test]
fn bad_example_what_an_all_of_merged_into_a_node_with_alternatives_is_compared() {
    let old = tried(true, true);
    broken_by(&old, &tried(false, true), "> button: the member is gone");
    broken_by(
        &old,
        &tried(true, false),
        "> to: the member is no longer always there",
    );
    passes(&old, &old);
    // A new required member of the merged part is new input for a request.
    let mut was = req(json!({"$ref": "#/components/schemas/TriedChangeBody"}));
    was["components"]["schemas"] = tried(true, true)["components"]["schemas"].clone();
    let mut now = was.clone();
    now["components"]["schemas"]["TriedChangeBody"]["allOf"][1]["properties"]["extra"] =
        json!({"type": "string"});
    now["components"]["schemas"]["TriedChangeBody"]["allOf"][1]["required"] =
        json!(["to", "button", "extra"]);
    broken_by(&was, &now, "> extra: a new member is required");
}

fn person_ref_by_shown(extra_first: bool, with_role: bool) -> Value {
    let variant = |shown: &str| {
        object(
            json!({"shown": {"type": "string", "enum": [shown]}, "id": {"type": "string"}}),
            &["shown", "id"],
        )
    };
    let mut variants = vec![variant("name")];
    if with_role {
        variants.push(variant("role"));
    }
    if extra_first {
        variants.insert(0, variant("someone"));
    }
    ans(json!({"oneOf": variants}))
}

#[test]
fn good_example_a_variant_named_by_its_shown_value_does_not_shift_when_one_is_inserted() {
    passes(
        &person_ref_by_shown(false, true),
        &person_ref_by_shown(true, true),
    );
}

#[test]
fn bad_example_a_variant_named_by_its_shown_value_is_found_gone() {
    broken_by(
        &person_ref_by_shown(false, true),
        &person_ref_by_shown(false, false),
        "variant(shown=role): the variant is gone",
    );
    // The verdict of TriedVerdictBody names its variants the same way.
    let old = tried(true, true);
    let mut now = old.clone();
    now["components"]["schemas"]["TriedVerdictBody"]["oneOf"] =
        json!([old["components"]["schemas"]["TriedVerdictBody"]["oneOf"][0].clone()]);
    broken_by(&old, &now, "variant(verdict=refused): the variant is gone");
}

#[test]
fn bad_example_a_schema_no_operation_reaches_keeps_its_values() {
    let old = api(
        json!({}),
        json!({}),
        json!({"Field": {"type": "string", "enum": ["a", "b"]}}),
    );
    let mut now = old.clone();
    now["components"]["schemas"]["Field"]["enum"] = json!(["a"]);
    broken_by(&old, &now, "schema Field: the value \"b\" is gone");
    let mut grown = old.clone();
    grown["components"]["schemas"]["Field"]["enum"] = json!(["a", "b", "c"]);
    passes(&old, &grown);
}

#[test]
fn bad_example_a_plain_object_becomes_a_union_or_the_other_way_round() {
    let plain = || {
        object(
            json!({"to": {"type": "string"}, "button": {"type": "string"}}),
            &["to", "button"],
        )
    };
    let union = |with_button: bool| {
        let mut node = object(json!({"to": {"type": "string"}}), &["to"]);
        if with_button {
            node["properties"]["button"] = json!({"type": "string"});
            node["required"] = json!(["to", "button"]);
        }
        node["oneOf"] = json!([
            object(
                json!({"verdict": {"type": "string", "enum": ["allowed"]}}),
                &["verdict"]
            ),
            object(
                json!({"verdict": {"type": "string", "enum": ["refused"]}}),
                &["verdict"]
            )
        ]);
        node
    };
    // A plain object becomes a union, and `button` vanishes in the same change.
    broken_by(
        &ans(plain()),
        &ans(union(false)),
        "> button: the member is gone",
    );
    // A union becomes a plain object, and `button` vanishes in the same change.
    broken_by(
        &ans(union(true)),
        &ans(object(json!({"to": {"type": "string"}}), &["to"])),
        "> button: the member is gone",
    );
    // For a request a plain object that turns into a union is a narrowing in itself.
    broken_by(&req(plain()), &req(union(true)), "one of several shapes");
}
