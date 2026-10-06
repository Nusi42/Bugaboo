//! The OpenAPI document of the API (ADR-0008).
//!
//! The document is built from the handlers' `#[utoipa::path]` descriptions (`crate::api`), so
//! it always matches the routes. `GET /api/v1/openapi.json` serves it; `bugaboo openapi` prints
//! it; `cargo xtask gen api` writes it to `web/packages/api-client/openapi.json` and generates
//! the TypeScript types from it. The document is also the input of ARCH-LEAN (list routes), and
//! its error part of ARCH-ERRORS.

use std::sync::OnceLock;

use axum::http::header::{ETAG, RETRY_AFTER};
use bb_core::{ErrorCode, Problem, error_code::PROBLEM_CONTENT_TYPE};
use utoipa::openapi::extensions::Extensions;
use utoipa::openapi::path::{Operation, PathItem};
use utoipa::openapi::response::{ResponseBuilder, ResponsesBuilder};
use utoipa::openapi::security::{ApiKey, ApiKeyValue, Http, HttpAuthScheme, SecurityScheme};
use utoipa::openapi::{ContentBuilder, HeaderBuilder, ObjectBuilder, Ref, RefOr, Type};
use utoipa::{Modify, OpenApi};

use crate::api::{AuthHandle, ProbeHandle};
use crate::cookies::{CSRF_HEADER, Cookie};

/// The mark of a field that holds a secret, set with `extensions` on the field of the answer type
/// (`crate::etag` reads it from the description).
pub const SECRET_MARK: &str = "x-secret";
/// The mark of a route whose answer holds personal data and is never stored, set with
/// `extensions` on the route.
pub const NEVER_STORED_MARK: &str = "x-never-stored";

/// The mark of a route that says what it needs: a key of the permission catalogue, or `public`,
/// `signed-in` or `own-account` (`crate::permissions`, M1-F32).
pub const PERMISSION_MARK: &str = "x-permission";

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
    components(schemas(Problem, ErrorCode)),
    tags(
        (name = "server", description = "The server itself."),
        (name = "auth", description = "Signing in, sessions, second step, passkeys, invitations and setup."),
        (name = "admin", description = "What only administrators see: the state of the server.")
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
        for item in openapi.paths.paths.values_mut() {
            for operation in operations(item) {
                add_standard_answers(operation);
                add_permission(operation);
            }
            if let Some(read) = item.get.as_mut() {
                add_read_answers(read);
            }
        }
    }
}

/// What the route needs, as `x-permission` (M1-F32): the declaration of `crate::permissions` for
/// the operation ID. A route that is not declared gets no mark, and the test of that file fails.
fn add_permission(operation: &mut Operation) {
    let Some(declared) = operation
        .operation_id
        .as_deref()
        .and_then(crate::permissions::declared)
    else {
        return;
    };
    let mark = Extensions::builder()
        .add(PERMISSION_MARK, serde_json::json!(declared.key()))
        .build();
    operation
        .extensions
        .get_or_insert_with(Extensions::default)
        .merge(mark);
}

fn operations(item: &mut PathItem) -> impl Iterator<Item = &mut Operation> {
    [
        &mut item.get,
        &mut item.put,
        &mut item.post,
        &mut item.delete,
        &mut item.options,
        &mut item.head,
        &mut item.patch,
        &mut item.trace,
    ]
    .into_iter()
    .filter_map(Option::as_mut)
}

/// The `ETag` header of an answer and the `304` answer to a read that already has it (PERF-03,
/// `crate::etag`); ARCH-LEAN requires both on every `GET`.
fn add_read_answers(operation: &mut Operation) {
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
fn add_standard_answers(operation: &mut Operation) {
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
