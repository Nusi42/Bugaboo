//! Routes: health and the browser app.

use std::future::Future;
use std::io;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, Uri, header};
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use tower_http::compression::predicate::{Predicate as _, SizeAbove};
use tower_http::compression::{CompressionLayer, DefaultPredicate};

use crate::headers;
use crate::listener::{ClientAddr, Incoming};
use crate::logging::{LogContext, log_request};
use crate::web::{INDEX, WEB_APP_MISSING, WebSource, decode_path};

const HEALTH_LIVE_BODY: &str = r#"{"status":"ok"}"#;
const HEALTH_READY_BODY: &str = r#"{"status":"ok","checks":{"data_folder":"ok"}}"#;
const HEALTH_NOT_READY_BODY: &str = r#"{"status":"unavailable","checks":{"data_folder":"failed"}}"#;
const NOT_FOUND: &str = "Not found";
/// Files Vite names by their content hash can be cached forever.
const IMMUTABLE_PREFIX: &str = "assets/";
const CACHE_IMMUTABLE: &str = "public, max-age=31536000, immutable";
const CACHE_REVALIDATE: &str = "no-cache";
/// Addresses below these are never browser pages: when missing they are plain 404s.
const NO_PAGE_FALLBACK: [&str; 3] = ["assets", "health", "api"];
/// The API's address space: an unknown address answers 404 to every method (M0-F07).
const API_PREFIX: &str = "api";
/// Smaller answers are not worth compressing.
const COMPRESS_FROM_BYTES: u64 = 256;

/// What the server needs to know to answer requests.
#[derive(Clone, Debug)]
pub struct Options {
    /// Where the browser app comes from.
    pub web: WebSource,
    /// Whether browsers reach us over HTTPS (here or at a trusted proxy): adds HSTS.
    pub https: bool,
    /// The data folder, for `/health/ready`.
    pub data_folder: PathBuf,
    /// Proxies whose `X-Forwarded-For` header may be believed for the log.
    pub trusted_proxies: Vec<IpAddr>,
}

#[derive(Clone)]
struct AppState {
    web: Arc<WebSource>,
    data_folder: Arc<PathBuf>,
}

/// The whole HTTP surface, security headers included.
pub fn router(options: Options) -> Router {
    let state = AppState {
        web: Arc::new(options.web),
        data_folder: Arc::new(options.data_folder),
    };
    let compress = DefaultPredicate::new()
        .and(SizeAbove::new(COMPRESS_FROM_BYTES))
        .and(is_text);
    let app = Router::new()
        .route("/health/live", get(health_live))
        .route("/health/ready", get(health_ready))
        .fallback(fallback)
        .with_state(state)
        .layer(CompressionLayer::new().compress_when(compress))
        .layer(from_fn_with_state(
            LogContext::new(options.trusted_proxies),
            log_request,
        ));
    headers::apply(app, options.https)
}

/// Serves `router(options)` on `incoming` until `shutdown` completes.
///
/// # Errors
/// Returns the I/O error that stopped the server.
pub async fn serve(
    incoming: Incoming,
    options: Options,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> io::Result<()> {
    axum::serve(
        incoming,
        router(options).into_make_service_with_connect_info::<ClientAddr>(),
    )
    .with_graceful_shutdown(shutdown)
    .await
}

/// Text compresses well; images, fonts and archives are already compressed.
fn is_text(
    _status: StatusCode,
    _version: axum::http::Version,
    headers: &HeaderMap,
    _extensions: &axum::http::Extensions,
) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|kind| {
            ["text/", "javascript", "json", "xml", "wasm"]
                .iter()
                .any(|part| kind.contains(part))
        })
}

async fn health_live() -> Response {
    json(StatusCode::OK, HEALTH_LIVE_BODY)
}

/// Ready when the data folder can be read; the database joins in with the store.
async fn health_ready(State(state): State<AppState>) -> Response {
    let usable = tokio::fs::read_dir(state.data_folder.as_path())
        .await
        .is_ok();
    if usable {
        json(StatusCode::OK, HEALTH_READY_BODY)
    } else {
        json(StatusCode::SERVICE_UNAVAILABLE, HEALTH_NOT_READY_BODY)
    }
}

fn json(status: StatusCode, body: &'static str) -> Response {
    (status, [(header::CONTENT_TYPE, "application/json")], body).into_response()
}

/// Everything that is not a route: files of the app for GET and HEAD, 404 below `/api` for every
/// method, 405 for other methods elsewhere.
async fn fallback(State(state): State<AppState>, method: Method, uri: Uri) -> Response {
    let web = &state.web;
    if decode_path(uri.path()).is_some_and(|path| is_api(&path)) {
        return not_found(web).await;
    }
    if method == Method::GET || method == Method::HEAD {
        return app_file(web, &uri).await;
    }
    (
        StatusCode::METHOD_NOT_ALLOWED,
        [(header::ALLOW, "GET, HEAD")],
    )
        .into_response()
}

fn is_api(path: &str) -> bool {
    path.trim_start_matches('/')
        .split('/')
        .next()
        .is_some_and(|first| first == API_PREFIX)
}

/// A file of the app; page addresses fall back to the start page.
async fn app_file(web: &WebSource, uri: &Uri) -> Response {
    let Some(path) = decode_path(uri.path()) else {
        return not_found(web).await;
    };
    let path = path.trim_start_matches('/');
    let wanted = if path.is_empty() { INDEX } else { path };
    let found = match web.load(wanted).await {
        Some(asset) => Some((wanted, asset)),
        None if is_page(wanted) => web.load(INDEX).await.map(|asset| (INDEX, asset)),
        None => None,
    };
    let Some((name, asset)) = found else {
        return not_found(web).await;
    };
    let cache = if name.starts_with(IMMUTABLE_PREFIX) {
        CACHE_IMMUTABLE
    } else {
        CACHE_REVALIDATE
    };
    let mut response = asset.bytes.into_response();
    let headers = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(&asset.content_type) {
        headers.insert(header::CONTENT_TYPE, value);
    }
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static(cache));
    response
}

/// A page address has no file extension and is not below a reserved prefix.
fn is_page(path: &str) -> bool {
    let first = path.split('/').next().unwrap_or_default();
    let last = path.rsplit('/').next().unwrap_or_default();
    !last.contains('.') && !NO_PAGE_FALLBACK.contains(&first)
}

/// A plain 404; only a missing start page says that the app has not been built.
async fn not_found(web: &WebSource) -> Response {
    let body = if web.has_index().await {
        NOT_FOUND
    } else {
        WEB_APP_MISSING
    };
    (StatusCode::NOT_FOUND, body).into_response()
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Method, Request};
    use tower::ServiceExt as _;

    use super::*;

    fn app_folder() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.html"), "<h1>Bugaboo</h1>").unwrap();
        std::fs::create_dir(dir.path().join("assets")).unwrap();
        std::fs::write(dir.path().join("assets/app-1a2b.js"), "export {};").unwrap();
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
            https: false,
            data_folder: std::env::temp_dir(),
            trusted_proxies: vec![],
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
            for path in ["/api/x", "/api", "/api/", "/%61pi/x"] {
                let request = Request::builder()
                    .method(method.clone())
                    .uri(path)
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
        for path in [
            "/assets/missing",
            "/assets",
            "/health",
            "/api/issues",
            "/api",
            "/missing.js",
        ] {
            let response = get_from(WebSource::Folder(dir.path().into()), path).await;
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
            assert_eq!(body(response).await, "Not found", "{path}");
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
        assert_eq!(body(response).await, WEB_APP_MISSING);

        let dir = app_folder();
        let response = get_from(WebSource::Folder(dir.path().into()), "/nope.js").await;
        assert_eq!(body(response).await, "Not found");
    }
}
