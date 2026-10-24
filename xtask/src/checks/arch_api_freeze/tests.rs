//! The tests of ARCH-API-FREEZE: what the comparison finds.

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
    now["paths"]["/api/v1/other"] =
        json!({"get": {"operationId": "getOther", "responses": {"200": {"description": "ok"}}}});
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
        post["responses"]["200"]["content"] = json!({"text/plain": {"schema": {"type": "string"}}});
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
fn bad_example_new_input_a_request_must_have() {
    let found = |change: &dyn Fn(&mut Value)| {
        let mut now = typed();
        change(&mut now);
        messages(&typed(), &now).join("\n")
    };
    // A new required member of a request schema that was there.
    let text = found(&|now| {
        now["components"]["schemas"]["NewThing"]["properties"]["colour"] =
            json!({"type": "string"});
        now["components"]["schemas"]["NewThing"]["required"] = json!(["name", "colour"]);
    });
    assert!(text.contains("required-member NewThing.colour"), "{text}");
    // A new required parameter of a route that was there.
    let text = found(&|now| {
        now["paths"]["/api/v1/things"]["post"]["parameters"]
            .as_array_mut()
            .unwrap()
            .push(json!({"name": "X-Org", "in": "header", "required": true, "schema": {"type": "string"}}));
    });
    assert!(
        text.contains("required-parameter POST /api/v1/things header X-Org"),
        "{text}"
    );
    // A request body that is required now, on a route that had none.
    let mut frozen = typed();
    frozen["paths"]["/api/v1/things"]["post"]
        .as_object_mut()
        .unwrap()
        .remove("requestBody");
    let mut now = typed();
    now["paths"]["/api/v1/things"]["post"]["requestBody"]["required"] = json!(true);
    let text = messages(&frozen, &now).join("\n");
    assert!(text.contains("required-body POST /api/v1/things"), "{text}");
}

#[test]
fn good_example_a_required_member_of_a_new_schema_or_a_new_variant_or_a_new_route() {
    let mut now = typed();
    now["components"]["schemas"]["Fresh"] =
        json!({"type": "object", "required": ["a"], "properties": {"a": {"type": "string"}}});
    now["paths"]["/api/v1/fresh"] = json!({"post": {
        "operationId": "createFresh",
        "parameters": [{"name": "must", "in": "query", "required": true, "schema": {"type": "string"}}],
        "requestBody": {"required": true, "content": {"application/json": {"schema": {"$ref": "#/components/schemas/Fresh"}}}},
        "responses": {"200": {"description": "ok"}}
    }});
    assert!(messages(&typed(), &now).is_empty());
    // A new variant of a union a request names may require its own members.
    let mut frozen = typed();
    frozen["components"]["schemas"]["NewThing"]["properties"]["how"] = json!({"oneOf": [{"type": "object", "required": ["kind"], "properties": {"kind": {"type": "string"}}}]});
    let mut now = frozen.clone();
    now["components"]["schemas"]["NewThing"]["properties"]["how"]["oneOf"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type": "object", "required": ["kind", "extra"], "properties": {"kind": {"type": "string"}, "extra": {"type": "string"}}}));
    assert!(messages(&frozen, &now).is_empty());
}

#[test]
fn bad_example_a_request_that_is_tighter_than_it_was() {
    let found = |change: &dyn Fn(&mut Value)| {
        let mut now = typed();
        change(&mut now);
        messages(&typed(), &now).join("\n")
    };
    // No limit before, a limit now.
    let text = found(&|now| {
        now["components"]["schemas"]["NewThing"]["properties"]["note"]["maxLength"] = json!(500);
    });
    assert!(
        text.contains("a new limit of 500 on what was free"),
        "{text}"
    );
    // A rule on a free text: a word list, a pattern, a format.
    let text = found(&|now| {
        now["components"]["schemas"]["NewThing"]["properties"]["note"]["enum"] = json!(["a", "b"]);
    });
    assert!(text.contains("free-text NewThing.note"), "{text}");
    let text = found(&|now| {
        now["components"]["schemas"]["NewThing"]["properties"]["note"]["pattern"] = json!("^a");
    });
    assert!(text.contains("free-text NewThing.note"), "{text}");
    let text = found(&|now| {
        now["components"]["schemas"]["NewThing"]["properties"]["note"]["format"] = json!("uuid");
    });
    assert!(text.contains("free-text NewThing.note"), "{text}");
    // An object that was open and is closed.
    let text = found(&|now| {
        now["components"]["schemas"]["NewThing"]["additionalProperties"] = json!(false);
    });
    assert!(text.contains("open-object NewThing"), "{text}");
    // A pattern and a default that change.
    let mut frozen = typed();
    frozen["components"]["schemas"]["NewThing"]["properties"]["note"]["pattern"] = json!("^a");
    frozen["components"]["schemas"]["NewThing"]["properties"]["note"]["default"] = json!("x");
    let mut now = frozen.clone();
    now["components"]["schemas"]["NewThing"]["properties"]["note"]["pattern"] = json!("^b");
    now["components"]["schemas"]["NewThing"]["properties"]["note"]["default"] = json!("y");
    let text = messages(&frozen, &now).join("\n");
    assert!(text.contains("pattern NewThing.note ^a"), "{text}");
    assert!(text.contains("default NewThing.note"), "{text}");
}

#[test]
fn bad_example_a_value_of_an_inline_parameter_that_goes_and_a_format_that_changes() {
    let mut frozen = typed();
    frozen["paths"]["/api/v1/things"]["post"]["parameters"][0]["schema"] =
        json!({"type": "string", "enum": ["person", "group", "role"]});
    let mut now = frozen.clone();
    now["paths"]["/api/v1/things"]["post"]["parameters"][0]["schema"]["enum"] =
        json!(["person", "group"]);
    let text = messages(&frozen, &now).join("\n");
    assert!(
        text.contains("parameter-value POST /api/v1/things query limit role"),
        "{text}"
    );
    // A new value there is an addition.
    now["paths"]["/api/v1/things"]["post"]["parameters"][0]["schema"]["enum"] =
        json!(["person", "group", "role", "team"]);
    assert!(messages(&frozen, &now).is_empty());
    // The format of an answer member is part of its type.
    let mut with_format = typed();
    with_format["components"]["schemas"]["Thing"]["properties"]["id"]["format"] = json!("uuid");
    let text = messages(&with_format, &typed()).join("\n");
    assert!(text.contains("type Thing.id format:uuid"), "{text}");
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
