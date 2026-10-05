//! ARCH-PERSON-REF: people are shown only through `PersonRef`, resolved for the viewer (ADR-0022,
//! ADR-0025 decision 11, PRIV-12).
//!
//! A name or an e-mail address in an answer that is not a `PersonRef` would reach a viewer whose
//! level for that kind of people information is *Role only* or *Hidden*. Two readings of the same
//! rule:
//!
//! - **The code:** a struct that derives `Serialize` in `bb-api` or `bb-app` and has a field
//!   called like a person's name or address (`display_name`, `email`, …).
//! - **The API description** (`web/packages/api-client/openapi.json`): a property called like that
//!   in a schema that an answer holds, however deep (what a person types in a request is theirs).
//!
//! The few places where an address or a name is right are listed in
//! `tools/checks/person-ref-exemptions.toml`, each with a reason: the person's own account (a
//! person may always see their own name and address) and the address an invitation is for (it
//! belongs to nobody yet). An exemption that matches nothing fails, so the file cannot go stale.

use std::collections::BTreeSet;

use anyhow::Context as _;
use serde::Deserialize;
use serde_json::Value;

use super::{Check, Context, Finding};
use crate::generate::api::SPEC_PATH;
use crate::rust_source::{Source, before_tests, read_product_sources};

/// Where the exemptions are written down, each with its reason.
const EXEMPTIONS_PATH: &str = "tools/checks/person-ref-exemptions.toml";
/// The crates whose types go to a browser or a script.
const VIEW_FOLDERS: [&str; 2] = ["crates/bb-api/src/", "crates/bb-app/src/"];
/// Field names that hold a person's name or address.
const PERSON_FIELDS: [&str; 9] = [
    "display_name",
    "email",
    "person_name",
    "user_name",
    "username",
    "author_name",
    "full_name",
    "first_name",
    "last_name",
];

pub(crate) struct ArchPersonRef;

impl Check for ArchPersonRef {
    fn id(&self) -> &'static str {
        "ARCH-PERSON-REF"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let sources = read_product_sources(context.repo)?;
        let text = std::fs::read_to_string(context.repo.path(SPEC_PATH))
            .with_context(|| format!("could not read {SPEC_PATH}; run `cargo xtask gen api`"))?;
        let spec: Value = serde_json::from_str(&text)
            .with_context(|| format!("{SPEC_PATH} is not valid JSON"))?;
        let exemptions = std::fs::read_to_string(context.repo.path(EXEMPTIONS_PATH))
            .with_context(|| format!("could not read {EXEMPTIONS_PATH}"))?;
        let exemptions: Exemptions = toml::from_str(&exemptions)
            .with_context(|| format!("{EXEMPTIONS_PATH} is not valid"))?;
        Ok(evaluate(&sources, &spec, &exemptions.exempt))
    }
}

/// The file of exemptions.
#[derive(Deserialize)]
struct Exemptions {
    #[serde(default)]
    exempt: Vec<Exemption>,
}

/// One field that may hold a name or an address: the type (the struct and the schema have one
/// name), the field, and why.
#[derive(Deserialize)]
struct Exemption {
    r#type: String,
    field: String,
    reason: String,
}

/// A struct that derives `Serialize` with a field of the list, found in the code.
struct Found {
    place: String,
    r#type: String,
    field: String,
}

/// The structs of a file that derive `Serialize`, with their person-like fields.
fn serialized_fields(path: &str, text: &str) -> Vec<Found> {
    let mut found = Vec::new();
    let lines: Vec<&str> = before_tests(text).lines().collect();
    let mut at = 0;
    while at < lines.len() {
        let trimmed = lines[at].trim_start();
        let is_struct = trimmed.starts_with("pub struct ")
            || trimmed.starts_with("pub(crate) struct ")
            || trimmed.starts_with("struct ");
        if is_struct && trimmed.ends_with('{') {
            let derives_serialize = (0..at)
                .rev()
                .take_while(|above| {
                    let line = lines[*above].trim_start();
                    line.starts_with("#[") || line.starts_with("///") || line.starts_with("//")
                })
                .any(|above| {
                    let line = lines[above];
                    line.contains("derive(") && line.contains("Serialize")
                });
            let name = trimmed
                .split_whitespace()
                .skip_while(|word| *word != "struct")
                .nth(1)
                .unwrap_or_default()
                .trim_end_matches('{')
                .split('<')
                .next()
                .unwrap_or_default()
                .to_owned();
            let mut end = at + 1;
            while end < lines.len()
                && !lines[end].starts_with('}')
                && !lines[end].starts_with("    }")
            {
                end += 1;
            }
            if derives_serialize {
                for (offset, field) in lines[at + 1..end.min(lines.len())].iter().enumerate() {
                    let field = field.trim_start();
                    let field = field
                        .strip_prefix("pub(crate) ")
                        .or_else(|| field.strip_prefix("pub "))
                        .unwrap_or(field);
                    if let Some((field_name, _)) = field.split_once(':')
                        && PERSON_FIELDS.contains(&field_name.trim())
                    {
                        found.push(Found {
                            place: format!("{path}:{}", at + offset + 2),
                            r#type: name.clone(),
                            field: field_name.trim().to_owned(),
                        });
                    }
                }
            }
            at = end;
        }
        at += 1;
    }
    found
}

/// The names of the schemas that an answer holds: those that the responses of the routes name, and
/// those they name in turn.
fn answer_schemas(spec: &Value) -> BTreeSet<String> {
    let mut reached = BTreeSet::new();
    let mut open: Vec<&Value> = Vec::new();
    for item in spec
        .get("paths")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(serde_json::Map::values)
    {
        for operation in item
            .as_object()
            .into_iter()
            .flat_map(serde_json::Map::values)
        {
            for response in operation
                .get("responses")
                .and_then(Value::as_object)
                .into_iter()
                .flat_map(serde_json::Map::values)
            {
                if let Some(schema) = response.pointer("/content/application~1json/schema") {
                    open.push(schema);
                }
            }
        }
    }
    while let Some(schema) = open.pop() {
        if let Some(name) = schema
            .get("$ref")
            .and_then(Value::as_str)
            .and_then(|reference| reference.strip_prefix("#/components/schemas/"))
            && reached.insert(name.to_owned())
            && let Some(found) = spec.pointer(&format!("/components/schemas/{name}"))
        {
            open.push(found);
        }
        if let Some(object) = schema.as_object() {
            open.extend(
                object
                    .values()
                    .filter(|value| value.is_object() || value.is_array()),
            );
        }
        if let Some(list) = schema.as_array() {
            open.extend(
                list.iter()
                    .filter(|value| value.is_object() || value.is_array()),
            );
        }
    }
    reached
}

fn evaluate(sources: &[Source], spec: &Value, exempt: &[Exemption]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut used: BTreeSet<(String, String)> = BTreeSet::new();
    let is_exempt = |r#type: &str, field: &str| {
        exempt
            .iter()
            .any(|entry| entry.r#type == r#type && entry.field == field)
    };
    for (path, text) in sources
        .iter()
        .filter(|(path, _)| VIEW_FOLDERS.iter().any(|folder| path.starts_with(folder)))
    {
        for found in serialized_fields(path, text) {
            if is_exempt(&found.r#type, &found.field) {
                used.insert((found.r#type, found.field));
            } else {
                findings.push(Finding::at(
                    found.place,
                    format!(
                        "`{}.{}` shows a person's name or address outside `PersonRef`. Show \
                         people as a `PersonRef`, resolved for the viewer; only the person's own \
                         account may carry a name and an address, and it is listed with a reason \
                         in {EXEMPTIONS_PATH}.",
                        found.r#type, found.field
                    ),
                ));
            }
        }
    }
    let answered = answer_schemas(spec);
    if let Some(schemas) = spec
        .pointer("/components/schemas")
        .and_then(Value::as_object)
    {
        for (schema, body) in schemas
            .iter()
            .filter(|(schema, _)| answered.contains(*schema))
        {
            for field in body
                .get("properties")
                .and_then(Value::as_object)
                .into_iter()
                .flat_map(serde_json::Map::keys)
                .filter(|field| PERSON_FIELDS.contains(&field.as_str()))
            {
                if is_exempt(schema, field) {
                    used.insert((schema.clone(), field.clone()));
                } else {
                    findings.push(Finding::at(
                        SPEC_PATH,
                        format!(
                            "The API answers with `{schema}.{field}`, a person's name or address \
                             outside `PersonRef`. Show people as a `PersonRef`, or list the field \
                             with a reason in {EXEMPTIONS_PATH}."
                        ),
                    ));
                }
            }
        }
    }
    for entry in exempt {
        if entry.reason.trim().is_empty() {
            findings.push(Finding::at(
                EXEMPTIONS_PATH,
                format!(
                    "The exemption of `{}.{}` needs a reason.",
                    entry.r#type, entry.field
                ),
            ));
        }
        if !used.contains(&(entry.r#type.clone(), entry.field.clone())) {
            findings.push(Finding::at(
                EXEMPTIONS_PATH,
                format!(
                    "The exemption of `{}.{}` matches nothing in the code or in the API \
                     description. Remove it.",
                    entry.r#type, entry.field
                ),
            ));
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn exemption(r#type: &str, field: &str) -> Exemption {
        Exemption {
            r#type: r#type.to_owned(),
            field: field.to_owned(),
            reason: "the person's own account".to_owned(),
        }
    }

    fn api_source(text: &str) -> Vec<Source> {
        vec![("crates/bb-api/src/api/x.rs".to_owned(), text.to_owned())]
    }

    #[test]
    fn good_example_people_are_a_person_ref_and_the_own_account_is_listed_with_a_reason() {
        let sources = api_source(
            "#[derive(Serialize, ToSchema)]\npub(crate) struct MemberBody {\n    /// Who.\n    person: PersonRef,\n    role: String,\n}\n\n#[derive(Serialize)]\npub(crate) struct MeBody {\n    email: String,\n}\n\n#[derive(Deserialize)]\npub(crate) struct InviteBody {\n    email: String,\n}\n",
        );
        let spec = json!({
            "paths": {"/m": {"get": {"responses": {"200": {"content": {"application/json": {"schema": {
                "type": "array", "items": {"$ref": "#/components/schemas/MemberBody"}
            }}}}}}}, "/me": {"get": {"responses": {"200": {"content": {"application/json": {"schema": {
                "$ref": "#/components/schemas/MeBody"
            }}}}}}}},
            "components": {"schemas": {
                "MemberBody": {"properties": {"person": {}, "role": {}}},
                "MeBody": {"properties": {"email": {}}},
                "InviteBody": {"properties": {"email": {}}},
            }},
        });
        assert!(evaluate(&sources, &spec, &[exemption("MeBody", "email")]).is_empty());
    }

    #[test]
    fn bad_example_a_name_and_an_address_in_a_view_and_in_the_description() {
        let sources = api_source(
            "#[derive(Serialize, ToSchema)]\npub(crate) struct AssigneeBody {\n    display_name: String,\n    pub email: String,\n}\n",
        );
        let spec = json!({
            "paths": {"/a": {"get": {"responses": {"200": {"content": {"application/json": {"schema": {
                "properties": {"inner": {"$ref": "#/components/schemas/AssigneeBody"}}
            }}}}}}}},
            "components": {"schemas": {
                "AssigneeBody": {"properties": {"display_name": {}, "email": {}}},
            }},
        });
        let findings = evaluate(&sources, &spec, &[]);
        // Two fields in the code, two properties in the description.
        assert_eq!(findings.len(), 4);
    }

    #[test]
    fn an_exemption_that_matches_nothing_or_has_no_reason_fails() {
        let findings = evaluate(&[], &json!({}), &[exemption("Gone", "email")]);
        assert_eq!(findings.len(), 1);
        let mut empty = exemption("Gone", "email");
        empty.reason = " ".to_owned();
        assert_eq!(evaluate(&[], &json!({}), &[empty]).len(), 2);
    }

    #[test]
    fn code_outside_the_view_crates_is_not_read() {
        let sources = vec![(
            "crates/bb-core/src/person.rs".to_owned(),
            "#[derive(Serialize)]\npub struct Person {\n    pub email: String,\n}\n".to_owned(),
        )];
        assert!(evaluate(&sources, &json!({}), &[]).is_empty());
    }
}
