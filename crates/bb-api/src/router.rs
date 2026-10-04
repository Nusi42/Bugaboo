//! Routes: health and the browser app.

use std::future::Future;
use std::io;
use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::http::{HeaderValue, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use tokio::net::TcpListener;

use crate::headers;
use crate::web::{INDEX, WEB_APP_MISSING, WebSource};

const HEALTH_LIVE_BODY: &str = r#"{"status":"ok"}"#;
/// Files Vite names by their content hash can be cached forever.
const IMMUTABLE_PREFIX: &str = "assets/";
const CACHE_IMMUTABLE: &str = "public, max-age=31536000, immutable";
const CACHE_REVALIDATE: &str = "no-cache";

/// The whole HTTP surface, security headers included.
pub fn router(web: WebSource) -> Router {
    let app = Router::new()
        .route("/health/live", get(health_live))
        .fallback(get(app_file))
        .with_state(Arc::new(web));
    headers::apply(app)
}

/// Serves `router(web)` on `listener` until `shutdown` completes.
///
/// # Errors
/// Returns the I/O error that stopped the server.
pub async fn serve(
    listener: TcpListener,
    web: WebSource,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> io::Result<()> {
    axum::serve(listener, router(web))
        .with_graceful_shutdown(shutdown)
        .await
}

async fn health_live() -> Response {
    (
        [(header::CONTENT_TYPE, "application/json")],
        HEALTH_LIVE_BODY,
    )
        .into_response()
}

/// A file of the app; page addresses (no file extension) fall back to the start page.
async fn app_file(State(web): State<Arc<WebSource>>, uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let wanted = if path.is_empty() { INDEX } else { path };
    let is_page = !wanted.rsplit('/').next().unwrap_or_default().contains('.');
    let found = match web.load(wanted).await {
        Some(asset) => Some((wanted, asset)),
        None if is_page => web.load(INDEX).await.map(|asset| (INDEX, asset)),
        None => None,
    };
    let Some((name, asset)) = found else {
        return (StatusCode::NOT_FOUND, WEB_APP_MISSING).into_response();
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

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt as _;

    use super::*;
    use crate::headers::SECURITY_HEADERS;

    fn app_folder() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.html"), "<h1>Bugaboo</h1>").unwrap();
        std::fs::create_dir(dir.path().join("assets")).unwrap();
        std::fs::write(dir.path().join("assets/app-1a2b.js"), "export {};").unwrap();
        std::fs::write(dir.path().join("robots.txt"), "x").unwrap();
        dir
    }

    async fn get_from(web: WebSource, path: &str) -> Response {
        let request = Request::get(path).body(Body::empty()).unwrap();
        router(web).oneshot(request).await.unwrap()
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
    async fn sec_11_every_response_carries_every_security_header() {
        let dir = app_folder();
        for path in [
            "/health/live",
            "/",
            "/some/page",
            "/assets/app-1a2b.js",
            "/missing.js",
            "/assets/../index.html",
        ] {
            let response = get_from(WebSource::Folder(dir.path().into()), path).await;
            for (name, value) in SECURITY_HEADERS {
                assert_eq!(header_of(&response, name), value, "{name} on {path}");
            }
        }
        let missing = get_from(WebSource::Folder("no/such/folder".into()), "/").await;
        for (name, value) in SECURITY_HEADERS {
            assert_eq!(header_of(&missing, name), value, "{name} without app");
        }
    }

    #[test]
    fn sec_11_csp_forbids_inline_code_and_framing() {
        let csp = SECURITY_HEADERS[0].1;
        for needed in [
            "default-src 'self'",
            "script-src 'self'",
            "style-src 'self'",
            "frame-ancestors 'none'",
            "base-uri 'none'",
            "require-trusted-types-for 'script'",
        ] {
            assert!(csp.contains(needed), "{needed}");
        }
        assert!(!csp.contains("unsafe-inline") && !csp.contains("unsafe-eval"));
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
    async fn dist_01_page_addresses_fall_back_to_the_start_page() {
        let dir = app_folder();
        let response = get_from(WebSource::Folder(dir.path().into()), "/issues/BUG-1").await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body(response).await, "<h1>Bugaboo</h1>");
    }

    #[tokio::test]
    async fn dist_01_missing_files_are_not_found_and_cannot_escape_the_folder() {
        let dir = app_folder();
        for path in ["/assets/missing.js", "/assets/../x.js", "/..%5Cx.js"] {
            let response = get_from(WebSource::Folder(dir.path().into()), path).await;
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
        }
    }

    #[tokio::test]
    async fn dist_01_missing_app_folder_says_what_to_do() {
        let web = WebSource::Folder("no/such/folder".into());
        assert!(!web.has_index().await);
        let response = get_from(web, "/").await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(body(response).await, WEB_APP_MISSING);
    }
}
