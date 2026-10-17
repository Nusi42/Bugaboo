//! SEC-11: the running router sends every security header on every kind of response.
//!
//! The values come from the one table in the code (`bb_api::SECURITY_HEADERS`); that the table says
//! what SEC-11 (architecture §7) demands is SEC-HEADERS' rule, checked there only.

// Dependencies of the library that this test does not use itself (`unused_crate_dependencies`).
use base64ct as _;
use bb_api_words as _;
use bb_app as _;
use bb_auth as _;
use bb_core as _;
use bb_dev_words as _;
use bb_policy as _;
use brotli as _;
use futures_util as _;
use getrandom as _;
use h2 as _;
use hmac as _;
use hyper_util as _;
use mime_guess as _;
use proptest as _;
use rcgen as _;
use rust_embed as _;
use rustls as _;
use serde as _;
use serde_json as _;
use sha2 as _;
use tokio as _;
use tokio_rustls as _;
use tower_http as _;
use tracing as _;
use tracing_subscriber as _;
use utoipa as _;
use utoipa_axum as _;

#[cfg(test)] // Clippy lets tests unwrap only inside test modules (clippy.toml).
mod headers {
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use axum::response::Response;
    use bb_api::{Options, WebSource, router};

    use bb_api::{HSTS_NAME, SECURITY_HEADERS, hsts_value};
    use tower::ServiceExt as _;

    fn app_folder() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        Options::write_test_app(dir.path()).unwrap();
        dir
    }

    fn options(web: WebSource, hsts_max_age: Option<u64>) -> Options {
        Options {
            web,
            hsts_max_age,
            ..Options::for_tests()
        }
    }

    async fn send_with(options: Options, method: Method, path: &str) -> Response {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .body(Body::empty())
            .unwrap();
        router(options).oneshot(request).await.unwrap()
    }

    async fn send(web: WebSource, method: Method, path: &str) -> Response {
        send_with(options(web, None), method, path).await
    }

    fn assert_headers(response: &Response, what: &str) {
        for (name, value) in SECURITY_HEADERS {
            let actual = response.headers().get(name).map(|v| v.to_str().unwrap());
            assert_eq!(actual, Some(value), "{name} on {what}");
        }
    }

    /// Every kind of response with the method and address that cause it, and its status.
    const KINDS: [(Method, &str, StatusCode); 18] = [
        (Method::GET, "/", StatusCode::OK),
        (Method::GET, "/some/page", StatusCode::OK),
        (Method::GET, "/assets/app-1a2b.js", StatusCode::OK),
        (Method::GET, "/health/live", StatusCode::OK),
        (Method::GET, "/missing.js", StatusCode::NOT_FOUND),
        (Method::GET, "/assets/missing.js", StatusCode::NOT_FOUND),
        (Method::GET, "/api/anything", StatusCode::NOT_FOUND),
        (Method::GET, "/health/other", StatusCode::NOT_FOUND),
        (Method::POST, "/", StatusCode::METHOD_NOT_ALLOWED),
        (Method::POST, "/some/page", StatusCode::METHOD_NOT_ALLOWED),
        (Method::POST, "/health/live", StatusCode::METHOD_NOT_ALLOWED),
        // A change without an origin is refused before the address is looked at; the headers stay.
        (Method::POST, "/api/x", StatusCode::FORBIDDEN),
        (Method::HEAD, "/", StatusCode::OK),
        (Method::HEAD, "/assets/app-1a2b.js", StatusCode::OK),
        (Method::HEAD, "/health/live", StatusCode::OK),
        (Method::HEAD, "/missing.js", StatusCode::NOT_FOUND),
        (Method::HEAD, "/api/anything", StatusCode::NOT_FOUND),
        (Method::HEAD, "/health/other", StatusCode::NOT_FOUND),
    ];

    #[tokio::test]
    async fn sec_11_every_kind_of_response_carries_the_security_headers() {
        let dir = app_folder();
        for (method, path, status) in KINDS {
            let web = WebSource::Folder(dir.path().into());
            let response = send(web, method.clone(), path).await;
            assert_eq!(response.status(), status, "{method} {path}");
            assert_headers(&response, &format!("{method} {path}"));
        }
        let without_app = send(WebSource::Folder("no/such/folder".into()), Method::GET, "/").await;
        assert_eq!(without_app.status(), StatusCode::NOT_FOUND);
        assert_headers(&without_app, "/ without app");
    }

    #[tokio::test]
    async fn sec_11_hsts_depends_on_https_and_on_who_made_the_certificate() {
        let dir = app_folder();
        let web = || WebSource::Folder(dir.path().into());
        for seconds in [
            bb_api::HSTS_MAX_AGE_SELF_SIGNED,
            bb_api::HSTS_MAX_AGE_TRUSTED,
        ] {
            let secure = send_with(options(web(), Some(seconds)), Method::GET, "/").await;
            let value = secure.headers().get(HSTS_NAME).unwrap().to_str().unwrap();
            assert_eq!(value, hsts_value(seconds));
            assert!(!value.to_lowercase().contains("includesubdomains"));
            assert_headers(&secure, "https");
        }
        let plain = send_with(options(web(), None), Method::GET, "/").await;
        assert!(plain.headers().get(HSTS_NAME).is_none());
    }

    #[tokio::test]
    async fn ops_08_health_answers_are_never_cached() {
        let dir = app_folder();
        let response = send(
            WebSource::Folder(dir.path().into()),
            Method::GET,
            "/health/live",
        )
        .await;
        assert_eq!(response.headers().get("cache-control").unwrap(), "no-store");
    }

    #[tokio::test]
    async fn ops_08_every_response_has_a_request_id() {
        let dir = app_folder();
        let first = send(WebSource::Folder(dir.path().into()), Method::GET, "/").await;
        let second = send(
            WebSource::Folder(dir.path().into()),
            Method::GET,
            "/missing.js",
        )
        .await;
        let id = |response: &Response| response.headers().get("x-request-id").unwrap().clone();
        assert_ne!(id(&first), id(&second));
    }
}
