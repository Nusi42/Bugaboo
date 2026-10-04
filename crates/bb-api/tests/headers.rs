//! SEC-11: the running router sends every security header on every kind of response.
//!
//! The expected values are written out here on purpose, so changing a header in the code
//! without changing the requirement (WP-05, architecture §7) fails this test.

// Dependencies of the library that this test does not use itself (`unused_crate_dependencies`).
use mime_guess as _;
use rust_embed as _;
use tokio as _;
use tower_http as _;

#[cfg(test)] // Clippy lets tests unwrap only inside test modules (clippy.toml).
mod headers {
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use axum::response::Response;
    use bb_api::{WebSource, router};
    use tower::ServiceExt as _;

    const EXPECTED: [(&str, &str); 6] = [
        (
            "content-security-policy",
            "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; \
             font-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; \
             form-action 'self'; require-trusted-types-for 'script'",
        ),
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "same-origin"),
        ("cross-origin-opener-policy", "same-origin"),
        ("cross-origin-resource-policy", "same-origin"),
        (
            "permissions-policy",
            "accelerometer=(), camera=(), display-capture=(), geolocation=(), gyroscope=(), \
             magnetometer=(), microphone=(), payment=(), usb=(), browsing-topics=()",
        ),
    ];

    fn app_folder() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.html"), "<h1>Bugaboo</h1>").unwrap();
        std::fs::create_dir(dir.path().join("assets")).unwrap();
        std::fs::write(dir.path().join("assets/app-1a2b.js"), "export {};").unwrap();
        dir
    }

    async fn send(web: WebSource, method: Method, path: &str) -> Response {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .body(Body::empty())
            .unwrap();
        router(web).oneshot(request).await.unwrap()
    }

    fn assert_headers(response: &Response, what: &str) {
        for (name, value) in EXPECTED {
            let actual = response.headers().get(name).map(|v| v.to_str().unwrap());
            assert_eq!(actual, Some(value), "{name} on {what}");
        }
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
            "/health/ready",
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
