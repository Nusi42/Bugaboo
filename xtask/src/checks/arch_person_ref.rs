//! ARCH-PERSON-REF: people are shown only through `PersonRef`, resolved for the viewer (ADR-0022,
//! ADR-0025 decision 11, PRIV-12).
//!
//! A name or an e-mail address in an answer that is not a `PersonRef` would reach a viewer whose
//! level for that kind of people information is *Role only* or *Hidden*. Two readings of the same
//! rule (three readings of it):
//!
//! - **The code:** a struct that derives `Serialize` in `bb-api` or `bb-app` and has a field
//!   called like a person's name or address (`display_name`, `email`, …). The name in the code (a raw `r#` identifier included) and the name `serde` gives
//!   it (`#[serde(rename = …)]`) both count.
//! - **The reads:** any read of `.display_name` or `.email` of a value in the code of `bb-api` and
//!   `bb-app` (without comments, texts and tests), and any destructuring that names the field
//!   (`let Person { display_name, .. } = p;`, in a parameter, in a `match` arm). A function that
//!   has such reads is listed with its file, its name, the number and a reason in the exemptions
//!   (`[[read]]`); one more read, or a read in another function, fails. Whatever a body calls the
//!   field, the name comes from the record and is read from it.
//! - **The API description** (`web/packages/api-client/openapi.json`): a property called like that
//!   in a schema that an answer holds, however deep (what a person types in a request is theirs).
//!
//! The few places where an address or a name is right are listed in
//! `tools/checks/person-ref-exemptions.toml`, each with a reason: the person's own account (a
//! person may always see their own name and address) and the address an invitation is for (it
//! belongs to nobody yet). An exemption that matches nothing fails, so the file cannot go stale.

use std::collections::BTreeSet;

use serde::Deserialize;
use serde_json::Value;

use self::reads::{ReadExemption, read_findings};
use super::api_spec;
use super::exemptions::{self, Used};
use super::{Check, Context, Finding};
use crate::generate::api::SPEC_PATH;
use crate::rust_source::{strip, without_tests};
use crate::source::{Kind, Source, contains_word, word_positions};

/// Where the exemptions are written down, each with its reason.
const EXEMPTIONS_PATH: &str = "tools/checks/person-ref-exemptions.toml";
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

mod reads;

pub(crate) struct ArchPersonRef;

impl Check for ArchPersonRef {
    fn id(&self) -> &'static str {
        "ARCH-PERSON-REF"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let sources = context.sources(self.id(), Kind::Rust)?;
        let spec = api_spec::load(context.repo)?;
        let exemptions: Exemptions = exemptions::load(context.repo, EXEMPTIONS_PATH)?;
        let mut findings = evaluate(&sources, &spec, &exemptions.exempt);
        findings.extend(read_findings(&sources, &exemptions.read));
        Ok(findings)
    }
}

/// The file of exemptions.
#[derive(Deserialize)]
struct Exemptions {
    #[serde(default)]
    exempt: Vec<Exemption>,
    #[serde(default)]
    read: Vec<ReadExemption>,
}

/// The reads of a person's name or address in the code that shapes answers and runs use cases: a
/// field `.display_name` or `.email` of any value. The code is read without comments, texts and tests.
/// What `serde` and the field names of a body hide (a body that calls the field `name`) is seen here,
/// because the name the person has comes from the record and is read from it.
const READ_FIELDS: [&str; 2] = ["display_name", "email"];

/// The folders whose code is read for those reads.
const READ_FOLDERS: [&str; 2] = ["crates/bb-api/src/", "crates/bb-app/src/"];

/// One field that may hold a name or an address: the type (the struct and the schema have one
/// name), the field, and why.
#[derive(Deserialize)]
struct Exemption {
    r#type: String,
    field: String,
    reason: String,
}

impl exemptions::Exemption for Exemption {
    fn name(&self) -> String {
        format!("{}.{}", self.r#type, self.field)
    }

    fn reason(&self) -> &str {
        &self.reason
    }

    fn unused(&self) -> String {
        "matches nothing in the code or in the API description. Remove it.".to_owned()
    }
}

/// A struct that derives `Serialize` with a field of the list, found in the code.
struct Found {
    place: String,
    r#type: String,
    field: String,
}

/// The structs and enums of a file that derive `Serialize`, with their person-like fields. Read
/// from the code with comments and texts taken out, by its braces, so a struct on one line and
/// the variants of an enum with named fields are read like any other.
fn serialized_fields(path: &str, text: &str) -> Vec<Found> {
    let without = without_tests(text);
    let original: Vec<char> = without.chars().collect();
    let stripped = strip(&without);
    let code: Vec<char> = stripped.chars().collect();
    let mut found = Vec::new();
    for keyword in ["struct", "enum"] {
        for byte in word_positions(&stripped, keyword) {
            // The code below counts characters, not bytes.
            let at = stripped[..byte].chars().count();
            let Some((name, open)) = item_head(&code, at + keyword.len()) else {
                continue;
            };
            if !derives_serialize(&code, at) {
                continue;
            }
            let Some(close) = matching(&code, open) else {
                continue;
            };
            let body = &code[open + 1..close];
            let mut fields = Vec::new();
            if keyword == "struct" {
                named_fields(body, &original, open + 1, &mut fields);
            } else {
                // The variants: `Name`, `Name(..)` or `Name { fields }`.
                for (start, variant) in split_top_level(body, open + 1) {
                    let Some(brace) = variant.iter().position(|c| *c == '{') else {
                        continue;
                    };
                    if variant[..brace].contains(&'(') {
                        continue;
                    }
                    let inner_end = variant
                        .iter()
                        .rposition(|c| *c == '}')
                        .unwrap_or(variant.len());
                    if brace < inner_end {
                        named_fields(
                            &variant[brace + 1..inner_end],
                            &original,
                            start + brace + 1,
                            &mut fields,
                        );
                    }
                }
            }
            for (field, position) in fields {
                if PERSON_FIELDS.contains(&field.as_str()) {
                    found.push(Found {
                        place: format!("{path}:{}", line_of(&code, position)),
                        r#type: name.clone(),
                        field,
                    });
                }
            }
        }
    }
    found
}

/// The name of the item after its keyword, and the place of the brace that opens its body; `None`
/// for a tuple struct, a unit struct or something that is not an item.
fn item_head(code: &[char], from: usize) -> Option<(String, usize)> {
    let mut at = from;
    while code.get(at).is_some_and(|c| c.is_whitespace()) {
        at += 1;
    }
    let name: String = code[at..]
        .iter()
        .take_while(|c| c.is_alphanumeric() || **c == '_')
        .collect();
    if name.is_empty() {
        return None;
    }
    // Generics and a where clause come before the brace; a `(` or `;` first is another kind.
    let mut depth = 0_i32;
    for (offset, c) in code[at + name.len()..].iter().enumerate() {
        match c {
            '<' => depth += 1,
            '>' => depth -= 1,
            '{' if depth <= 0 => {
                let open = at + name.len() + offset;
                return Some((name, open));
            }
            '(' | ';' if depth <= 0 => return None,
            _ => {}
        }
    }
    None
}

/// Whether the attributes above the item that starts at `keyword_at` derive `Serialize`.
fn derives_serialize(code: &[char], keyword_at: usize) -> bool {
    let mut end = keyword_at;
    loop {
        // Back over white space and a visibility (`pub`, `pub(crate)`).
        while end > 0 && code[end - 1].is_whitespace() {
            end -= 1;
        }
        if end > 0
            && code[end - 1] == ')'
            && let Some(open) = code[..end].iter().rposition(|c| *c == '(')
        {
            let head: String = code[..open].iter().collect();
            if head.trim_end().ends_with("pub") {
                end = head.trim_end().chars().count() - 3;
                continue;
            }
        }
        if end >= 3 && code[end - 3..end] == ['p', 'u', 'b'] {
            end -= 3;
            continue;
        }
        if end > 0 && code[end - 1] == ']' {
            // An attribute: `#[...]`, possibly with brackets inside.
            let mut depth = 0_i32;
            let mut start = None;
            for index in (0..end).rev() {
                match code[index] {
                    ']' => depth += 1,
                    '[' => {
                        depth -= 1;
                        if depth == 0 {
                            start = Some(index);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let Some(start) = start else {
                return false;
            };
            let attribute: String = code[start..end].iter().collect();
            if attribute.contains("derive(") && contains_word(&attribute, "Serialize") {
                return true;
            }
            // Over the `#` before the bracket.
            end = start.saturating_sub(1);
            continue;
        }
        return false;
    }
}

/// The place of the bracket that closes the one at `open`.
fn matching(code: &[char], open: usize) -> Option<usize> {
    let mut depth = 0_i32;
    for (offset, c) in code[open..].iter().enumerate() {
        match c {
            '{' | '(' | '[' => depth += 1,
            '}' | ')' | ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + offset);
                }
            }
            _ => {}
        }
    }
    None
}

/// The parts of a body between the commas of its top level, each with the place it starts at.
fn split_top_level(body: &[char], offset: usize) -> Vec<(usize, &[char])> {
    let mut parts = Vec::new();
    let mut depth = 0_i32;
    let mut start = 0;
    for (index, c) in body.iter().enumerate() {
        match c {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' | '>' => depth -= 1,
            ',' if depth == 0 => {
                parts.push((offset + start, &body[start..index]));
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push((offset + start, &body[start..]));
    parts
}

/// The names of the fields in a body of `name: Type` parts, with the place of each name. A field
/// is known by its name in the code (without the `r#` of a raw identifier) and by the name `serde`
/// gives it in the answer (`#[serde(rename = "…")]`, read from the original text, as the stripped
/// code has no texts): both are listed, and either may be a person's name or address.
fn named_fields(
    body: &[char],
    original: &[char],
    offset: usize,
    fields: &mut Vec<(String, usize)>,
) {
    for (start, part) in split_top_level(body, offset) {
        let text: String = part.iter().collect();
        let Some((before, _)) = text.split_once(':') else {
            continue;
        };
        // Attributes (`#[serde(...)]`) and the visibility come before the name.
        let name = before
            .rsplit(|c: char| c.is_whitespace() || c == ']' || c == ')')
            .next()
            .unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        let at = text.find(name).map_or(start, |inside| start + inside);
        let written: String = original
            .get(start..start + before.chars().count())
            .unwrap_or_default()
            .iter()
            .collect();
        if let Some(renamed) = serde_rename(&written) {
            fields.push((renamed, at));
        }
        fields.push((name.strip_prefix("r#").unwrap_or(name).to_owned(), at));
    }
}

/// The name `serde` gives the field in the answer, from the attributes written above it:
/// `rename = "x"`, or `rename(serialize = "x", …)`.
fn serde_rename(attributes: &str) -> Option<String> {
    let quoted = |text: &str| {
        let rest = &text[text.find('"')? + 1..];
        Some(rest[..rest.find('"')?].to_owned())
    };
    attributes.match_indices("rename").find_map(|(at, word)| {
        let rest = &attributes[at + word.len()..];
        if rest.starts_with(|c: char| c.is_alphanumeric() || c == '_') {
            // `rename_all` and friends.
            return None;
        }
        let rest = rest.trim_start();
        if let Some(value) = rest.strip_prefix('=') {
            quoted(value)
        } else if let Some(list) = rest.strip_prefix('(') {
            let list = &list[..list.find(')').unwrap_or(list.len())];
            quoted(&list[list.find("serialize")? + "serialize".len()..])
        } else {
            None
        }
    })
}

/// The line (from 1) of the place in the code.
fn line_of(code: &[char], position: usize) -> usize {
    code[..position.min(code.len())]
        .iter()
        .filter(|c| **c == '\n')
        .count()
        + 1
}

/// The names of the schemas that an answer holds: those that the responses of the routes name, and
/// those they name in turn.
fn answer_schemas(spec: &Value) -> BTreeSet<String> {
    let mut reached = BTreeSet::new();
    let mut open: Vec<&Value> = api_spec::operations(spec)
        .into_iter()
        .flatten()
        .flat_map(|operation| api_spec::answer_schemas(spec, operation.body))
        .collect();
    while let Some(schema) = open.pop() {
        if let Some(name) = schema
            .get("$ref")
            .and_then(Value::as_str)
            .and_then(|reference| reference.strip_prefix("#/components/schemas/"))
            && reached.insert(name.to_owned())
        {
            open.push(api_spec::resolve(spec, schema));
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
    let mut used = Used::default();
    let exemption_of = |r#type: &str, field: &str| {
        exempt
            .iter()
            .position(|entry| entry.r#type == r#type && entry.field == field)
    };
    for (path, text) in sources {
        for found in serialized_fields(path, text) {
            if let Some(index) = exemption_of(&found.r#type, &found.field) {
                used.mark(index);
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
                if let Some(index) = exemption_of(schema, field) {
                    used.mark(index);
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
    findings.extend(exemptions::judge(EXEMPTIONS_PATH, exempt, &used));
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
            "#[derive(Serialize, ToSchema)]\npub(crate) struct AssignedPersonBody {\n    display_name: String,\n    pub email: String,\n}\n",
        );
        let spec = json!({
            "paths": {"/a": {"get": {"responses": {"200": {"content": {"application/json": {"schema": {
                "properties": {"inner": {"$ref": "#/components/schemas/AssignedPersonBody"}}
            }}}}}}}},
            "components": {"schemas": {
                "AssignedPersonBody": {"properties": {"display_name": {}, "email": {}}},
            }},
        });
        let findings = evaluate(&sources, &spec, &[]);
        // Two fields in the code, two properties in the description.
        assert_eq!(findings.len(), 4);
    }

    #[test]
    fn bad_example_a_struct_on_one_line_and_the_variants_of_an_enum() {
        let sources = api_source(
            "#[derive(Serialize)] pub struct Short { pub display_name: String }\n\n#[derive(Serialize, ToSchema)]\n#[serde(tag = \"kind\")]\npub enum Who {\n    Named { id: String, email: String },\n    Other(String),\n    Nobody,\n}\n\n#[derive(Clone, serde::Serialize)]\npub(crate) struct Multi<'a> {\n    #[serde(skip)]\n    note: &'a str,\n    #[schema(example = \"x\")]\n    pub(crate) username: String,\n}\n",
        );
        let findings = evaluate(&sources, &json!({}), &[]);
        let messages: Vec<String> = findings.iter().map(|f| f.message.clone()).collect();
        assert_eq!(findings.len(), 3, "{messages:?}");
        assert!(messages.iter().any(|m| m.contains("`Short.display_name`")));
        assert!(messages.iter().any(|m| m.contains("`Who.email`")));
        assert!(messages.iter().any(|m| m.contains("`Multi.username`")));
        let places: Vec<Option<&str>> = findings.iter().map(|f| f.location.as_deref()).collect();
        assert!(places.contains(&Some("crates/bb-api/src/api/x.rs:1")));
        assert!(places.contains(&Some("crates/bb-api/src/api/x.rs:6")));
    }

    #[test]
    fn good_example_types_that_are_not_serialized_or_have_no_named_person_field() {
        let sources = api_source(
            "#[derive(Deserialize)] pub struct In { email: String }\n#[derive(Serialize)] pub struct Own(pub String);\n#[derive(Serialize)] pub enum Who { Name(String), Person { person: PersonRef } }\n// #[derive(Serialize)] struct Comment { email: String }\nconst TEXT: &str = \"#[derive(Serialize)] struct T { email: String }\";\n#[derive(Serialize)] pub struct Calm { note: String }\n#[cfg(test)]\nmod tests {\n    #[derive(Serialize)] struct T { email: String }\n}\n",
        );
        assert!(evaluate(&sources, &json!({}), &[]).is_empty());
    }

    #[test]
    fn bad_example_a_raw_identifier_and_a_name_given_by_serde() {
        let sources = api_source(
            "#[derive(Serialize)]\npub struct Raw {\n    pub r#email: String,\n}\n\n#[derive(Serialize)]\npub struct Renamed {\n    #[serde(rename = \"email\")]\n    contact: String,\n    #[serde(rename(serialize = \"display_name\", deserialize = \"n\"))]\n    shown: String,\n    #[serde(rename = \"label\")]\n    display_name: String,\n}\n",
        );
        let findings = evaluate(&sources, &json!({}), &[]);
        let messages: Vec<String> = findings.iter().map(|f| f.message.clone()).collect();
        assert_eq!(findings.len(), 4, "{messages:?}");
        assert!(messages.iter().any(|m| m.contains("`Raw.email`")));
        assert!(messages.iter().any(|m| m.contains("`Renamed.email`")));
        assert!(
            messages
                .iter()
                .any(|m| m.contains("`Renamed.display_name`"))
        );
    }

    #[test]
    fn an_exemption_that_matches_nothing_or_has_no_reason_fails() {
        let findings = evaluate(&[], &json!({}), &[exemption("Gone", "email")]);
        assert_eq!(findings.len(), 1);
        let mut empty = exemption("Gone", "email");
        empty.reason = " ".to_owned();
        assert_eq!(evaluate(&[], &json!({}), &[empty]).len(), 2);
    }

    fn allowed(file: &str, function: &str, reads: usize) -> ReadExemption {
        ReadExemption {
            file: file.to_owned(),
            function: function.to_owned(),
            reads,
            reason: "the own account of the person".to_owned(),
        }
    }

    #[test]
    fn bad_example_a_read_of_a_name_or_an_address_that_nobody_allowed() {
        let sources = api_source(
            "fn body(person: &Person) -> Body { Body { name: person.display_name.clone(), to: person.email.clone() } }",
        );
        let found = read_findings(&sources, &[]);
        assert_eq!(found.len(), 1);
        assert!(found[0].message.contains("2 times"), "{}", found[0].message);
        // More reads than the entry allows, an entry for a file without a read, and no reason.
        let found = read_findings(
            &sources,
            &[allowed("crates/bb-api/src/api/x.rs", "body", 1)],
        );
        assert!(
            found[0].message.contains("allows 1"),
            "{}",
            found[0].message
        );
        let found = read_findings(
            &sources,
            &[allowed("crates/bb-api/src/api/y.rs", "body", 1)],
        );
        assert!(
            found
                .iter()
                .any(|each| each.message.contains("matches no read"))
        );
        // The same number of reads allowed for another function is no allowance.
        let found = read_findings(
            &sources,
            &[allowed("crates/bb-api/src/api/x.rs", "other", 2)],
        );
        assert_eq!(found.len(), 2);
        let bare = ReadExemption {
            reason: String::new(),
            ..allowed("crates/bb-api/src/api/x.rs", "body", 2)
        };
        assert!(!read_findings(&sources, &[bare]).is_empty());
    }

    #[test]
    fn good_example_the_reads_an_entry_allows_and_what_is_no_read() {
        let sources = api_source(
            "fn body(p: &Person) -> Body { Body { name: p.display_name.clone() } }
// p.email is said in a comment
fn other(p: &Person) { let _ = p.display_name_of_group; let _ = \"p.email\"; }
#[cfg(test)]
mod tests { fn t(p: &Person) { let _ = p.email; } }",
        );
        assert!(
            read_findings(
                &sources,
                &[allowed("crates/bb-api/src/api/x.rs", "body", 1)]
            )
            .is_empty()
        );
        // A file outside bb-api and bb-app, and a file of tests, are not read.
        let elsewhere = vec![
            (
                "crates/bb-store/src/x.rs".to_owned(),
                "fn f(p: P) { p.email }".to_owned(),
            ),
            (
                "crates/bb-app/src/tests/x.rs".to_owned(),
                "fn f(p: P) { p.email }".to_owned(),
            ),
        ];
        assert!(read_findings(&elsewhere, &[]).is_empty());
    }
}
