//! What a browser or a cache may do with an API answer: entity tags and `304`, and the cache rules
//! (PERF-03, architecture §7 "Performance").
//!
//! One middleware, [`cache_rules`], decides for every answer below `/api`, so a route never
//! writes a cache header and a route added later cannot forget one:
//!
//! - **A read** (`GET`, `HEAD` answered `200`) gets a weak `ETag` made from a hash of the final
//!   answer body, and `Cache-Control: private, no-cache`: the browser keeps it but asks again
//!   every time, with `If-None-Match`, and a match answers `304` without a body. The hash is
//!   taken after the answer is complete (after masking), so two people with different rights
//!   never get the same tag for different text.
//! - **An answer that hands out a secret** (a new session, an access key, backup codes, an
//!   invitation link: [`SECRET_ANSWERS`]) is `no-store`, has no `ETag`, and is marked
//!   [`SecretAnswer`] so compression leaves it alone (architecture §7 "Browser protections").
//! - **Everything else** (answers to changes, errors) is `no-store` unless the route said
//!   otherwise.
//!
//! The files of the browser app are not API answers: they keep their content hash and
//! `immutable` (`crate::router`).

use axum::body::Body;
use axum::extract::{MatchedPath, Request};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse as _, Response};
use base64ct::{Base64UrlUnpadded, Encoding as _};
use sha2::{Digest as _, Sha256};

use crate::api::is_api;
use crate::errors::ApiError;

/// The answers that hand out a secret: the method and the route as the API description writes
/// it. They are never stored, never get an `ETag` and are never compressed (BREACH). A test
/// keeps the list complete: every route whose answer schema holds a field marked `x-secret` is
/// here, and every entry is a route of the API description.
pub(crate) const SECRET_ANSWERS: [(&str, &str); 12] = [
    // A new session: its secret travels in the cookie.
    ("POST", "/api/v1/auth/setup"),
    ("POST", "/api/v1/auth/sign-in"),
    ("POST", "/api/v1/auth/sign-up"),
    ("POST", "/api/v1/auth/invitation/accept"),
    ("POST", "/api/v1/auth/passkey/sign-in"),
    ("POST", "/api/v1/auth/second-step"),
    ("POST", "/api/v1/auth/second-step/passkey"),
    ("POST", "/api/v1/auth/password"),
    // A secret in the body: an invitation link, the secret of the code app, backup codes.
    ("POST", "/api/v1/auth/invitations"),
    ("POST", "/api/v1/auth/two-step/code-app"),
    ("POST", "/api/v1/auth/two-step/code-app/confirm"),
    ("POST", "/api/v1/auth/two-step/backup-codes"),
];

/// The cache rule of an answer a browser may keep and ask about again.
const REVALIDATE: &str = "private, no-cache";
/// The cache rule of everything that must not be stored.
const NEVER_STORE: &str = "no-store";
/// The biggest answer that is read into memory to make its `ETag`.
const ETAG_BODY_LIMIT: usize = 16 * 1024 * 1024;
/// Keeps an `ETag` apart from any other hash made in the server.
const ETAG_DOMAIN: &[u8] = b"bugaboo.etag.v1\0";
/// How many bytes of the hash the `ETag` holds.
const ETAG_BYTES: usize = 16;

/// Put on the response of an answer that hands out a secret, for the layers outside.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SecretAnswer;

/// Whether the answer of this method and route (as the API description writes it) hands out a
/// secret.
pub(crate) fn is_secret_answer(method: &Method, route: &str) -> bool {
    SECRET_ANSWERS.iter().any(|(secret_method, secret_route)| {
        method.as_str() == *secret_method && route == *secret_route
    })
}

/// The weak `ETag` of an answer body.
fn etag_of(body: &[u8]) -> String {
    let digest = Sha256::new()
        .chain_update(ETAG_DOMAIN)
        .chain_update(body)
        .finalize();
    format!(
        "W/\"{}\"",
        Base64UrlUnpadded::encode_string(&digest[..ETAG_BYTES])
    )
}

/// The part of a tag that is compared: weak comparison ignores the `W/` mark (RFC 9110 §8.8.3.2).
fn opaque(tag: &str) -> &str {
    tag.trim().trim_start_matches("W/")
}

/// Whether `If-None-Match` names the tag, or is `*`.
fn matches(if_none_match: &HeaderMap, etag: &str) -> bool {
    if_none_match
        .get_all(header::IF_NONE_MATCH)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .any(|tag| tag.trim() == "*" || opaque(tag) == opaque(etag))
}

fn is_read(method: &Method) -> bool {
    matches!(*method, Method::GET | Method::HEAD)
}

fn set_cache_control(response: &mut Response, rule: &'static str, overriding: bool) {
    let headers = response.headers_mut();
    if overriding || !headers.contains_key(header::CACHE_CONTROL) {
        headers.insert(header::CACHE_CONTROL, HeaderValue::from_static(rule));
    }
}

/// Applies the rules above to the answer of every request below `/api`.
pub(crate) async fn cache_rules(request: Request, next: Next) -> Response {
    if !is_api(request.uri().path()) {
        return next.run(request).await;
    }
    let method = request.method().clone();
    let secret = request
        .extensions()
        .get::<MatchedPath>()
        .is_some_and(|route| is_secret_answer(&method, route.as_str()));
    let conditions = request.headers().clone();
    let mut response = next.run(request).await;
    if secret {
        response.headers_mut().remove(header::ETAG);
        set_cache_control(&mut response, NEVER_STORE, true);
        response.extensions_mut().insert(SecretAnswer);
        return response;
    }
    let stored_by_choice = response
        .headers()
        .get(header::CACHE_CONTROL)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|rule| rule.contains(NEVER_STORE));
    if !is_read(&method) || response.status() != StatusCode::OK || stored_by_choice {
        set_cache_control(&mut response, NEVER_STORE, false);
        return response;
    }
    revalidate(response, &conditions).await
}

/// Gives a read its `ETag` and its cache rule, and turns it into `304` when the client already
/// has this body.
async fn revalidate(response: Response, conditions: &HeaderMap) -> Response {
    let (mut parts, body) = response.into_parts();
    let Ok(bytes) = axum::body::to_bytes(body, ETAG_BODY_LIMIT).await else {
        return ApiError::internal(&"an answer is too big to give an ETag").into_response();
    };
    let etag = etag_of(&bytes);
    let Ok(value) = HeaderValue::from_str(&etag) else {
        return Response::from_parts(parts, Body::from(bytes));
    };
    parts.headers.insert(header::ETAG, value);
    parts
        .headers
        .entry(header::CACHE_CONTROL)
        .or_insert(HeaderValue::from_static(REVALIDATE));
    if matches(conditions, &etag) {
        parts.status = StatusCode::NOT_MODIFIED;
        parts.headers.remove(header::CONTENT_LENGTH);
        parts.headers.remove(header::CONTENT_TYPE);
        return Response::from_parts(parts, Body::empty());
    }
    Response::from_parts(parts, Body::from(bytes))
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::http::Request;
    use axum::middleware::from_fn;
    use axum::routing::{get, post};
    use tower::ServiceExt as _;

    use super::*;

    fn app() -> Router {
        Router::new()
            .route("/api/v1/thing", get(|| async { r#"{"name":"Ada"}"# }))
            .route("/api/v1/other", get(|| async { r#"{"name":"Bob"}"# }))
            .route(
                "/api/v1/kept",
                get(|| async { ([(header::CACHE_CONTROL, "private, max-age=60")], "x") }),
            )
            .route(
                "/api/v1/hidden",
                get(|| async { ([(header::CACHE_CONTROL, "no-store")], "x") }),
            )
            .route("/api/v1/missing", get(|| async { StatusCode::NOT_FOUND }))
            .route("/api/v1/changes", post(|| async { "done" }))
            .route("/api/v1/auth/sign-in", post(|| async { "a session" }))
            .route("/page", get(|| async { "<h1>Bugaboo</h1>" }))
            .layer(from_fn(cache_rules))
    }

    async fn call(method: &str, path: &str, if_none_match: Option<&str>) -> Response {
        let mut request = Request::builder().method(method).uri(path);
        if let Some(value) = if_none_match {
            request = request.header(header::IF_NONE_MATCH, value);
        }
        app()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    fn header_of(response: &Response, name: header::HeaderName) -> Option<&str> {
        response.headers().get(name).and_then(|v| v.to_str().ok())
    }

    #[tokio::test]
    async fn perf_03_a_read_answers_a_weak_etag_and_may_be_kept_but_asked_about_again() {
        let response = call("GET", "/api/v1/thing", None).await;
        assert_eq!(response.status(), StatusCode::OK);
        let etag = header_of(&response, header::ETAG).unwrap();
        assert!(etag.starts_with("W/\"") && etag.ends_with('"'), "{etag}");
        assert_eq!(
            header_of(&response, header::CACHE_CONTROL),
            Some("private, no-cache")
        );
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body[..], br#"{"name":"Ada"}"#);
    }

    #[tokio::test]
    async fn perf_03_the_same_body_has_the_same_etag_and_another_body_another() {
        let first = call("GET", "/api/v1/thing", None).await;
        let again = call("GET", "/api/v1/thing", None).await;
        let other = call("GET", "/api/v1/other", None).await;
        assert_eq!(
            header_of(&first, header::ETAG),
            header_of(&again, header::ETAG)
        );
        assert_ne!(
            header_of(&first, header::ETAG),
            header_of(&other, header::ETAG)
        );
    }

    #[tokio::test]
    async fn perf_03_a_matching_if_none_match_answers_304_without_a_body() {
        let tag = call("GET", "/api/v1/thing", None).await;
        let tag = header_of(&tag, header::ETAG).unwrap().to_owned();
        let strong = tag.trim_start_matches("W/").to_owned();
        for presented in [
            tag.clone(),
            strong,
            format!("\"nope\", {tag}"),
            "*".to_owned(),
        ] {
            let response = call("GET", "/api/v1/thing", Some(&presented)).await;
            assert_eq!(response.status(), StatusCode::NOT_MODIFIED, "{presented}");
            assert_eq!(header_of(&response, header::ETAG), Some(tag.as_str()));
            assert_eq!(
                header_of(&response, header::CACHE_CONTROL),
                Some("private, no-cache")
            );
            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            assert!(body.is_empty(), "{presented}");
        }
        for presented in ["\"nope\"", "", "W/\"other\"", "x"] {
            let response = call("GET", "/api/v1/thing", Some(presented)).await;
            assert_eq!(response.status(), StatusCode::OK, "{presented}");
        }
        // The tag of another answer never matches.
        let response = call("GET", "/api/v1/other", Some(&tag)).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn perf_03_head_gets_the_etag_too() {
        let response = call("HEAD", "/api/v1/thing", None).await;
        assert!(header_of(&response, header::ETAG).is_some());
    }

    #[tokio::test]
    async fn perf_03_a_route_that_chose_its_own_cache_rule_keeps_it() {
        let kept = call("GET", "/api/v1/kept", None).await;
        assert_eq!(
            header_of(&kept, header::CACHE_CONTROL),
            Some("private, max-age=60")
        );
        assert!(header_of(&kept, header::ETAG).is_some());
        // A route that says no-store is not given a tag a cache could revalidate.
        let hidden = call("GET", "/api/v1/hidden", None).await;
        assert_eq!(header_of(&hidden, header::CACHE_CONTROL), Some("no-store"));
        assert!(header_of(&hidden, header::ETAG).is_none());
    }

    #[tokio::test]
    async fn perf_03_changes_and_failures_are_never_stored() {
        for (method, path) in [("POST", "/api/v1/changes"), ("GET", "/api/v1/missing")] {
            let response = call(method, path, Some("*")).await;
            assert_ne!(response.status(), StatusCode::NOT_MODIFIED, "{path}");
            assert_eq!(
                header_of(&response, header::CACHE_CONTROL),
                Some("no-store"),
                "{path}"
            );
            assert!(header_of(&response, header::ETAG).is_none(), "{path}");
        }
    }

    #[tokio::test]
    async fn perf_03_a_secret_answer_is_never_stored_and_is_marked_for_compression() {
        let response = call("POST", "/api/v1/auth/sign-in", None).await;
        assert_eq!(
            header_of(&response, header::CACHE_CONTROL),
            Some("no-store")
        );
        assert!(header_of(&response, header::ETAG).is_none());
        assert!(response.extensions().get::<SecretAnswer>().is_some());
        // The other answers are not marked.
        let plain = call("GET", "/api/v1/thing", None).await;
        assert!(plain.extensions().get::<SecretAnswer>().is_none());
    }

    #[tokio::test]
    async fn perf_03_what_is_not_an_api_answer_is_left_alone() {
        let response = call("GET", "/page", None).await;
        assert!(header_of(&response, header::ETAG).is_none());
        assert!(header_of(&response, header::CACHE_CONTROL).is_none());
    }

    #[test]
    fn the_secret_answers_are_unique_routes_of_the_api() {
        let mut seen = std::collections::BTreeSet::new();
        for (method, route) in SECRET_ANSWERS {
            assert!(
                seen.insert((method, route)),
                "{method} {route} is listed twice"
            );
            assert!(route.starts_with("/api/v1/"), "{route}");
            assert!(is_secret_answer(&method.parse().unwrap(), route));
        }
        assert!(!is_secret_answer(&Method::GET, "/api/v1/auth/sign-in"));
    }

    /// Every schema below `value`, following references into the document.
    fn schemas_below<'a>(
        document: &'a serde_json::Value,
        value: &'a serde_json::Value,
        found: &mut Vec<&'a serde_json::Value>,
    ) {
        if found.iter().any(|seen| std::ptr::eq(*seen, value)) {
            return;
        }
        found.push(value);
        if let Some(target) = value
            .get("$ref")
            .and_then(serde_json::Value::as_str)
            .and_then(|reference| reference.strip_prefix('#'))
            .and_then(|pointer| document.pointer(pointer))
        {
            schemas_below(document, target, found);
        }
        for key in ["items"] {
            if let Some(inner) = value.get(key) {
                schemas_below(document, inner, found);
            }
        }
        for key in ["properties"] {
            for inner in value
                .get(key)
                .and_then(serde_json::Value::as_object)
                .into_iter()
                .flat_map(serde_json::Map::values)
            {
                schemas_below(document, inner, found);
            }
        }
        for key in ["allOf", "anyOf", "oneOf"] {
            for inner in value
                .get(key)
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
            {
                schemas_below(document, inner, found);
            }
        }
    }

    /// The `x-secret` mark of a field (set with `extensions` on the field of the answer type).
    const SECRET_MARK: &str = "x-secret";
    /// Parts of a field name (or its ending `codes`) that say it holds a secret: such a field must
    /// carry the mark.
    const SECRET_NAMES: [&str; 5] = ["secret", "token", "password", "otpauth", "link_path"];

    fn is_marked(schema: &serde_json::Value) -> bool {
        schema.get(SECRET_MARK) == Some(&serde_json::Value::Bool(true))
    }

    /// Each answer of the API description with the schemas it holds.
    fn answers(document: &serde_json::Value) -> Vec<(String, String, Vec<&serde_json::Value>)> {
        let mut answers = Vec::new();
        let paths = document["paths"].as_object().unwrap();
        for (route, item) in paths {
            for (method, operation) in item.as_object().unwrap() {
                let mut schemas = Vec::new();
                for response in operation["responses"].as_object().unwrap().values() {
                    if let Some(schema) = response.pointer("/content/application~1json/schema") {
                        schemas_below(document, schema, &mut schemas);
                    }
                }
                answers.push((method.to_uppercase(), route.clone(), schemas));
            }
        }
        answers
    }

    #[test]
    fn perf_03_every_route_whose_answer_holds_a_secret_field_is_on_the_list() {
        let document = serde_json::to_value(crate::openapi::document()).unwrap();
        let answers = answers(&document);
        let mut marked = 0;
        for (method, route, schemas) in &answers {
            let holds_secret = schemas.iter().any(|schema| is_marked(schema));
            let listed = is_secret_answer(&method.parse().unwrap(), route);
            marked += usize::from(holds_secret);
            assert!(
                !holds_secret || listed,
                "{method} {route} answers with a field marked {SECRET_MARK} but is not in SECRET_ANSWERS"
            );
        }
        assert!(marked >= 4, "{marked}");
        // The list names routes that exist.
        for (method, route) in SECRET_ANSWERS {
            assert!(
                answers
                    .iter()
                    .any(|(known_method, known_route, _)| known_method == method
                        && known_route == route),
                "{method} {route} is in SECRET_ANSWERS but not in the API description"
            );
        }
    }

    #[test]
    fn perf_03_a_field_named_like_a_secret_must_carry_the_mark() {
        let document = serde_json::to_value(crate::openapi::document()).unwrap();
        for (method, route, schemas) in answers(&document) {
            for schema in schemas {
                let Some(properties) = schema
                    .get("properties")
                    .and_then(serde_json::Value::as_object)
                else {
                    continue;
                };
                for (name, property) in properties {
                    let lower = name.to_lowercase();
                    let looks_secret = lower.ends_with("codes")
                        || SECRET_NAMES.iter().any(|part| lower.contains(part));
                    let marked =
                        is_marked(property) || property.get("items").is_some_and(is_marked);
                    assert!(
                        !looks_secret || marked,
                        "{method} {route}: the answer field `{name}` looks like a secret but is not marked {SECRET_MARK}"
                    );
                }
            }
        }
    }
}
