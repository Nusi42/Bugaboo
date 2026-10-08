//! What a browser or a cache may do with an API answer: entity tags and `304`, and the cache rules
//! (PERF-03, architecture §7 "Performance").
//!
//! One middleware, [`cache_rules`], decides for every answer below `/api`, so a route never
//! writes a cache header and a route added later cannot forget one. Which of three kinds an answer
//! is comes from the route table (`crate::permissions`, the same table that puts the marks into the
//! API description, [`Classes`]) and from the answer itself:
//!
//! - **A secret** (a route the table declares `Secret`, marked `x-secret` in the API description,
//!   or any answer that sets a cookie: a new session, an invitation link, the code app, backup
//!   codes) is `no-store`, has no `ETag`, and is marked [`SecretAnswer`]. The compression of
//!   `crate::router` never compresses a marked answer, nor one that sets a cookie (BREACH, M2-Q2,
//!   architecture §7 "Browser protections").
//! - **Personal data** (a `GET` route the table declares `NeverStored`, marked `x-never-stored`:
//!   who is signed in, the sessions, the invitations, the second step, the state of the server) is
//!   `no-store` and has no `ETag`: a shared computer keeps nothing of it after the person has
//!   left.
//! - **Everything else** that is a read (`GET`, `HEAD` answered `200`) gets a weak `ETag` made
//!   with the seal of this server run from the final answer body, and `Cache-Control: private,
//!   no-cache`: the browser keeps it but asks again every time, with `If-None-Match`, and a match
//!   answers `304` without a body. The tag is made after the answer is complete (after masking), so
//!   two people with different rights never get the same tag for different text. Answers to
//!   changes and errors are `no-store`.
//!
//! An answer that clears the session cookie also tells the browser to forget what it cached
//! (`Clear-Site-Data: "cache"`). The files of the browser app are not API answers: they keep their
//! content hash and `immutable` (`crate::router`).

use std::collections::BTreeSet;
use std::sync::OnceLock;

use axum::body::{Body, HttpBody as _};
use axum::extract::{MatchedPath, Request};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::Response;
use base64ct::{Base64UrlUnpadded, Encoding as _};

use crate::api::is_api;
use crate::cookies::{Cookie, sets_a_cookie};
use crate::openapi::Operation;
use crate::permissions::Answer;
use crate::seal;

/// The cache rule of an answer a browser may keep and ask about again.
const REVALIDATE: &str = "private, no-cache";
/// The cache rule of everything that must not be stored.
const NEVER_STORE: &str = "no-store";
/// What tells a browser to drop what it cached for this website.
const CLEAR_CACHE: HeaderName = HeaderName::from_static("clear-site-data");
/// The biggest answer that is read into memory to make its `ETag`; a bigger one, or one of
/// unknown size, is passed on as it is, without a tag.
const ETAG_BODY_LIMIT: usize = 16 * 1024 * 1024;
/// What the tag of an answer is bound to in the seal.
const ETAG_PURPOSE: &str = "etag";

/// Put on the response of an answer that holds a secret, for the layers outside (compression
/// skips it).
#[derive(Clone, Copy, Debug)]
pub(crate) struct SecretAnswer;

/// What kind of answer a route gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Secret,
    PersonalData,
    Ordinary,
}

/// The routes of each kind, as the API description says them (method and route).
#[derive(Debug, Default)]
struct Classes {
    secret: BTreeSet<(String, String)>,
    personal: BTreeSet<(String, String)>,
}

impl Classes {
    fn kind_of(&self, method: &Method, route: &str) -> Kind {
        // A `HEAD` is answered by the route of the `GET`, so it is the same kind.
        let method = if *method == Method::HEAD {
            Method::GET
        } else {
            method.clone()
        };
        let key = (method.as_str().to_owned(), route.to_owned());
        if self.secret.contains(&key) {
            Kind::Secret
        } else if self.personal.contains(&key) {
            Kind::PersonalData
        } else {
            Kind::Ordinary
        }
    }
}

/// The routes of each kind, from the table of `crate::permissions` through the operations of the
/// API description.
fn classes_of(operations: &[Operation]) -> Classes {
    let mut classes = Classes::default();
    for operation in operations {
        let key = (operation.method.as_str().to_owned(), operation.path.clone());
        match operation.kind {
            Answer::Secret => {
                classes.secret.insert(key);
            }
            Answer::NeverStored => {
                classes.personal.insert(key);
            }
            Answer::Ordinary => {}
        }
    }
    classes
}

/// The kinds of the routes of this API, read from the table once.
fn classes() -> &'static Classes {
    static CLASSES: OnceLock<Classes> = OnceLock::new();
    CLASSES.get_or_init(|| classes_of(crate::openapi::operations()))
}

/// The weak `ETag` of an answer body, or `None` when the seal has no key.
fn etag_of(body: &[u8]) -> Option<String> {
    let tag = seal::process()?.tag(ETAG_PURPOSE, body)?;
    Some(format!("W/\"{}\"", Base64UrlUnpadded::encode_string(&tag)))
}

/// The part of a tag that is compared: weak comparison ignores the `W/` mark (RFC 9110 §8.8.3.2).
fn opaque(tag: &str) -> &str {
    tag.trim().trim_start_matches("W/")
}

/// Whether `If-None-Match` names the tag, or is `*`.
fn matches(conditions: &HeaderMap, etag: &str) -> bool {
    conditions
        .get_all(header::IF_NONE_MATCH)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .any(|tag| tag.trim() == "*" || opaque(tag) == opaque(etag))
}

fn is_read(method: &Method) -> bool {
    matches!(*method, Method::GET | Method::HEAD)
}

/// Makes the answer something no cache keeps: `no-store` (always, or only when the route did not
/// choose a rule) and no tag.
fn never_store(response: &mut Response, overriding: bool) {
    let headers = response.headers_mut();
    if overriding || !headers.contains_key(header::CACHE_CONTROL) {
        headers.insert(header::CACHE_CONTROL, HeaderValue::from_static(NEVER_STORE));
    }
    if overriding {
        headers.remove(header::ETAG);
    }
}

/// Applies the rules above to the answer of every request below `/api`.
pub(crate) async fn cache_rules(request: Request, next: Next) -> Response {
    if !is_api(request.uri().path()) {
        return next.run(request).await;
    }
    let method = request.method().clone();
    let kind = request
        .extensions()
        .get::<MatchedPath>()
        .map_or(Kind::Ordinary, |route| {
            classes().kind_of(&method, route.as_str())
        });
    let conditions = request.headers().clone();
    let mut response = next.run(request).await;
    if Cookie::Session.is_cleared_in(response.headers()) {
        response
            .headers_mut()
            .insert(CLEAR_CACHE, HeaderValue::from_static("\"cache\""));
    }
    if kind == Kind::Secret || sets_a_cookie(response.headers()) {
        never_store(&mut response, true);
        response.extensions_mut().insert(SecretAnswer);
        return response;
    }
    if kind == Kind::PersonalData {
        never_store(&mut response, true);
        return response;
    }
    let stored_by_choice = response
        .headers()
        .get(header::CACHE_CONTROL)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|rule| rule.contains(NEVER_STORE));
    if !is_read(&method) || response.status() != StatusCode::OK || stored_by_choice {
        never_store(&mut response, false);
        return response;
    }
    revalidate(response, &conditions).await
}

/// Gives a read its `ETag` and its cache rule, and turns it into `304` when the client already
/// has this body. An answer that is too big, of unknown size or cannot be tagged is passed on as
/// it is, `no-store`.
async fn revalidate(response: Response, conditions: &HeaderMap) -> Response {
    let (mut parts, body) = response.into_parts();
    let small = body
        .size_hint()
        .upper()
        .and_then(|bytes| usize::try_from(bytes).ok())
        .is_some_and(|bytes| bytes <= ETAG_BODY_LIMIT);
    if !small {
        let mut passed = Response::from_parts(parts, body);
        never_store(&mut passed, true);
        return passed;
    }
    let Ok(bytes) = axum::body::to_bytes(body, ETAG_BODY_LIMIT).await else {
        // The body failed while it was read: nothing is left to send but an empty answer.
        let mut failed = Response::from_parts(parts, Body::empty());
        *failed.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        never_store(&mut failed, true);
        return failed;
    };
    let tag = etag_of(&bytes)
        .and_then(|etag| HeaderValue::from_str(&etag).ok().map(|value| (etag, value)));
    let Some((etag, value)) = tag else {
        let mut untagged = Response::from_parts(parts, Body::from(bytes));
        never_store(&mut untagged, true);
        return untagged;
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
    use axum::routing::{MethodFilter, get, on, post};
    use serde_json::Value;
    use tower::ServiceExt as _;

    use super::*;

    /// Parts of a field name (or its ending `codes`) that say it holds a secret: a route whose answer
    /// has such a field is declared a secret in the table.
    const SECRET_NAMES: [&str; 5] = ["secret", "token", "password", "otpauth", "link_path"];

    fn app() -> Router {
        let big = vec![b'x'; ETAG_BODY_LIMIT + 1];
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
            .route(
                "/api/v1/cookie",
                get(|| async { ([(header::SET_COOKIE, "a=b")], "a session") }),
            )
            .route(
                "/api/v1/signs-out",
                post(|| async {
                    let mut response = Response::new(Body::empty());
                    Cookie::Session.forget(&mut response);
                    response
                }),
            )
            .route("/api/v1/auth/me", get(|| async { r#"{"name":"Ada"}"# }))
            .route("/api/v1/auth/invitations", post(|| async { "a link" }))
            .route(
                "/api/v1/big",
                get(move || {
                    let big = big.clone();
                    async move { big }
                }),
            )
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

    fn header_of(response: &Response, name: HeaderName) -> Option<&str> {
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
    async fn perf_03_head_gets_the_same_etag_as_get() {
        let read = call("GET", "/api/v1/thing", None).await;
        let head = call("HEAD", "/api/v1/thing", None).await;
        assert!(header_of(&head, header::ETAG).is_some());
        assert_eq!(
            header_of(&head, header::ETAG),
            header_of(&read, header::ETAG)
        );
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
    async fn perf_03_an_answer_that_sets_a_cookie_is_a_secret_whatever_the_route() {
        let response = call("GET", "/api/v1/cookie", Some("*")).await;
        assert_eq!(response.status(), StatusCode::OK);
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
    async fn perf_03_an_answer_with_a_secret_field_is_never_stored_and_marked_as_a_secret() {
        // The invitation link: a route of the real API description whose answer holds a secret.
        let response = call("POST", "/api/v1/auth/invitations", None).await;
        assert_eq!(
            header_of(&response, header::CACHE_CONTROL),
            Some("no-store")
        );
        assert!(header_of(&response, header::ETAG).is_none());
        assert!(response.extensions().get::<SecretAnswer>().is_some());
    }

    #[tokio::test]
    async fn perf_03_an_answer_with_personal_data_is_never_stored_and_has_no_tag() {
        let response = call("GET", "/api/v1/auth/me", Some("*")).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            header_of(&response, header::CACHE_CONTROL),
            Some("no-store")
        );
        assert!(header_of(&response, header::ETAG).is_none());
        // Not a secret: compression may treat it like any other answer.
        assert!(response.extensions().get::<SecretAnswer>().is_none());
        // `HEAD` is the same route as `GET`: the same rule, no tag.
        let head = call("HEAD", "/api/v1/auth/me", None).await;
        assert_eq!(header_of(&head, header::CACHE_CONTROL), Some("no-store"));
        assert!(header_of(&head, header::ETAG).is_none());
    }

    #[tokio::test]
    async fn sec_12_an_answer_that_clears_the_session_cookie_clears_the_cache_too() {
        let response = call("POST", "/api/v1/signs-out", None).await;
        assert_eq!(header_of(&response, CLEAR_CACHE), Some("\"cache\""));
        assert_eq!(
            header_of(&response, header::CACHE_CONTROL),
            Some("no-store")
        );
        // An ordinary answer does not.
        let ordinary = call("GET", "/api/v1/thing", None).await;
        assert!(header_of(&ordinary, CLEAR_CACHE).is_none());
    }

    #[tokio::test]
    async fn perf_03_an_answer_over_the_limit_is_passed_on_whole_without_a_tag() {
        let response = call("GET", "/api/v1/big", None).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            header_of(&response, header::CACHE_CONTROL),
            Some("no-store")
        );
        assert!(header_of(&response, header::ETAG).is_none());
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(body.len(), ETAG_BODY_LIMIT + 1);
    }

    #[tokio::test]
    async fn perf_03_what_is_not_an_api_answer_is_left_alone() {
        let response = call("GET", "/page", None).await;
        assert!(header_of(&response, header::ETAG).is_none());
        assert!(header_of(&response, header::CACHE_CONTROL).is_none());
    }

    fn real_document() -> Value {
        serde_json::to_value(crate::openapi::document()).unwrap()
    }

    /// Every schema below `value`, following references into the document.
    fn schemas_below<'a>(document: &'a Value, value: &'a Value, found: &mut Vec<&'a Value>) {
        if found.iter().any(|seen| std::ptr::eq(*seen, value)) {
            return;
        }
        found.push(value);
        if let Some(target) = value
            .get("$ref")
            .and_then(Value::as_str)
            .and_then(|reference| reference.strip_prefix('#'))
            .and_then(|pointer| document.pointer(pointer))
        {
            schemas_below(document, target, found);
        }
        if let Some(inner) = value.get("items") {
            schemas_below(document, inner, found);
        }
        for inner in value
            .get("properties")
            .and_then(Value::as_object)
            .into_iter()
            .flat_map(serde_json::Map::values)
        {
            schemas_below(document, inner, found);
        }
        for key in ["allOf", "anyOf", "oneOf"] {
            for inner in value
                .get(key)
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                schemas_below(document, inner, found);
            }
        }
    }

    /// Each operation of the table with the schemas its answers hold.
    fn answers(document: &Value) -> Vec<(&'static Operation, Vec<&Value>)> {
        crate::openapi::operations()
            .iter()
            .map(|operation| {
                let described =
                    &document["paths"][&operation.path][operation.method.as_str().to_lowercase()];
                let mut schemas = Vec::new();
                for response in described["responses"].as_object().unwrap().values() {
                    if let Some(schema) = response.pointer("/content/application~1json/schema") {
                        schemas_below(document, schema, &mut schemas);
                    }
                }
                (operation, schemas)
            })
            .collect()
    }

    /// Asks `app` for `method` and `path` with every kind of compression the server knows, and says
    /// how the answer was compressed.
    async fn encoding_of(app: &Router, method: &Method, path: &str) -> Option<String> {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .header(header::ACCEPT_ENCODING, "br, gzip")
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{method} {path}");
        header_of(&response, header::CONTENT_ENCODING).map(str::to_owned)
    }

    /// A big JSON answer from every route of the API description, through the rules and the
    /// compression of the server: the answers of the routes that hand out a secret are never
    /// compressed and the answers of every other route are, so the table decides.
    #[tokio::test]
    async fn perf_03_no_route_that_hands_out_a_secret_is_compressed() {
        let classes = classes();
        let mut app = Router::new();
        let mut routes = Vec::new();
        for operation in crate::openapi::operations() {
            let filter = MethodFilter::try_from(operation.method.clone()).unwrap();
            app = app.route(
                &operation.path,
                on(filter, || async {
                    (
                        [(header::CONTENT_TYPE, "application/json")],
                        "x".repeat(4096),
                    )
                }),
            );
            let path = operation
                .path
                .split('/')
                .map(|part| if part.starts_with('{') { "1" } else { part })
                .collect::<Vec<_>>()
                .join("/");
            routes.push((&operation.method, &operation.path, path));
        }
        let app = app
            .layer(from_fn(cache_rules))
            .layer(crate::router::compression_layer());
        let (mut secret, mut other) = (0, 0);
        for (method, route, path) in &routes {
            let encoding = encoding_of(&app, method, path).await;
            if classes.kind_of(method, route) == Kind::Secret {
                secret += 1;
                assert_eq!(encoding, None, "{method} {route} hands out a secret");
            } else {
                other += 1;
                assert_eq!(encoding.as_deref(), Some("br"), "{method} {route}");
            }
        }
        assert!(secret > 0, "no route hands out a secret");
        assert!(other > secret, "{other}");
    }

    #[tokio::test]
    async fn perf_03_a_big_answer_that_sets_a_cookie_is_never_compressed_whatever_the_route() {
        let app = Router::new()
            .route(
                "/api/v1/cookie",
                get(|| async {
                    (
                        [
                            (header::CONTENT_TYPE, "application/json"),
                            (header::SET_COOKIE, "a=b"),
                        ],
                        "x".repeat(4096),
                    )
                }),
            )
            .route(
                "/api/v1/plain",
                get(|| async {
                    (
                        [(header::CONTENT_TYPE, "application/json")],
                        "x".repeat(4096),
                    )
                }),
            )
            .layer(from_fn(cache_rules))
            .layer(crate::router::compression_layer());
        assert_eq!(
            encoding_of(&app, &Method::GET, "/api/v1/cookie").await,
            None
        );
        assert!(
            encoding_of(&app, &Method::GET, "/api/v1/plain")
                .await
                .is_some()
        );
    }

    #[test]
    fn perf_03_a_route_whose_answer_has_a_field_named_like_a_secret_is_declared_a_secret() {
        let document = real_document();
        for (operation, schemas) in answers(&document) {
            for schema in schemas {
                let Some(properties) = schema.get("properties").and_then(Value::as_object) else {
                    continue;
                };
                for name in properties.keys() {
                    let lower = name.to_lowercase();
                    let looks_secret = lower.ends_with("codes")
                        || SECRET_NAMES.iter().any(|part| lower.contains(part));
                    assert!(
                        !looks_secret || operation.kind == Answer::Secret,
                        "{} {}: the answer field `{name}` looks like a secret but the route is not declared Secret in the table",
                        operation.method,
                        operation.path
                    );
                }
            }
        }
    }

    #[test]
    fn the_table_decides_the_kind_of_a_route() {
        let operation = |method: Method, path: &str, kind: Answer| Operation {
            method,
            path: path.to_owned(),
            id: path.to_owned(),
            needs: crate::permissions::Declared::Public,
            kind,
        };
        let classes = classes_of(&[
            operation(Method::GET, "/a", Answer::NeverStored),
            operation(Method::POST, "/b", Answer::Secret),
            operation(Method::GET, "/c", Answer::Ordinary),
            operation(Method::PUT, "/d", Answer::Ordinary),
        ]);
        assert_eq!(classes.kind_of(&Method::GET, "/a"), Kind::PersonalData);
        assert_eq!(classes.kind_of(&Method::HEAD, "/a"), Kind::PersonalData);
        assert_eq!(classes.kind_of(&Method::POST, "/b"), Kind::Secret);
        assert_eq!(classes.kind_of(&Method::GET, "/c"), Kind::Ordinary);
        assert_eq!(classes.kind_of(&Method::PUT, "/d"), Kind::Ordinary);
        assert_eq!(classes.kind_of(&Method::GET, "/b"), Kind::Ordinary);
    }
}
