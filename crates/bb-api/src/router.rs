//! Routes: health and the browser app.

use std::future::Future;
use std::io;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, Uri, header};
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Extension, Router};
use bb_core::message::{self, Message};
use tower_http::compression::predicate::{Predicate as _, SizeAbove};
use tower_http::compression::{CompressionLayer, DefaultPredicate};

use crate::api::{AuthHandle, ProbeHandle};
use crate::limits::{Limits, LimitsConfig};
use crate::{api, headers};
use hyper_util::rt::{TokioExecutor, TokioIo, TokioTimer};
use hyper_util::server::conn::auto::Builder;
use hyper_util::service::TowerToHyperService;
use tokio::sync::watch;
use tokio::task::JoinSet;

use crate::listener::{Accepted, ClientAddr, Connection, Incoming};
use crate::logging::{ConnectionTls, LogContext, log_request};
use crate::web::{INDEX, WebSource, decode_path};

const HEALTH_LIVE_BODY: &str = r#"{"status":"ok"}"#;
/// Files Vite names by their content hash can be cached forever.
const IMMUTABLE_PREFIX: &str = "assets/";
const CACHE_IMMUTABLE: &str = "public, max-age=31536000, immutable";
const CACHE_REVALIDATE: &str = "no-cache";
/// Addresses below these are never browser pages: when missing they are plain 404s.
const NO_PAGE_FALLBACK: [&str; 3] = ["assets", "health", "api"];
/// Smaller answers are not worth compressing.
const COMPRESS_FROM_BYTES: u64 = 256;
/// Running requests get this long to finish when the server stops.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(10);

/// What the server needs to know to answer requests.
#[derive(Clone, Debug)]
pub struct Options {
    /// Where the browser app comes from.
    pub web: WebSource,
    /// HSTS lifetime in seconds when browsers reach us over HTTPS (here or at a trusted proxy);
    /// `None` for plain HTTP. See [`crate::HSTS_MAX_AGE_SELF_SIGNED`].
    pub hsts_max_age: Option<u64>,
    /// How long a client may take to send the headers of a request. This also ends idle
    /// connections that wait for a next request.
    pub header_timeout: Duration,
    /// The data folder, for `/health/ready`.
    pub data_folder: PathBuf,
    /// Proxies whose `X-Forwarded-For` header may be believed for the log.
    pub trusted_proxies: Vec<IpAddr>,
    /// How often one client may call the API (SEC-14).
    pub limits: LimitsConfig,
    /// The use cases of signing in; `AuthHandle::none()` when there is no database yet.
    pub auth: AuthHandle,
    /// How the server measures itself, for the status page and `/health/ready`;
    /// `ProbeHandle::none()` when there is no database yet.
    pub probe: ProbeHandle,
}

#[derive(Clone)]
struct AppState {
    web: Arc<WebSource>,
    data_folder: Arc<PathBuf>,
    auth: AuthHandle,
    probe: ProbeHandle,
}

impl axum::extract::FromRef<AppState> for AuthHandle {
    fn from_ref(state: &AppState) -> Self {
        state.auth.clone()
    }
}

impl axum::extract::FromRef<AppState> for ProbeHandle {
    fn from_ref(state: &AppState) -> Self {
        state.probe.clone()
    }
}

/// The whole HTTP surface, security headers included.
pub fn router(options: Options) -> Router {
    let hsts_max_age = options.hsts_max_age;
    let limits = Limits::new(options.limits);
    let state = AppState {
        web: Arc::new(options.web),
        data_folder: Arc::new(options.data_folder),
        auth: options.auth,
        probe: options.probe,
    };
    let auth = state.auth.clone();
    let compress = DefaultPredicate::new()
        .and(SizeAbove::new(COMPRESS_FROM_BYTES))
        .and(is_text);
    let app = Router::new()
        .route("/health/live", get(health_live))
        .route("/health/ready", get(health_ready))
        .merge(api::routes(&limits).split_for_parts().0)
        .fallback(fallback)
        .method_not_allowed_fallback(api::method_not_allowed)
        .with_state(state);
    let app = api::rules(app, limits, auth)
        .layer(CompressionLayer::new().compress_when(compress))
        .layer(from_fn_with_state(
            LogContext::new(&options.trusted_proxies),
            log_request,
        ));
    headers::apply(app, hsts_max_age)
}

/// Serves `router(options)` on `incoming` until `shutdown` completes, then lets running
/// requests finish (at most [`SHUTDOWN_GRACE`]).
///
/// # Errors
/// Never fails today; the result type is kept for errors of the listener.
pub async fn serve(
    mut incoming: Incoming,
    options: Options,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> io::Result<()> {
    let header_timeout = options.header_timeout;
    let app = router(options);
    let (stop, stopped) = watch::channel(false);
    let mut tasks = JoinSet::new();
    tokio::pin!(shutdown);
    loop {
        let accepted = tokio::select! {
            () = &mut shutdown => break,
            accepted = incoming.accept() => accepted,
        };
        let Some(accepted) = accepted else { break };
        let tls = matches!(accepted.io, Connection::Tls(_));
        let service = TowerToHyperService::new(
            app.clone()
                .layer(Extension(ConnectInfo(ClientAddr(accepted.peer))))
                .layer(Extension(ConnectionTls(tls))),
        );
        let mut stopped = stopped.clone();
        tasks.spawn(async move {
            let Accepted { io, permit, .. } = accepted;
            let mut builder = Builder::new(TokioExecutor::new());
            builder
                .http1()
                .timer(TokioTimer::new())
                .header_read_timeout(header_timeout);
            builder
                .http2()
                .timer(TokioTimer::new())
                .keep_alive_interval(header_timeout)
                .keep_alive_timeout(header_timeout);
            let connection = builder.serve_connection_with_upgrades(TokioIo::new(io), service);
            tokio::pin!(connection);
            tokio::select! {
                _ = connection.as_mut() => {}
                _ = stopped.changed() => {
                    connection.as_mut().graceful_shutdown();
                    let _ = connection.await;
                }
            }
            drop(permit);
        });
        // Finished connections are collected as we go.
        while tasks.try_join_next().is_some() {}
    }
    let _ = stop.send(true);
    let _ = tokio::time::timeout(SHUTDOWN_GRACE, async {
        while tasks.join_next().await.is_some() {}
    })
    .await;
    Ok(())
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
            // JSON (API answers, problem+json) is never compressed: a compressed answer that
            // mixes a secret with text the attacker chooses can leak the secret (BREACH).
            !kind.contains("json")
                && ["text/", "javascript", "xml", "wasm"]
                    .iter()
                    .any(|part| kind.contains(part))
        })
}

async fn health_live() -> Response {
    json(StatusCode::OK, HEALTH_LIVE_BODY)
}

/// Ready when the data folder can be read and, once the server has a database (M1-F19), the
/// database answers. The body names each check, never a reason: it is open to everyone.
async fn health_ready(State(state): State<AppState>) -> Response {
    let folder = tokio::fs::read_dir(state.data_folder.as_path())
        .await
        .is_ok();
    let database = match state.probe.probe() {
        Some(probe) => Some(probe.database_ok().await),
        None => None,
    };
    let word = |ok: bool| if ok { "ok" } else { "failed" };
    let ready = folder && database != Some(false);
    let mut checks = format!(r#""data_folder":"{}""#, word(folder));
    if let Some(ok) = database {
        checks.push_str(&format!(r#","database":"{}""#, word(ok)));
    }
    let status = if ready { "ok" } else { "unavailable" };
    json(
        if ready {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        },
        &format!(r#"{{"status":"{status}","checks":{{{checks}}}}}"#),
    )
}

fn json(status: StatusCode, body: &str) -> Response {
    (
        status,
        [
            (header::CONTENT_TYPE, "application/json"),
            // A health answer must always be fresh.
            (header::CACHE_CONTROL, "no-store"),
        ],
        body.to_owned(),
    )
        .into_response()
}

/// Everything that is not a route: files of the app for GET and HEAD, problem+json 404 below
/// `/api` for every method (M0-F07), 405 for other methods elsewhere.
async fn fallback(State(state): State<AppState>, method: Method, uri: Uri) -> Response {
    let web = &state.web;
    if api::is_api(uri.path()) {
        return api::not_found();
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
    let key = if web.has_index().await {
        message::SERVER_NOT_FOUND
    } else {
        message::SERVER_WEB_APP_MISSING
    };
    let body = Message::from(key).render();
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
            hsts_max_age: None,
            header_timeout: Duration::from_secs(30),
            data_folder: std::env::temp_dir(),
            trusted_proxies: vec![],
            limits: LimitsConfig::default(),
            auth: AuthHandle::none(),
            probe: ProbeHandle::none(),
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
}
