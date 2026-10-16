//! The tests of router.

use std::fmt::Write as _;

use axum::body::Body;
use axum::http::{Method, Request};
use tower::ServiceExt as _;

use super::*;

fn app_folder() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    Options::write_test_app(dir.path()).unwrap();
    std::fs::write(dir.path().join("robots.txt"), "x").unwrap();
    std::fs::write(dir.path().join("my file.txt"), "spaced").unwrap();
    dir
}

async fn get_from(web: WebSource, path: &str) -> Response {
    let request = Request::get(path).body(Body::empty()).unwrap();
    router(options(web)).oneshot(request).await.unwrap()
}

fn options(web: WebSource) -> Options {
    Options {
        web,
        ..Options::for_tests()
    }
}

async fn body(response: Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

fn header_of<'a>(response: &'a Response, name: &str) -> &'a str {
    response.headers().get(name).unwrap().to_str().unwrap()
}

#[tokio::test]
async fn dist_01_health_live_answers_ok_json() {
    let dir = app_folder();
    let response = get_from(WebSource::Folder(dir.path().into()), "/health/live").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(header_of(&response, "content-type"), "application/json");
    assert_eq!(body(response).await, r#"{"status":"ok"}"#);
}

#[tokio::test]
async fn ops_08_health_ready_checks_the_data_folder() {
    let data = tempfile::tempdir().unwrap();
    let mut ready = options(WebSource::Folder("none".into()));
    ready.data_folder = data.path().to_path_buf();
    let request = Request::get("/health/ready").body(Body::empty()).unwrap();
    let response = router(ready).oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(body(response).await.contains(r#""data_folder":"ok""#));

    let mut missing = options(WebSource::Folder("none".into()));
    missing.data_folder = data.path().join("not-there");
    let request = Request::get("/health/ready").body(Body::empty()).unwrap();
    let response = router(missing).oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(body(response).await.contains("unavailable"));
}

#[tokio::test]
async fn dist_01_unknown_api_addresses_are_not_found_for_every_method() {
    let dir = app_folder();
    for method in [
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::PATCH,
    ] {
        for path in [
            "/api/x",
            "/api",
            "/api/",
            "/%61pi/x",
            "/api/%zz",
            "/./api/x",
            "/%2e/api/x",
            "/API/x",
        ] {
            let request = Request::builder()
                .method(method.clone())
                .uri(path)
                .header("sec-fetch-site", "same-origin")
                .body(Body::empty())
                .unwrap();
            let response = router(options(WebSource::Folder(dir.path().into())))
                .oneshot(request)
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{method} {path}");
        }
    }
}

#[tokio::test]
async fn dist_01_serves_files_with_content_types_and_cache_rules() {
    let dir = app_folder();
    let web = || WebSource::Folder(dir.path().into());
    let page = get_from(web(), "/").await;
    assert_eq!(header_of(&page, "content-type"), "text/html; charset=utf-8");
    assert_eq!(header_of(&page, "cache-control"), "no-cache");
    assert_eq!(body(page).await, "<h1>Bugaboo</h1>");

    let script = get_from(web(), "/assets/app-1a2b.js").await;
    assert!(header_of(&script, "content-type").contains("javascript"));
    assert!(header_of(&script, "cache-control").contains("immutable"));

    let text = get_from(web(), "/robots.txt").await;
    assert_eq!(header_of(&text, "cache-control"), "no-cache");
}

#[tokio::test]
async fn perf_03_files_with_a_content_hash_are_kept_forever_and_api_answers_are_asked_again() {
    let dir = app_folder();
    let web = || WebSource::Folder(dir.path().into());
    // The file name holds the hash of the content: nothing to ask about.
    let script = get_from(web(), "/assets/app-1a2b.js").await;
    assert_eq!(
        header_of(&script, "cache-control"),
        "public, max-age=31536000, immutable"
    );
    // An API answer through the whole router: a tag, kept privately, `304` on a match.
    let status = get_from(web(), "/api/v1/status").await;
    assert_eq!(header_of(&status, "cache-control"), "private, no-cache");
    let tag = header_of(&status, "etag").to_owned();
    let request = Request::get("/api/v1/status")
        .header("if-none-match", &tag)
        .body(Body::empty())
        .unwrap();
    let again = router(options(web())).oneshot(request).await.unwrap();
    assert_eq!(again.status(), StatusCode::NOT_MODIFIED);
    assert!(body(again).await.is_empty());
}

#[test]
fn perf_04_the_compression_settings_are_readable() {
    // Brotli takes 0 to 11; the number itself is the settings file's to decide.
    assert!(compression_quality().is_some_and(|quality| (0..=11).contains(&quality)));
}

/// What the budget check counts is what a browser gets: the script, asked for with Brotli,
/// comes back as many bytes as the Brotli of the file at the quality of the settings file.
#[tokio::test]
async fn perf_04_the_server_compresses_with_the_quality_the_budget_is_measured_with() {
    // Text that compresses very differently at different qualities.
    let mut script = String::new();
    for n in 0..4000 {
        writeln!(
            script,
            "export const item{n} = {{ name: \"entry {}\", weight: {} }};",
            n * 7919 % 1013,
            n * 31 % 97
        )
        .unwrap();
    }
    let dir = app_folder();
    std::fs::write(dir.path().join("assets/big-3c4d.js"), &script).unwrap();
    let request = Request::get("/assets/big-3c4d.js")
        .header("accept-encoding", "br")
        .body(Body::empty())
        .unwrap();
    let response = router(options(WebSource::Folder(dir.path().into())))
        .oneshot(request)
        .await
        .unwrap();
    assert_eq!(header_of(&response, "content-encoding"), "br");
    let sent = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let size_at = |quality: i32| {
        let parameters = brotli::enc::BrotliEncoderParams {
            quality,
            ..Default::default()
        };
        let mut output = Vec::new();
        brotli::BrotliCompress(&mut script.as_bytes(), &mut output, &parameters).unwrap();
        output.len()
    };
    let quality = compression_quality().unwrap();
    assert_eq!(sent.len(), size_at(quality));
    // The qualities differ here, so the test would notice a change of either side.
    assert_ne!(size_at(quality), size_at(11));
}

/// An answer of `size` bytes with this content type and extras, through the compression of
/// the server, asked for with every kind of compression the server knows.
async fn compressed_as(
    kind: &'static str,
    size: u64,
    secret: bool,
    cookie: bool,
) -> Option<String> {
    let size = usize::try_from(size).unwrap();
    let app = Router::new()
        .route(
            "/answer",
            get(move || async move {
                let mut response = Response::new(Body::from("a".repeat(size)));
                let headers = response.headers_mut();
                headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(kind));
                if cookie {
                    headers.insert(header::SET_COOKIE, HeaderValue::from_static("a=b"));
                }
                if secret {
                    response.extensions_mut().insert(SecretAnswer);
                }
                response
            }),
        )
        .layer(compression_layer());
    let request = Request::get("/answer")
        .header("accept-encoding", "br, gzip")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    response
        .headers()
        .get(header::CONTENT_ENCODING)
        .map(|value| value.to_str().unwrap().to_owned())
}

#[tokio::test]
async fn sec_11_json_answers_above_1_kib_are_compressed_and_smaller_ones_are_not() {
    for kind in ["application/json", "application/problem+json"] {
        assert_eq!(
            compressed_as(kind, JSON_COMPRESS_ABOVE_BYTES + 1, false, false)
                .await
                .as_deref(),
            Some("br"),
            "{kind}"
        );
        // Exactly 1 KiB is not above it.
        for size in [0, 100, JSON_COMPRESS_ABOVE_BYTES] {
            assert_eq!(
                compressed_as(kind, size, false, false).await,
                None,
                "{kind} {size}"
            );
        }
    }
    // The charset does not change the kind.
    assert!(
        compressed_as("application/json; charset=utf-8", 5000, false, false)
            .await
            .is_some()
    );
}

#[tokio::test]
async fn sec_11_an_answer_that_hands_out_a_secret_is_never_compressed_however_big() {
    for kind in ["application/json", "application/problem+json", "text/html"] {
        assert_eq!(
            compressed_as(kind, 100_000, true, false).await,
            None,
            "{kind}"
        );
        assert_eq!(
            compressed_as(kind, 100_000, false, true).await,
            None,
            "{kind}"
        );
        assert_eq!(
            compressed_as(kind, 100_000, true, true).await,
            None,
            "{kind}"
        );
    }
}

#[tokio::test]
async fn sec_11_files_keep_their_rules() {
    // Text from 256 bytes on, other JSON and files that are compressed already never.
    assert!(
        compressed_as(
            "text/html; charset=utf-8",
            COMPRESS_FROM_BYTES,
            false,
            false
        )
        .await
        .is_some()
    );
    assert_eq!(
        compressed_as("text/html", COMPRESS_FROM_BYTES - 1, false, false).await,
        None
    );
    assert!(
        compressed_as("text/javascript", 5000, false, false)
            .await
            .is_some()
    );
    for kind in ["application/manifest+json", "image/png", "font/woff2"] {
        assert_eq!(
            compressed_as(kind, 100_000, false, false).await,
            None,
            "{kind}"
        );
    }
}

#[tokio::test]
async fn dist_01_percent_escapes_in_file_names_are_decoded() {
    let dir = app_folder();
    let response = get_from(WebSource::Folder(dir.path().into()), "/my%20file.txt").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body(response).await, "spaced");
}

#[tokio::test]
async fn dist_01_page_addresses_fall_back_to_the_start_page() {
    let dir = app_folder();
    let response = get_from(WebSource::Folder(dir.path().into()), "/issues/BUG-1").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body(response).await, "<h1>Bugaboo</h1>");
}

#[tokio::test]
async fn dist_01_assets_health_and_api_never_fall_back_to_the_start_page() {
    let dir = app_folder();
    for path in ["/assets/missing", "/assets", "/health", "/missing.js"] {
        let response = get_from(WebSource::Folder(dir.path().into()), path).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
        assert_eq!(body(response).await, "Not found", "{path}");
    }
    // Below /api the answer is problem+json, never the start page or plain text.
    for path in ["/api/issues", "/api"] {
        let response = get_from(WebSource::Folder(dir.path().into()), path).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
        assert_eq!(
            header_of(&response, "content-type"),
            "application/problem+json",
            "{path}"
        );
        assert!(
            body(response).await.contains("resource.not_found"),
            "{path}"
        );
    }
}

#[tokio::test]
async fn sec_11_unsafe_file_paths_are_not_found() {
    let dir = app_folder();
    for path in [
        "/assets/../index.html",
        "/..%5Cx.js",
        "/%2e%2e/x.js",
        "/%2e%2e%2findex.html",
        "/%00.js",
        "/aux.js",
        "/assets/NUL.txt",
        "/%zz.js",
    ] {
        let response = get_from(WebSource::Folder(dir.path().into()), path).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
    }
}

#[tokio::test]
async fn sec_11_device_names_are_never_opened_as_files() {
    let dir = app_folder();
    for path in ["/CON", "/com1", "/nul", "/%2e%2e/x"] {
        let response = get_from(WebSource::Folder(dir.path().into()), path).await;
        // Page-like addresses only ever get the start page.
        assert_eq!(body(response).await, "<h1>Bugaboo</h1>", "{path}");
    }
}

#[tokio::test]
async fn dist_01_a_folder_is_not_a_file() {
    let dir = app_folder();
    std::fs::create_dir(dir.path().join("folder.d")).unwrap();
    let response = get_from(WebSource::Folder(dir.path().into()), "/folder.d").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn dist_01_only_a_missing_start_page_says_the_app_is_not_built() {
    let web = WebSource::Folder("no/such/folder".into());
    assert!(!web.has_index().await);
    let response = get_from(web, "/").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        body(response).await,
        Message::from(message::SERVER_WEB_APP_MISSING).render()
    );

    let dir = app_folder();
    let response = get_from(WebSource::Folder(dir.path().into()), "/nope.js").await;
    assert_eq!(body(response).await, "Not found");
}
