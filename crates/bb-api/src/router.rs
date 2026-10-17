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
use tower_http::compression::predicate::{Predicate, SizeAbove};
use tower_http::compression::{CompressionLayer, CompressionLevel, DefaultPredicate};

use crate::api::{AuthHandle, ProbeHandle};
use crate::cookies::sets_a_cookie;
use crate::etag::SecretAnswer;
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
/// Smaller files are not worth compressing.
const COMPRESS_FROM_BYTES: u64 = 256;
/// A JSON answer is compressed only when it is bigger than this (1 KiB, M2-Q2): below it the
/// saving is small. The one definition of the threshold.
const JSON_COMPRESS_ABOVE_BYTES: u64 = 1024;
/// The kinds of JSON answer that may be compressed. Another kind with `json` in its name is not
/// an API answer and stays as it is.
const JSON_TYPES: [&str; 2] = ["application/json", "application/problem+json"];
/// The compression quality, written once in a file that the budget check of the first page
/// (PERF-BUNDLE) reads too, so the size it measures is the size this server sends.
const COMPRESSION_SETTINGS: &str = include_str!("../../../tools/dev/compression.json");
/// How long a client may take to send the headers of a request when nothing else is set.
pub const DEFAULT_HEADER_TIMEOUT: Duration = Duration::from_secs(30);
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

#[cfg(feature = "testing")]
impl Options {
    /// Options for a router in a test: no browser app, plain HTTP, no trusted proxy, the default
    /// limits and no database behind the sign-in routes. A test changes what it needs with
    /// `Options { web, ..Options::for_tests() }`.
    #[must_use]
    pub fn for_tests() -> Self {
        Self {
            web: WebSource::Folder("no/such/folder".into()),
            hsts_max_age: None,
            header_timeout: DEFAULT_HEADER_TIMEOUT,
            data_folder: std::env::temp_dir(),
            trusted_proxies: vec![],
            limits: LimitsConfig::default(),
            auth: AuthHandle::none(),
            probe: ProbeHandle::none(),
        }
    }

    /// Writes a small browser app into `folder`: a start page and one script with a hashed name.
    ///
    /// # Errors
    /// Fails when a file cannot be written.
    pub fn write_test_app(folder: &std::path::Path) -> io::Result<()> {
        std::fs::write(folder.join("index.html"), "<h1>Bugaboo</h1>")?;
        std::fs::create_dir(folder.join("assets"))?;
        std::fs::write(folder.join("assets/app-1a2b.js"), "export {};")
    }
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
    let header_auth = auth.clone();
    let app = Router::new()
        .route("/health/live", get(health_live))
        .route("/health/ready", get(health_ready))
        .merge(api::routes(&limits).split_for_parts().0)
        .fallback(fallback)
        .method_not_allowed_fallback(api::method_not_allowed)
        .with_state(state);
    let app = api::rules(app, limits, auth)
        .layer(compression_layer())
        .layer(from_fn_with_state(
            LogContext::new(&options.trusted_proxies),
            log_request,
        ));
    headers::apply(app, hsts_max_age, header_auth)
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

/// The quality from the settings file; an unreadable file (a test keeps it readable) leaves the
/// default of the library.
fn compression_level() -> CompressionLevel {
    compression_quality().map_or(CompressionLevel::Default, CompressionLevel::Precise)
}

fn compression_quality() -> Option<i32> {
    serde_json::from_str::<serde_json::Value>(COMPRESSION_SETTINGS)
        .ok()?
        .get("quality")?
        .as_i64()
        .and_then(|quality| i32::try_from(quality).ok())
}

/// The compression of answers, outside the rules that mark an answer a secret (`crate::etag`).
pub(crate) fn compression_layer() -> CompressionLayer<impl Predicate> {
    CompressionLayer::new()
        .quality(compression_level())
        .compress_when(
            DefaultPredicate::new()
                .and(is_compressible)
                .and(WorthTheTrouble),
        )
}

/// What a response is, for compression.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Content {
    /// An API answer: compressed above [`JSON_COMPRESS_ABOVE_BYTES`], never when it holds a
    /// secret.
    Json,
    /// Text of the browser app, compressed from [`COMPRESS_FROM_BYTES`].
    Text,
    /// Images, fonts, archives (already compressed) and everything else.
    Other,
}

fn content_of(headers: &HeaderMap) -> Content {
    let Some(kind) = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
    else {
        return Content::Other;
    };
    let essence = kind
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if JSON_TYPES.contains(&essence.as_str()) {
        Content::Json
    } else if essence.contains("json") {
        Content::Other
    } else if ["text/", "javascript", "xml", "wasm"]
        .iter()
        .any(|part| essence.contains(part))
    {
        Content::Text
    } else {
        Content::Other
    }
}

/// Which answers may be compressed at all. A compressed answer that mixes a secret with text the
/// attacker chooses can leak the secret (BREACH), so an answer that hands out a secret (marked
/// [`SecretAnswer`]) or sets a cookie is never compressed, whatever its type and size.
fn is_compressible(
    _status: StatusCode,
    _version: axum::http::Version,
    headers: &HeaderMap,
    extensions: &axum::http::Extensions,
) -> bool {
    extensions.get::<SecretAnswer>().is_none()
        && !sets_a_cookie(headers)
        && content_of(headers) != Content::Other
}

/// Answers below their size are not compressed: JSON from [`JSON_COMPRESS_ABOVE_BYTES`], files
/// from [`COMPRESS_FROM_BYTES`].
#[derive(Clone, Copy)]
struct WorthTheTrouble;

impl Predicate for WorthTheTrouble {
    fn should_compress<B>(&self, response: &Response<B>) -> bool
    where
        B: axum::body::HttpBody,
    {
        let from = if content_of(response.headers()) == Content::Json {
            // `SizeAbove` compresses from its number on; "above" is one more.
            JSON_COMPRESS_ABOVE_BYTES + 1
        } else {
            COMPRESS_FROM_BYTES
        };
        SizeAbove::new(from).should_compress(response)
    }
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
    let checks = match database {
        Some(ok) => format!(
            r#""data_folder":"{}","database":"{}""#,
            word(folder),
            word(ok)
        ),
        None => format!(r#""data_folder":"{}""#, word(folder)),
    };
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
mod tests;
