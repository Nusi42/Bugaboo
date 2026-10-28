//! ARCH-VERSIONED: every change takes the version it read, and every save of long text also takes
//! the token of its edit lock (ADR-0019, amendment M3).
//!
//! Read from the generated API description, so a new change route is found without anyone
//! registering it. A *change operation* is any operation but a `GET`, `HEAD` or `OPTIONS`. Its
//! request body must have the property `version`, listed as required so a request without it is
//! refused, or the route must be named in
//! `tools/checks/version-exemptions.toml` with a reason: signing in has no stored version, making a
//! new record has nothing to be stale, an idempotent removal loses nothing. A route marked
//! `x-long-text` (from the table of `bb-api`) saves long text: its body must have `version` and
//! `lock_token`, and it can never be exempt. An exemption that names a route that is not there,
//! or one that has a version now, fails, so the file shrinks as the routes are fixed and cannot go
//! stale. A change of an existing record (a body with `version`) that holds a long-text property
//! (`LONG_TEXT_PROPERTIES`) and is not marked `x-long-text` fails: long text is never written without
//! its lock. The static check sees only properties of the body that are named in the description;
//! the own long-text fields of an issue, which travel inside the open `fields` map, are refused at
//! run time (M3 WP-05). A route that takes a version must also document the structured answer to a stale
//! version (`ConflictProblem`) as its `409`, so the generated client types it.
//!
//! One narrow category waives only `required`: a route named under `[[optional]]` in the same file
//! took its `version` after the API was frozen (ADR-0008), so the member must stay optional for the
//! clients that do not send it (the last change wins for them). It must still have `version` among
//! its members and document `ConflictProblem` as its `409`; a route whose `version` is required
//! already, or has none, or is not a change, fails, so the list cannot go stale.

use bb_api_words::marks::LONG_TEXT;
use bb_api_words::versions::{CONFLICT_SCHEMA, LOCK_TOKEN, LONG_TEXT_PROPERTIES, VERSION};
use serde::Deserialize;
use serde_json::Value;

use super::api_spec::{self, Operation, resolve};
use super::exemptions;
use super::{Check, Context, Finding};

/// Where the exemptions are written down, each with its reason.
const EXEMPTIONS_PATH: &str = "tools/checks/version-exemptions.toml";
/// The methods that read and so need no version.
const READS: [&str; 3] = ["GET", "HEAD", "OPTIONS"];
/// The status of the answer to a stale version.
const CONFLICT_STATUS: &str = "409";

pub(crate) struct ArchVersioned;

impl Check for ArchVersioned {
    fn id(&self) -> &'static str {
        "ARCH-VERSIONED"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let spec = api_spec::load(context.repo)?;
        let file: Exemptions = exemptions::load(context.repo, EXEMPTIONS_PATH)?;
        Ok(evaluate(&spec, &file.exempt, &file.optional))
    }
}

/// The file of exemptions.
#[derive(Deserialize)]
struct Exemptions {
    #[serde(default)]
    exempt: Vec<Exemption>,
    #[serde(default)]
    optional: Vec<Exemption>,
}

/// Routes (`POST /api/v1/auth/sign-in`) that take no version, and why.
#[derive(Deserialize)]
struct Exemption {
    routes: Vec<String>,
    reason: String,
}

impl exemptions::Exemption for Exemption {
    fn name(&self) -> String {
        self.routes.first().cloned().unwrap_or_default()
    }

    fn reason(&self) -> &str {
        &self.reason
    }
}

/// The properties of the JSON body of an operation, with a `$ref` and `allOf` followed.
fn body_properties<'a>(spec: &'a Value, operation: &'a Value) -> Vec<&'a str> {
    let schema = operation
        .pointer("/requestBody/content/application~1json/schema")
        .map(|schema| resolve(spec, schema));
    let mut names = Vec::new();
    if let Some(schema) = schema {
        collect(spec, schema, &mut names);
    }
    names
}

fn collect<'a>(spec: &'a Value, schema: &'a Value, names: &mut Vec<&'a str>) {
    collect_in(spec, schema, "properties", names);
}

/// The names the body must have (`required`), so a request without a version is refused by the
/// description and not only by the code.
fn body_required<'a>(spec: &'a Value, operation: &'a Value) -> Vec<&'a str> {
    let schema = operation
        .pointer("/requestBody/content/application~1json/schema")
        .map(|schema| resolve(spec, schema));
    let mut names = Vec::new();
    if let Some(schema) = schema {
        collect_in(spec, schema, "required", &mut names);
    }
    names
}

/// The keys of `properties`, or the strings of `required`, of a schema and its `allOf` members.
fn collect_in<'a>(spec: &'a Value, schema: &'a Value, key: &str, names: &mut Vec<&'a str>) {
    match schema.get(key) {
        Some(Value::Object(properties)) => names.extend(properties.keys().map(String::as_str)),
        Some(Value::Array(listed)) => names.extend(listed.iter().filter_map(Value::as_str)),
        _ => {}
    }
    for member in schema
        .get("allOf")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        collect_in(spec, resolve(spec, member), key, names);
    }
}

/// Whether the answer to a stale version is documented as the structured conflict.
fn documents_conflict(spec: &Value, operation: &Value) -> bool {
    operation
        .pointer(&format!("/responses/{CONFLICT_STATUS}"))
        .map(|answer| resolve(spec, answer))
        .and_then(|answer| answer.get("content"))
        .and_then(Value::as_object)
        .is_some_and(|content| {
            content.values().any(|media| {
                media
                    .pointer("/schema/$ref")
                    .and_then(Value::as_str)
                    .is_some_and(|reference| reference.ends_with(CONFLICT_SCHEMA))
            })
        })
}

fn is_change(operation: &Operation<'_>) -> bool {
    !READS.contains(&operation.method.as_str())
}

fn route_findings(
    spec: &Value,
    found: &Operation<'_>,
    exempt: Option<&Exemption>,
    optional: bool,
) -> Vec<Finding> {
    let place = found.place();
    let properties = body_properties(spec, found.body);
    let required = body_required(spec, found.body);
    let versioned = properties.contains(&VERSION);
    let long_text = found.body.get(LONG_TEXT) == Some(&Value::Bool(true));
    let mut findings = Vec::new();
    if long_text {
        for (needed, why) in [
            (VERSION, "the version the person started from"),
            (
                LOCK_TOKEN,
                "the token of the edit lock, so a save without the lock is refused",
            ),
        ] {
            if !properties.contains(&needed) {
                findings.push(Finding::at(
                    &place,
                    format!("A route that saves long text must take `{needed}` in its body: {why} (ADR-0019)."),
                ));
            }
        }
        if exempt.is_some() {
            findings.push(Finding::at(
                &place,
                format!("A route that saves long text is never exempt from `{VERSION}` and `{LOCK_TOKEN}`: remove it from {EXEMPTIONS_PATH}."),
            ));
        }
    } else if versioned {
        // A change of an existing record never writes long text but under its lock (M3-F50).
        for name in LONG_TEXT_PROPERTIES {
            if properties.contains(name) {
                findings.push(Finding::at(
                    &place,
                    format!("A change of a record must not take the long text `{name}` in its body: long text is saved by a route marked `x-long-text`, under the edit lock (ADR-0019)."),
                ));
            }
        }
    } else if exempt.is_none() {
        findings.push(Finding::at(
            &place,
            format!("A change must take the `{VERSION}` it read in its body, so nobody overwrites another change unseen (ADR-0019). If it cannot have one (signing in, making a new record), name the route with its reason in {EXEMPTIONS_PATH}."),
        ));
    }
    // A property that is optional in the description is a request that may leave the version out.
    for needed in [VERSION, LOCK_TOKEN] {
        // The version is required, unless a route that took it after the freeze is listed as
        // optional (a save of long text is never listed); the lock token only for long text.
        let must = if needed == VERSION {
            long_text || !optional
        } else {
            long_text
        };
        if properties.contains(&needed) && must && !required.contains(&needed) {
            findings.push(Finding::at(
                &place,
                format!("`{needed}` must be required in the body (not optional), so a request without it is refused."),
            ));
        }
    }
    if versioned && !documents_conflict(spec, found.body) {
        findings.push(Finding::at(
            &place,
            format!("A route that takes a `{VERSION}` must document the answer to a stale version: its `{CONFLICT_STATUS}` has the schema `{CONFLICT_SCHEMA}`."),
        ));
    }
    findings
}

fn evaluate(spec: &Value, exempt: &[Exemption], optional: &[Exemption]) -> Vec<Finding> {
    let Some(operations) = api_spec::operations(spec) else {
        return vec![api_spec::no_paths()];
    };
    let mut findings = exemptions::reason_findings(EXEMPTIONS_PATH, exempt);
    findings.extend(exemptions::reason_findings(EXEMPTIONS_PATH, optional));
    let changes: Vec<&Operation<'_>> = operations
        .iter()
        .filter(|operation| is_change(operation))
        .collect();
    for found in &changes {
        let place = found.place();
        let entry = exempt.iter().find(|entry| entry.routes.contains(&place));
        let waived = optional.iter().any(|entry| entry.routes.contains(&place));
        findings.extend(route_findings(spec, found, entry, waived));
    }
    for entry in optional {
        for route in &entry.routes {
            let Some(found) = changes.iter().find(|found| found.place() == *route) else {
                findings.push(Finding::at(
                    EXEMPTIONS_PATH,
                    format!("The optional version `{route}` names a change that the API description does not have. Remove it."),
                ));
                continue;
            };
            if !body_properties(spec, found.body).contains(&VERSION) {
                findings.push(Finding::at(
                    EXEMPTIONS_PATH,
                    format!("`{route}` is listed under `[[optional]]` but takes no `{VERSION}`: the category waives only `required`, so the member must be there."),
                ));
            } else if body_required(spec, found.body).contains(&VERSION) {
                findings.push(Finding::at(
                    EXEMPTIONS_PATH,
                    format!(
                        "`{route}` requires its `{VERSION}` now. Remove it from `[[optional]]`."
                    ),
                ));
            }
        }
    }
    for entry in exempt {
        for route in &entry.routes {
            match changes.iter().find(|found| found.place() == *route) {
                None => findings.push(Finding::at(
                    EXEMPTIONS_PATH,
                    format!("The exemption `{route}` names a change that the API description does not have. Remove it."),
                )),
                Some(found) if body_properties(spec, found.body).contains(&VERSION) => {
                    findings.push(Finding::at(
                        EXEMPTIONS_PATH,
                        format!("`{route}` takes a `{VERSION}` now. Remove its exemption."),
                    ));
                }
                Some(_) => {}
            }
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const LONG: &str = "Because a person cannot have one, and that is final.";

    /// A change operation with a body that has these properties.
    fn change(properties: &[&str], conflict: bool, long_text: bool) -> Value {
        let body: serde_json::Map<String, Value> = properties
            .iter()
            .map(|name| ((*name).to_owned(), json!({"type": "integer"})))
            .collect();
        let required: Vec<&str> = properties.to_vec();
        let mut operation = json!({
            "requestBody": {"content": {"application/json": {"schema": {"type": "object", "properties": body, "required": required}}}},
            "responses": {"200": {"description": "ok"}}
        });
        if conflict {
            operation["responses"]["409"] = json!({"description": "stale", "content": {
                "application/problem+json": {"schema": {"$ref": "#/components/schemas/ConflictProblem"}}
            }});
        }
        if long_text {
            operation[LONG_TEXT] = Value::Bool(true);
        }
        operation
    }

    fn spec(operation: &Value) -> Value {
        json!({"paths": {"/api/v1/things/{id}": {"patch": operation}, "/api/v1/things": {"get": {"responses": {}}}}})
    }

    fn exempt(routes: &[&str], reason: &str) -> Exemption {
        Exemption {
            routes: routes.iter().map(|route| (*route).to_owned()).collect(),
            reason: reason.to_owned(),
        }
    }

    fn messages(spec: &Value, exempt: &[Exemption]) -> Vec<String> {
        messages_with(spec, exempt, &[])
    }

    fn messages_with(spec: &Value, exempt: &[Exemption], optional: &[Exemption]) -> Vec<String> {
        evaluate(spec, exempt, optional)
            .into_iter()
            .map(|finding| finding.message)
            .collect()
    }

    /// A change whose `version` is a member but not required.
    fn optional_version() -> Value {
        let mut optional = change(&["name", "version"], true, false);
        optional["requestBody"]["content"]["application/json"]["schema"]["required"] =
            json!(["name"]);
        optional
    }

    #[test]
    fn good_example_a_version_added_after_the_freeze_may_stay_optional() {
        let named = [exempt(&["PATCH /api/v1/things/{id}"], LONG)];
        assert!(messages_with(&spec(&optional_version()), &[], &named).is_empty());
    }

    #[test]
    fn bad_example_an_optional_version_is_still_a_version_with_its_conflict_answer() {
        let named = [exempt(&["PATCH /api/v1/things/{id}"], LONG)];
        let mut no_answer = optional_version();
        no_answer["responses"] = json!({"200": {"description": "ok"}});
        let found = messages_with(&spec(&no_answer), &[], &named).join(
            "
",
        );
        assert!(found.contains("ConflictProblem"), "{found}");
        let renamed = spec(&change(&["name", "base_version"], true, false));
        let found = messages_with(&renamed, &[], &named).join(
            "
",
        );
        assert!(found.contains("takes no `version`"), "{found}");
    }

    #[test]
    fn bad_example_the_optional_category_waives_nothing_else_and_cannot_go_stale() {
        let named = [exempt(&["PATCH /api/v1/things/{id}"], LONG)];
        // Not listed: the version must be required.
        let found = messages(&spec(&optional_version()), &[]).join(
            "
",
        );
        assert!(found.contains("must be required"), "{found}");
        // Listed, but the version is required now, or the route is gone.
        let found = messages_with(&spec(&change(&["version"], true, false)), &[], &named).join(
            "
",
        );
        assert!(found.contains("requires its `version` now"), "{found}");
        let gone = [exempt(&["POST /api/v1/gone"], LONG)];
        let found = messages_with(&spec(&optional_version()), &[], &gone).join(
            "
",
        );
        assert!(found.contains("does not have"), "{found}");
    }

    #[test]
    fn good_example_a_change_with_its_version_and_the_conflict_answer() {
        let good = spec(&change(&["name", "version"], true, false));
        assert!(messages(&good, &[]).is_empty());
    }

    #[test]
    fn good_example_a_change_that_cannot_have_a_version_is_named_with_its_reason() {
        let sign_in = spec(&change(&["name"], false, false));
        let named = [exempt(&["PATCH /api/v1/things/{id}"], LONG)];
        assert!(messages(&sign_in, &named).is_empty());
    }

    #[test]
    fn good_example_a_save_of_long_text_takes_the_version_and_the_lock_token() {
        let save = spec(&change(&["text", "version", "lock_token"], true, true));
        assert!(messages(&save, &[]).is_empty());
    }

    #[test]
    fn good_example_reads_need_no_version() {
        let reads = json!({"paths": {"/api/v1/things": {"get": {"responses": {}}, "head": {"responses": {}}}}});
        assert!(messages(&reads, &[]).is_empty());
    }

    #[test]
    fn bad_example_a_change_without_a_version() {
        let bad = spec(&change(&["name"], false, false));
        let found = messages(&bad, &[]).join("\n");
        assert!(found.contains("must take the `version`"), "{found}");
    }

    #[test]
    fn bad_example_a_version_that_is_optional_in_the_body() {
        let mut optional = change(&["name", "version"], true, false);
        optional["requestBody"]["content"]["application/json"]["schema"]["required"] =
            json!(["name"]);
        let found = messages(&spec(&optional), &[]).join(
            "
",
        );
        assert!(found.contains("must be required"), "{found}");
    }

    #[test]
    fn bad_example_a_change_of_a_record_that_takes_the_description_without_the_lock() {
        let bad = spec(&change(&["title", "description", "version"], true, false));
        let found = messages(&bad, &[]).join(
            "
",
        );
        assert!(found.contains("long text `description`"), "{found}");
    }

    #[test]
    fn good_example_making_a_record_may_take_the_description() {
        let made = spec(&change(&["title", "description"], false, false));
        let named = [exempt(&["PATCH /api/v1/things/{id}"], LONG)];
        assert!(messages(&made, &named).is_empty());
    }

    #[test]
    fn bad_example_a_change_without_a_body_at_all() {
        let bad = spec(&json!({"responses": {}}));
        assert_eq!(messages(&bad, &[]).len(), 1);
    }

    #[test]
    fn bad_example_a_save_of_long_text_without_the_lock_token() {
        let bad = spec(&change(&["text", "version"], true, true));
        let found = messages(&bad, &[]).join("\n");
        assert!(found.contains("`lock_token`"), "{found}");
        let neither = spec(&change(&["text"], false, true));
        assert_eq!(messages(&neither, &[]).len(), 2);
    }

    #[test]
    fn bad_example_a_save_of_long_text_is_never_exempt() {
        let save = spec(&change(&["text", "version", "lock_token"], true, true));
        let found = messages(&save, &[exempt(&["PATCH /api/v1/things/{id}"], LONG)]).join("\n");
        assert!(found.contains("never exempt"), "{found}");
    }

    #[test]
    fn bad_example_a_version_whose_stale_answer_is_not_described() {
        let bad = spec(&change(&["version"], false, false));
        let found = messages(&bad, &[]).join("\n");
        assert!(found.contains("ConflictProblem"), "{found}");
    }

    #[test]
    fn bad_example_an_exemption_needs_a_reason_and_a_route_that_needs_it() {
        let versioned = spec(&change(&["version"], true, false));
        let found = messages(
            &versioned,
            &[
                exempt(&["PATCH /api/v1/things/{id}"], LONG),
                exempt(&["POST /api/v1/gone"], LONG),
            ],
        )
        .join("\n");
        assert!(found.contains("takes a `version` now"), "{found}");
        assert!(found.contains("does not have"), "{found}");
        let unversioned = spec(&change(&["name"], false, false));
        let found = messages(
            &unversioned,
            &[exempt(&["PATCH /api/v1/things/{id}"], "No.")],
        )
        .join("\n");
        assert!(found.contains("needs a reason"), "{found}");
    }

    #[test]
    fn a_document_without_paths_is_a_finding() {
        assert_eq!(messages(&json!({}), &[]).len(), 1);
    }

    #[test]
    fn arch_versioned_every_change_route_takes_a_version() {
        let repo = crate::repo::Repo::discover().unwrap();
        let spec = api_spec::load(&repo).unwrap();
        let file: Exemptions = exemptions::load(&repo, EXEMPTIONS_PATH).unwrap();
        let found = messages_with(&spec, &file.exempt, &file.optional);
        assert!(found.is_empty(), "{found:?}");
    }
}
