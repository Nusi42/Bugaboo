//! SEC-11: the running router sends every security header on every kind of response.
//!
//! The expected values are written out here on purpose, so changing a header in the code
//! without changing the requirement (SEC-11, architecture §7) fails this test (SEC-HEADERS).

// Dependencies of the library that this test does not use itself (`unused_crate_dependencies`).
use bb_core as _;
use hyper_util as _;
use mime_guess as _;
use rcgen as _;
use rust_embed as _;
use rustls as _;
use serde as _;
use serde_json as _;
use tokio as _;
use tokio_rustls as _;
use tower_http as _;
use tracing as _;
use tracing_subscriber as _;
use utoipa as _;
use utoipa_axum as _;

mod common;

#[cfg(test)] // Clippy lets tests unwrap only inside test modules (clippy.toml).
mod headers {
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use axum::response::Response;
    use bb_api::{Options, WebSource, router};

    use super::common::{EXPECTED, HSTS_NAME, HSTS_SELF_SIGNED, HSTS_TRUSTED};
    use tower::ServiceExt as _;

    fn app_folder() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.html"), "<h1>Bugaboo</h1>").unwrap();
        std::fs::create_dir(dir.path().join("assets")).unwrap();
        std::fs::write(dir.path().join("assets/app-1a2b.js"), "export {};").unwrap();
        dir
    }

    fn options(web: WebSource, hsts_max_age: Option<u64>) -> Options {
        Options {
            web,
            hsts_max_age,
            header_timeout: std::time::Duration::from_secs(30),
            data_folder: std::env::temp_dir(),
            trusted_proxies: vec![],
            limits: bb_api::LimitsConfig::default(),
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
        for (name, value) in EXPECTED {
            let actual = response.headers().get(name).map(|v| v.to_str().unwrap());
            assert_eq!(actual, Some(value), "{name} on {what}");
        }
    }

    #[tokio::test]
    async fn sec_11_headers_on_a_page_an_asset_an_api_error_and_a_404() {
        let dir = app_folder();
        let cases = [
            ("/", StatusCode::OK),
            ("/assets/app-1a2b.js", StatusCode::OK),
            ("/api/anything", StatusCode::NOT_FOUND),
            ("/missing.js", StatusCode::NOT_FOUND),
        ];
        for (path, status) in cases {
            let response = send(WebSource::Folder(dir.path().into()), Method::GET, path).await;
            assert_eq!(response.status(), status, "{path}");
            assert_headers(&response, path);
        }
        let post = send(WebSource::Folder(dir.path().into()), Method::POST, "/api/x").await;
        assert_eq!(post.status(), StatusCode::NOT_FOUND);
        assert_headers(&post, "POST /api/x");
    }

    #[tokio::test]
    async fn sec_11_hsts_depends_on_https_and_on_who_made_the_certificate() {
        let dir = app_folder();
        let web = || WebSource::Folder(dir.path().into());
        for (seconds, expected) in [
            (bb_api::HSTS_MAX_AGE_SELF_SIGNED, HSTS_SELF_SIGNED),
            (bb_api::HSTS_MAX_AGE_TRUSTED, HSTS_TRUSTED),
        ] {
            let secure = send_with(options(web(), Some(seconds)), Method::GET, "/").await;
            let value = secure.headers().get(HSTS_NAME).unwrap().to_str().unwrap();
            assert_eq!(value, expected);
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

    #[tokio::test]
    async fn sec_11_headers_on_success_responses() {
        let dir = app_folder();
        for path in ["/health/live", "/", "/some/page", "/assets/app-1a2b.js"] {
            let response = send(WebSource::Folder(dir.path().into()), Method::GET, path).await;
            assert_eq!(response.status(), StatusCode::OK, "{path}");
            assert_headers(&response, path);
        }
    }

    #[tokio::test]
    async fn sec_11_headers_on_not_found() {
        let dir = app_folder();
        for path in [
            "/missing.js",
            "/assets/missing.js",
            "/api/x",
            "/health/other",
        ] {
            let response = send(WebSource::Folder(dir.path().into()), Method::GET, path).await;
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
            assert_headers(&response, path);
        }
        let without_app = send(WebSource::Folder("no/such/folder".into()), Method::GET, "/").await;
        assert_eq!(without_app.status(), StatusCode::NOT_FOUND);
        assert_headers(&without_app, "/ without app");
    }

    #[tokio::test]
    async fn sec_11_headers_on_method_not_allowed() {
        let dir = app_folder();
        for path in ["/health/live", "/", "/some/page"] {
            let response = send(WebSource::Folder(dir.path().into()), Method::POST, path).await;
            assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED, "{path}");
            assert_headers(&response, path);
        }
    }

    #[tokio::test]
    async fn sec_11_headers_on_head_requests() {
        let dir = app_folder();
        for path in ["/health/live", "/", "/assets/app-1a2b.js", "/missing.js"] {
            let response = send(WebSource::Folder(dir.path().into()), Method::HEAD, path).await;
            assert_headers(&response, path);
        }
    }
}
