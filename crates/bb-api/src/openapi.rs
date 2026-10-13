//! The OpenAPI document of the API (ADR-0008).
//!
//! The document is built from the handlers' `#[utoipa::path]` descriptions (`crate::api`), so
//! it always matches the routes. `GET /api/v1/openapi.json` serves it; `bugaboo openapi` prints
//! it; `cargo xtask gen api` writes it to `web/packages/api-client/openapi.json` and generates
//! the TypeScript types from it. The document is also the input of ARCH-LEAN (list routes), and
//! its error part of ARCH-ERRORS.

use std::sync::OnceLock;

use axum::http::Method;
use axum::http::header::{ETAG, RETRY_AFTER};
use bb_api_words::marks;
use bb_api_words::versions::{CONFLICT_SCHEMA, VERSION};
use bb_core::fields::Field;
use bb_core::{ErrorCode, Problem, error_code::PROBLEM_CONTENT_TYPE};
use utoipa::openapi::extensions::Extensions;
use utoipa::openapi::path::{Operation as OperationSpec, PathItem};
use utoipa::openapi::response::{ResponseBuilder, ResponsesBuilder};
use utoipa::openapi::schema::{ArrayItems, Components, Object, Schema};
use utoipa::openapi::security::{ApiKey, ApiKeyValue, Http, HttpAuthScheme, SecurityScheme};
use utoipa::openapi::{ContentBuilder, HeaderBuilder, ObjectBuilder, Ref, RefOr, Type};
use utoipa::{Modify, OpenApi};

use crate::api::{AuthHandle, ProbeHandle};
use crate::conflict::ConflictProblem;
use crate::cookies::{CSRF_HEADER, Cookie};
use crate::permissions::{Answer, Declared};

/// The name of the security scheme for the session cookie.
const SESSION_SCHEME: &str = "sessionCookie";
/// The name of the security scheme for the CSRF header.
const CSRF_SCHEME: &str = "csrfToken";
/// The name of the security scheme for the access key of a script or an app.
const ACCESS_KEY_SCHEME: &str = "accessKey";

/// The parts of the document that do not belong to one route.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Bugaboo API",
        description = "The API of Bugaboo. The browser app uses exactly this API. Every error \
            is application/problem+json with a stable code and a message key."
    ),
    components(schemas(Problem, ErrorCode, ConflictProblem, Field)),
    tags(
        (name = "server", description = "The server itself."),
        (name = "auth", description = "Signing in, sessions, second step, passkeys, invitations and setup."),
        (name = "admin", description = "What only administrators see: the state of the server."),
        (name = "settings", description = "Settings in layers: the organization's, a project's and a person's own."),
        (name = "editing", description = "Edit locks, drafts and the check of what is on the screen: nobody loses text, and no change overwrites another unseen."),
        (name = "access", description = "What roles see (field permissions, who sees people) and what a person can do and why (check access).")
    )
)]
struct ApiDoc;

/// The document without routes; `crate::api::routes` adds them.
pub(crate) fn base() -> utoipa::openapi::OpenApi {
    let mut document = ApiDoc::openapi();
    // The licence of the product is not decided here; an empty name would be invalid.
    document.info.license = None;
    document
}

/// The state the routes are described with: nothing is served, so nothing is behind the handles.
#[derive(Clone)]
struct Described {
    auth: AuthHandle,
    probe: ProbeHandle,
}

impl axum::extract::FromRef<Described> for AuthHandle {
    fn from_ref(state: &Described) -> Self {
        state.auth.clone()
    }
}

impl axum::extract::FromRef<Described> for ProbeHandle {
    fn from_ref(state: &Described) -> Self {
        state.probe.clone()
    }
}

/// The finished document.
#[must_use]
pub fn document() -> utoipa::openapi::OpenApi {
    // The limits only decide how a route is wrapped; the description is the same for any numbers.
    let limits = crate::limits::Limits::new(crate::limits::LimitsConfig::default());
    let mut document = crate::api::routes::<Described>(&limits).into_openapi();
    // After the routes are in: the standard answers go to every route.
    StandardParts.modify(&mut document);
    document
}

/// The document as text, in the form that is written to disk (pretty, stable order, one final
/// line break). The extensions of one object are in the order of their names, as utoipa keeps
/// them in a hash map and would write them in another order every run.
#[must_use]
pub fn json() -> &'static str {
    static JSON: OnceLock<String> = OnceLock::new();
    JSON.get_or_init(|| {
        // A document of plain data always serializes; the empty text is what an impossible
        // failure would leave, and the contract test would catch it.
        let mut text = document()
            .to_pretty_json()
            .ok()
            .and_then(|text| steady_text(&text))
            .unwrap_or_default();
        text.push('\n');
        text
    })
}

/// The string properties of answers that hold HTML the server rendered with `bb-markdown`, as the
/// name of the schema and the property: each gets the mark `x-safe-html`, and the generated client
/// turns its value into a `SafeHtml` (M3-F12). A new rendered field is added here and nowhere else;
/// the test below holds the table to the real schemas.
pub(crate) const SAFE_HTML_PROPERTIES: &[(&str, &str)] = &[
    ("IssueDetailBody", "description_html"),
    ("CommentItem", "html"),
    ("CommentTextItem", "html"),
    ("HistoryEntryItem", "html"),
    ("MentionedComment", "html"),
    ("OwnHtmlBody", "html"),
];

/// The schema of a page of `item` and the path to the property inside its items: a page holds its
/// own copy of the item, so it gets the mark too, wherever such a page exists.
fn page_entry(schema: &str, property: &str) -> (String, String) {
    (format!("Page_{schema}"), format!("items/{property}"))
}

/// Every schema and path that gets the mark: the table, and the pages of its items that exist in
/// `components`.
fn marked_paths(components: &Components) -> Vec<(String, String)> {
    let mut paths = Vec::new();
    for (schema, property) in SAFE_HTML_PROPERTIES {
        paths.push(((*schema).to_owned(), (*property).to_owned()));
        let (page, path) = page_entry(schema, property);
        if components.schemas.contains_key(&page) {
            paths.push((page, path));
        }
    }
    paths
}

/// The property at `path` of a schema: names separated by `/`, where a name that is a list goes into
/// the items of the list.
fn property_at<'a>(object: &'a mut Object, path: &str) -> Option<&'a mut Object> {
    let (first, rest) = match path.split_once('/') {
        Some((first, rest)) => (first, Some(rest)),
        None => (path, None),
    };
    let inner = match object.properties.get_mut(first)? {
        RefOr::T(Schema::Object(inner)) => Some(inner),
        RefOr::T(Schema::Array(array)) => match &mut array.items {
            ArrayItems::RefOrSchema(item) => match item.as_mut() {
                RefOr::T(Schema::Object(item)) => Some(item),
                _ => None,
            },
            ArrayItems::False => None,
        },
        _ => None,
    }?;
    match rest {
        None => Some(inner),
        Some(rest) => property_at(inner, rest),
    }
}

/// Puts `x-safe-html` on the properties of [`SAFE_HTML_PROPERTIES`]. Returns the ones that were
/// not found (a test says there are none).
fn mark_safe_html(openapi: &mut utoipa::openapi::OpenApi) -> Vec<(String, String)> {
    let mut missing = Vec::new();
    let components = openapi.components.get_or_insert_with(Default::default);
    for (schema, property) in marked_paths(components) {
        let object = match components.schemas.get_mut(&schema) {
            Some(RefOr::T(Schema::Object(object))) => property_at(object, &property),
            _ => None,
        };
        match object {
            Some(property) => {
                property
                    .extensions
                    .get_or_insert_with(Extensions::default)
                    .merge(
                        Extensions::builder()
                            .add(marks::SAFE_HTML, serde_json::json!(true))
                            .build(),
                    );
            }
            _ => missing.push((schema, property)),
        }
    }
    missing
}

/// What every route has: the answers every request can get, and the security schemes.
struct StandardParts;

impl Modify for StandardParts {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            SESSION_SCHEME,
            SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new(Cookie::Session.name()))),
        );
        let mut bearer = Http::new(HttpAuthScheme::Bearer);
        bearer.bearer_format = Some("bb_ access key".to_owned());
        bearer.description = Some(
            "An access key from My settings, sent as `Authorization: Bearer bb_…`. It can do at most what its owner can do at the moment of the request, and nothing on the owner's own sign-in data."
                .to_owned(),
        );
        components.add_security_scheme(ACCESS_KEY_SCHEME, SecurityScheme::Http(bearer));
        components.add_security_scheme(
            CSRF_SCHEME,
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new(CSRF_HEADER))),
        );
        mark_safe_html(openapi);
        let components = openapi.components.clone().unwrap_or_default();
        for item in openapi.paths.paths.values_mut() {
            for (_, operation) in each_operation(item) {
                add_standard_answers(operation);
                add_marks(operation);
                add_conflict_answer(&components, operation);
            }
            if let Some(read) = item.get.as_mut() {
                add_read_answers(read);
            }
        }
    }
}

/// What the route needs and what kind of answer it gives, as `x-permission`, `x-never-stored` and
/// `x-secret` (M1-F32, PERF-03): the row of `crate::permissions` for the operation ID, the only
/// place that decides. A route that has no row gets no mark, and ARCH-PERMS fails for it.
fn add_marks(operation: &mut OperationSpec) {
    let Some(row) = operation
        .operation_id
        .as_deref()
        .and_then(crate::permissions::declared)
    else {
        return;
    };
    let mut marks =
        Extensions::builder().add(marks::PERMISSION, serde_json::json!(row.needs.key()));
    match row.kind {
        Answer::NeverStored => marks = marks.add(marks::NEVER_STORED, serde_json::json!(true)),
        Answer::Secret => marks = marks.add(marks::SECRET, serde_json::json!(true)),
        Answer::Ordinary => {}
    }
    if crate::permissions::LONG_TEXT_ROUTES.contains(&row.id) {
        marks = marks.add(marks::LONG_TEXT, serde_json::json!(true));
    }
    operation
        .extensions
        .get_or_insert_with(Extensions::default)
        .merge(marks.build());
}

/// Whether the JSON body of the operation has the property `name`.
fn body_has(components: &Components, operation: &OperationSpec, name: &str) -> bool {
    let Some(RefOr::T(Schema::Object(body))) = operation
        .request_body
        .as_ref()
        .and_then(|body| match body {
            RefOr::T(body) => body.content.get("application/json"),
            RefOr::Ref(_) => None,
        })
        .and_then(|content| match content {
            RefOr::T(content) => content.schema.as_ref(),
            RefOr::Ref(_) => None,
        })
        .map(|schema| match schema {
            RefOr::Ref(reference) => reference
                .ref_location
                .rsplit('/')
                .next()
                .and_then(|schema| components.schemas.get(schema))
                .cloned()
                .unwrap_or_else(|| schema.clone()),
            RefOr::T(_) => schema.clone(),
        })
    else {
        return false;
    };
    body.properties.contains_key(name)
}

/// A route that takes a `version` answers a stale one with the structured conflict
/// (`ConflictProblem`, ADR-0019 amendment M3): the description says so for every such route, so the
/// generated client types it for all of them and no route describes it by hand.
fn add_conflict_answer(components: &Components, operation: &mut OperationSpec) {
    if !body_has(components, operation, VERSION) {
        return;
    }
    let code = ErrorCode::VersionChanged;
    let content = ContentBuilder::new()
        .schema(Some(Ref::from_schema_name(CONFLICT_SCHEMA)))
        .build();
    let status = code.status().to_string();
    let answer = match operation.responses.responses.remove(&status) {
        Some(RefOr::T(mut known)) => {
            known
                .content
                .insert(PROBLEM_CONTENT_TYPE.to_owned(), RefOr::T(content));
            known
        }
        _ => ResponseBuilder::new()
            .description(format!("{} ({})", code.message(), code.code()))
            .content(PROBLEM_CONTENT_TYPE, content)
            .build(),
    };
    operation
        .responses
        .responses
        .insert(status, RefOr::T(answer));
}

/// The operations of one address with the name of their method, the one list of methods.
fn each_operation(item: &mut PathItem) -> impl Iterator<Item = (Method, &mut OperationSpec)> {
    [
        (Method::GET, &mut item.get),
        (Method::PUT, &mut item.put),
        (Method::POST, &mut item.post),
        (Method::DELETE, &mut item.delete),
        (Method::OPTIONS, &mut item.options),
        (Method::HEAD, &mut item.head),
        (Method::PATCH, &mut item.patch),
        (Method::TRACE, &mut item.trace),
    ]
    .into_iter()
    .filter_map(|(method, operation)| operation.as_mut().map(|operation| (method, operation)))
}

/// One operation of the API description: a method on an address, with what the table of
/// `crate::permissions` says about its route.
#[derive(Clone, Debug)]
pub struct Operation {
    /// The method (`GET`, `PATCH`, …).
    pub method: Method,
    /// The address as the description writes it, with `{…}` for the parts that vary.
    pub path: String,
    /// The operation ID (`listProjects`).
    pub id: String,
    /// What the route needs.
    pub needs: Declared,
    /// What kind of answer it gives.
    pub kind: Answer,
}

impl Operation {
    /// Whether the operation changes something (any method but `GET`, `HEAD` and `OPTIONS`).
    #[must_use]
    pub fn changes(&self) -> bool {
        crate::api::changes(&self.method)
    }
}

/// Every operation of the API description that has a row in the table, in the order of address
/// and method. This is the one walk over the routes: tests and checks that go through the routes
/// use it, so a method nobody thought of (`PATCH`) is not left out. A route without a row is not
/// in it; ARCH-PERMS fails for that route.
#[must_use]
pub fn operations() -> &'static [Operation] {
    static OPERATIONS: OnceLock<Vec<Operation>> = OnceLock::new();
    OPERATIONS.get_or_init(|| {
        let mut document = document();
        let mut found = Vec::new();
        for (path, item) in &mut document.paths.paths {
            for (method, operation) in each_operation(item) {
                let row = operation
                    .operation_id
                    .as_deref()
                    .and_then(crate::permissions::declared);
                if let Some(row) = row {
                    found.push(Operation {
                        method,
                        path: path.clone(),
                        id: row.id.to_owned(),
                        needs: row.needs,
                        kind: row.kind,
                    });
                }
            }
        }
        found
    })
}

/// The `ETag` header of an answer and the `304` answer to a read that already has it (PERF-03,
/// `crate::etag`); ARCH-LEAN requires both on every `GET`.
fn add_read_answers(operation: &mut OperationSpec) {
    let etag = || {
        HeaderBuilder::new()
            .description(Some(
                "A tag for this answer. Send it back as `If-None-Match` to get `304` when nothing changed.",
            ))
            .schema(Some(ObjectBuilder::new().schema_type(Type::String).build()))
            .build()
    };
    let responses = &mut operation.responses.responses;
    if let Some(RefOr::T(ok)) = responses.get_mut("200") {
        ok.headers
            .insert(ETAG.as_str().to_owned(), RefOr::T(etag()));
    }
    let not_modified = ResponseBuilder::new()
        .description("Nothing changed since the `ETag` that `If-None-Match` named. No body.")
        .header(ETAG.as_str(), etag())
        .build();
    responses
        .entry("304".to_owned())
        .or_insert(RefOr::T(not_modified));
}

/// Any request can be refused for going over a limit, or fail inside the server.
fn add_standard_answers(operation: &mut OperationSpec) {
    let problem = || {
        ContentBuilder::new()
            .schema(Some(Ref::from_schema_name("Problem")))
            .build()
    };
    let answer = |code: ErrorCode| {
        ResponseBuilder::new().description(format!("{} ({})", code.message(), code.code()))
    };
    let mut responses = ResponsesBuilder::new();
    for code in [
        ErrorCode::TooManyRequests,
        ErrorCode::Internal,
        ErrorCode::Unavailable,
    ] {
        let mut response = answer(code).content(PROBLEM_CONTENT_TYPE, problem());
        if code == ErrorCode::TooManyRequests {
            response = response.header(
                RETRY_AFTER.as_str(),
                HeaderBuilder::new()
                    .description(Some("Seconds to wait before trying again."))
                    .schema(Some(
                        ObjectBuilder::new().schema_type(Type::Integer).build(),
                    ))
                    .build(),
            );
        }
        responses = responses.response(code.status().to_string(), response.build());
    }
    for (status, response) in &responses.build().responses {
        operation
            .responses
            .responses
            .entry(status.clone())
            .or_insert_with(|| response.clone());
    }
}

/// A JSON document that keeps the order of its keys as it was written. The extensions of
/// utoipa (`x-…`) sit in a hash map, so two runs of the program write them in two orders;
/// [`Ordered::steady`] puts them in the order of their names and touches nothing else.
enum Ordered {
    Object(Vec<(String, Ordered)>),
    Array(Vec<Ordered>),
    Leaf(serde_json::Value),
}

impl Ordered {
    /// The extensions of every object in the order of their names, in the places where they are.
    fn steady(&mut self) {
        match self {
            Self::Object(entries) => {
                let places: Vec<usize> = entries
                    .iter()
                    .enumerate()
                    .filter(|(_, (key, _))| key.starts_with("x-"))
                    .map(|(place, _)| place)
                    .collect();
                let mut extensions: Vec<(String, Ordered)> = places
                    .iter()
                    .map(|place| {
                        std::mem::replace(
                            &mut entries[*place],
                            (String::new(), Self::Array(Vec::new())),
                        )
                    })
                    .collect();
                extensions.sort_by(|a, b| a.0.cmp(&b.0));
                for (place, entry) in places.into_iter().zip(extensions) {
                    entries[place] = entry;
                }
                for (_, value) in entries {
                    value.steady();
                }
            }
            Self::Array(items) => items.iter_mut().for_each(Self::steady),
            Self::Leaf(_) => {}
        }
    }
}

impl<'de> serde::Deserialize<'de> for Ordered {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Reader;

        impl<'de> serde::de::Visitor<'de> for Reader {
            type Value = Ordered;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a JSON value")
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Ordered, A::Error> {
                let mut entries = Vec::new();
                while let Some(entry) = map.next_entry::<String, Ordered>()? {
                    entries.push(entry);
                }
                Ok(Ordered::Object(entries))
            }

            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<Ordered, A::Error> {
                let mut items = Vec::new();
                while let Some(item) = seq.next_element::<Ordered>()? {
                    items.push(item);
                }
                Ok(Ordered::Array(items))
            }

            fn visit_bool<E>(self, value: bool) -> Result<Ordered, E> {
                Ok(Ordered::Leaf(value.into()))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Ordered, E> {
                Ok(Ordered::Leaf(value.into()))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Ordered, E> {
                Ok(Ordered::Leaf(value.into()))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Ordered, E> {
                Ok(Ordered::Leaf(serde_json::Value::from(value)))
            }

            fn visit_str<E>(self, value: &str) -> Result<Ordered, E> {
                Ok(Ordered::Leaf(value.into()))
            }

            fn visit_unit<E>(self) -> Result<Ordered, E> {
                Ok(Ordered::Leaf(serde_json::Value::Null))
            }
        }

        deserializer.deserialize_any(Reader)
    }
}

impl serde::Serialize for Ordered {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::{SerializeMap as _, SerializeSeq as _};
        match self {
            Self::Object(entries) => {
                let mut map = serializer.serialize_map(Some(entries.len()))?;
                for (key, value) in entries {
                    map.serialize_entry(key, value)?;
                }
                map.end()
            }
            Self::Array(items) => {
                let mut seq = serializer.serialize_seq(Some(items.len()))?;
                for item in items {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }
            Self::Leaf(value) => value.serialize(serializer),
        }
    }
}

/// The text of the document with the order of the extensions fixed.
fn steady_text(text: &str) -> Option<String> {
    let mut document: Ordered = serde_json::from_str(text).ok()?;
    document.steady();
    serde_json::to_string_pretty(&document).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m3_f12_every_server_rendered_property_has_the_mark_and_no_other_has() {
        let value = serde_json::to_value(document()).unwrap();
        let schemas = &value["components"]["schemas"];
        let paths = marked_paths(document().components.as_ref().unwrap());
        for (schema, path) in &paths {
            // A name that is a list goes into the items of the list.
            let mut at = &schemas[schema.as_str()];
            for name in path.split('/') {
                at = &at["properties"][name];
                if at["type"] == "array" {
                    at = &at["items"];
                }
            }
            assert_eq!(
                at[marks::SAFE_HTML],
                serde_json::json!(true),
                "{schema}.{path} lacks the mark"
            );
            // A text that may be missing is a string or null.
            let is_text = at["type"] == "string"
                || at["type"]
                    .as_array()
                    .is_some_and(|kinds| kinds.contains(&serde_json::json!("string")));
            assert!(is_text, "{schema}.{path} is no text");
        }
        let marked = value
            .to_string()
            .matches(&format!("\"{}\":true", marks::SAFE_HTML))
            .count();
        assert_eq!(marked, paths.len());
    }

    #[test]
    fn the_extensions_come_in_the_order_of_their_names_and_nothing_else_moves() {
        let text = r#"{"b":1,"x-permission":"a","a":[{"x-never-stored":true,"x-also":null,"z":2}],"x-never-stored":true}"#;
        let steady = steady_text(text).unwrap();
        let again = steady_text(
            r#"{"b":1,"x-never-stored":true,"a":[{"z":2,"x-also":null,"x-never-stored":true}],"x-permission":"a"}"#,
        )
        .unwrap();
        // The slots of the extensions stay where they were; only the names in them are sorted.
        assert_eq!(
            steady.split_whitespace().collect::<String>(),
            r#"{"b":1,"x-never-stored":true,"a":[{"x-also":null,"x-never-stored":true,"z":2}],"x-permission":"a"}"#
        );
        assert_eq!(
            again.split_whitespace().collect::<String>(),
            r#"{"b":1,"x-never-stored":true,"a":[{"z":2,"x-also":null,"x-never-stored":true}],"x-permission":"a"}"#
        );
    }

    #[test]
    fn the_text_of_the_document_is_the_same_every_time() {
        // Whatever order the hash map wrote them in, the text says them in one order.
        let first = steady_text(&document().to_pretty_json().unwrap()).unwrap();
        for _ in 0..8 {
            let next = steady_text(&document().to_pretty_json().unwrap()).unwrap();
            assert_eq!(first, next);
        }
    }
}
