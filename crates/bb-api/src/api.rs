//! The API below `/api/v1`: its routes, and the rules every API request goes through.
//!
//! Each route is described once, with `#[utoipa::path]` on its handler; [`routes`] collects
//! them, and the router and the OpenAPI document (`crate::openapi`) both come from that list, so
//! they cannot disagree (ADR-0008). Changes that break clients need a new version prefix and an
//! ADR.
//!
//! Rules for all API requests, applied by [`rules`]:
//! - the client network is counted against the rate limit (`crate::limits`);
//! - a request has [`TIME_LIMIT`] to finish, which also ends a body that arrives too slowly
//!   (M1-F06), and a body may be [`BODY_LIMIT_BYTES`] big;
//! - a request that changes something must come from one of Bugaboo's own pages ([`origin_check`])
//!   and, when it carries a session, with the token of that session (`auth::csrf_check`); a
//!   request that comes with an access key (`Authorization: Bearer bb_…`) and no session cookie is
//!   from a script: it has no cookie for a forged request to ride on, and a page of another
//!   website cannot send that header (there is no CORS), so it needs neither;
//! - a request that comes with an access key counts against the key as well (`limits`);
//! - every answer has its cache rule, and every read an `ETag` and a `304` (`crate::etag`);
//! - every error answer is problem+json with the request ID (`crate::errors`).

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::extract::{DefaultBodyLimit, Request};
use axum::http::{HeaderMap, Method, header};
use axum::middleware::{Next, from_fn, from_fn_with_state};
use axum::response::{IntoResponse as _, Response};
use bb_core::ErrorCode;
use bb_core::own_fields::Edit;
use serde::{Deserialize, Deserializer, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::errors::{self, ApiError};
use crate::etag;
use crate::limits::{self, Limits};
use crate::logging::Scheme;
use crate::openapi;
use crate::web::decode_path;

mod access_bodies;
mod access_keys;
mod admin;
mod assignments;
mod auth;
mod auth_factors;
mod auth_people;
mod auth_sign_in;
mod check_access;
mod comments;
mod editing;
mod field_permissions;
mod fields;
mod follow;
mod groups;
mod history;
mod issue_types;
mod issues;
mod layouts;
mod many_changes;
mod notifications;
mod own_access;
mod people;
mod people_visibility;
mod previews;
mod projects;
mod projects_move;
mod relations;
mod roles;
mod second_step_gap;
mod settings;
mod tags;

pub use admin::ProbeHandle;
pub use auth::AuthHandle;

/// The first part of every API address.
pub(crate) const API_PREFIX: &str = "api";

/// The version prefix of the routes below (ADR-0008).
pub const VERSION_PATH: &str = "/api/v1";

/// A request that has not finished after this long is stopped. Routes that need longer (file
/// uploads, exports) get their own, larger limit when they exist.
pub const TIME_LIMIT: Duration = Duration::from_secs(30);

/// The biggest request body the API accepts, unless a route sets its own limit.
pub const BODY_LIMIT_BYTES: usize = 1024 * 1024;

/// What `GET /api/v1/status` answers: the server is up.
#[derive(Serialize, ToSchema)]
pub(crate) struct ServerStatus {
    /// Always `ok`; an answer at all means the server is running.
    status: StatusValue,
}

/// The only value a status can have today.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
enum StatusValue {
    Ok,
}

/// Whether the server is up. Open to everyone, and the example route of the API skeleton.
#[utoipa::path(
    get,
    path = "/api/v1/status",
    operation_id = "getStatus",
    tag = "server",
    responses((status = 200, description = "The server is running.", body = ServerStatus))
)]
async fn get_status() -> axum::Json<ServerStatus> {
    axum::Json(ServerStatus {
        status: StatusValue::Ok,
    })
}

/// The description of this API (OpenAPI 3.1), for tools and for the generated browser client.
#[utoipa::path(
    get,
    path = "/api/v1/openapi.json",
    operation_id = "getOpenApi",
    tag = "server",
    responses((
        status = 200,
        description = "The OpenAPI document of this API.",
        content_type = "application/json",
        body = Object
    ))
)]
async fn get_openapi() -> Response {
    (
        [(header::CONTENT_TYPE, "application/json")],
        openapi::json(),
    )
        .into_response()
}

/// Every route of the API with its description. `limits` holds the per-network limit for the
/// routes that try a password, a code or a secret.
pub(crate) fn routes<S>(limits: &Arc<Limits>) -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
    ProbeHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::with_openapi(openapi::base())
        .routes(routes!(get_status))
        .routes(routes!(get_openapi))
        .merge(auth::routes(limits))
        .merge(access_keys::routes())
        .merge(second_step_gap::routes())
        .merge(admin::routes())
        .merge(settings::routes())
        .merge(projects::routes())
        .merge(fields::routes())
        .merge(issue_types::routes())
        .merge(layouts::routes())
        .merge(issues::routes())
        .merge(relations::routes())
        .merge(tags::routes())
        .merge(many_changes::routes())
        .merge(comments::routes())
        .merge(history::routes())
        .merge(follow::routes())
        .merge(notifications::routes())
        .merge(people::routes())
        .merge(groups::routes())
        .merge(roles::routes())
        .merge(own_access::routes())
        .merge(assignments::routes())
        .merge(field_permissions::routes())
        .merge(people_visibility::routes())
        .merge(check_access::routes())
        .merge(editing::routes())
}

/// Whether the address belongs to the API. Escapes are decoded first; an address with a
/// malformed escape is judged as written, and `.` and `..` parts are ignored, so no spelling
/// of an API address ends up as a page or a 405.
pub(crate) fn is_api(path: &str) -> bool {
    let decoded = decode_path(path).unwrap_or_else(|| path.to_owned());
    decoded
        .split('/')
        .find(|part| !part.is_empty() && *part != "." && *part != "..")
        .is_some_and(|first| first.eq_ignore_ascii_case(API_PREFIX))
}

/// The access key a request presents: `Authorization: Bearer bb_…`. Anything else in that header
/// is no key and is left alone, so a page's own requests never meet this.
pub(crate) fn presented_key(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|key| bb_auth::looks_like_access_key(key))
}

/// Whether the request comes from a script with an access key: it presents one and has no
/// session cookie. A request with a cookie is judged as a request of a browser, key or not.
pub(crate) fn is_key_request(headers: &HeaderMap) -> bool {
    presented_key(headers).is_some() && crate::cookies::Cookie::Session.read(headers).is_none()
}

/// Puts the rules for API requests around `router`.
///
/// The layers apply to requests below `/api` only; pages, files and health checks pass through
/// untouched.
pub(crate) fn rules(router: Router, limits: Arc<Limits>, auth: AuthHandle) -> Router {
    router
        // Innermost: the cache rules and ETags are made from the answer the route gave.
        .layer(from_fn(etag::cache_rules))
        // Innermost of the checks on changes: the origin is judged first, then the token.
        .layer(from_fn_with_state(auth, auth::csrf_check))
        .layer(from_fn(origin_check))
        .layer(from_fn(time_limit))
        .layer(from_fn_with_state(
            Arc::clone(&limits),
            limits::limit_access_keys,
        ))
        .layer(from_fn_with_state(limits, limits::limit_requests))
        .layer(DefaultBodyLimit::max(BODY_LIMIT_BYTES))
        .layer(from_fn(errors::add_request_id))
}

/// Whether a request with this method changes something: any method but GET, HEAD and OPTIONS.
pub(crate) fn changes(method: &Method) -> bool {
    !matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

/// Refuses a request that changes something (see [`changes`]) below `/api`
/// unless it comes from the server's own origin: `Origin` names the same host as `Host`, or, when
/// the browser sent no `Origin`, `Sec-Fetch-Site` is `same-origin`. With neither the request is
/// refused (fail closed). This is the second layer after `SameSite` cookies and the CSRF token.
async fn origin_check(request: Request, next: Next) -> Response {
    if changes(request.method())
        && is_api(request.uri().path())
        && !is_key_request(request.headers())
        && !is_same_origin(&request)
    {
        return ApiError::new(ErrorCode::WrongOrigin).into_response();
    }
    next.run(request).await
}

/// The server's own origin is the scheme the browser used (see [`Scheme`]) and the authority it
/// asked for: the one in the address of the request when there is one (HTTP/2 and absolute-form
/// requests carry no `Host` header), else the `Host` header.
fn is_same_origin(request: &Request) -> bool {
    let headers = request.headers();
    let text = |name| headers.get(name).and_then(|value| value.to_str().ok());
    if let Some(origin) = text(header::ORIGIN) {
        let authority = request
            .uri()
            .authority()
            .map(axum::http::uri::Authority::as_str)
            .or_else(|| text(header::HOST));
        let scheme = request
            .extensions()
            .get::<Scheme>()
            .map_or("http", |scheme| scheme.0);
        // `null` and anything that is not scheme://host[:port] never match.
        return authority.is_some_and(|authority| {
            origin.eq_ignore_ascii_case(&format!("{scheme}://{authority}"))
        });
    }
    text(axum::http::HeaderName::from_static("sec-fetch-site")) == Some("same-origin")
}

/// Stops an API request that takes longer than [`TIME_LIMIT`].
async fn time_limit(request: Request, next: Next) -> Response {
    if !is_api(request.uri().path()) {
        return next.run(request).await;
    }
    match tokio::time::timeout(TIME_LIMIT, next.run(request)).await {
        Ok(response) => response,
        Err(_) => ApiError::new(ErrorCode::TimedOut).into_response(),
    }
}

/// The answer for a wrong method on a known address.
pub(crate) async fn method_not_allowed() -> ApiError {
    ApiError::new(ErrorCode::MethodNotAllowed)
}

/// The answer for an address below `/api` that does not exist.
pub(crate) fn not_found() -> Response {
    ApiError::new(ErrorCode::NotFound).into_response()
}

/// The ID in an address of the API; a text that is no ID names nothing, so it is not found.
pub(crate) fn path_id<Id: std::str::FromStr>(text: &str) -> Result<Id, ApiError> {
    text.parse().map_err(|_| ApiError::new(ErrorCode::NotFound))
}

/// The ID in a request body or query; a text that is no ID is not a valid request.
pub(crate) fn body_id<Id: std::str::FromStr>(text: &str) -> Result<Id, ApiError> {
    text.parse()
        .map_err(|_| ApiError::new(ErrorCode::InvalidRequest))
}

/// A member that is `Clear` when the request says `null` and `Keep` when it says nothing
/// (`#[serde(default, deserialize_with = "crate::api::null_or_value")]`).
pub(crate) fn null_or_value<'de, D, T>(deserializer: D) -> Result<Edit<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Edit::said)
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode, header};
    use axum::routing::{get, post};
    use bb_dev_words::accounts::PUBLIC_URL;
    use tower::ServiceExt as _;

    use super::*;
    use crate::errors::ApiJson;

    /// The host and port of the public address.
    fn authority() -> &'static str {
        PUBLIC_URL.strip_prefix("https://").unwrap()
    }

    /// The public address with another scheme.
    fn with_scheme(scheme: &str) -> String {
        format!("{scheme}://{}", authority())
    }

    fn change(origin: Option<&str>, fetch_site: Option<&str>, method: &str) -> Request<Body> {
        change_with(origin, fetch_site, method, "https")
    }

    fn change_with(
        origin: Option<&str>,
        fetch_site: Option<&str>,
        method: &str,
        scheme: &'static str,
    ) -> Request<Body> {
        let mut request = Request::builder()
            .method(method)
            .uri("/api/v1/echo")
            .header(header::HOST, authority())
            .header(header::CONTENT_TYPE, "application/json");
        if let Some(origin) = origin {
            request = request.header(header::ORIGIN, origin);
        }
        if let Some(site) = fetch_site {
            request = request.header("sec-fetch-site", site);
        }
        let mut request = request.body(Body::from("{}")).unwrap();
        request.extensions_mut().insert(Scheme(scheme));
        request
    }

    #[tokio::test]
    async fn sec_06_changes_must_come_from_the_servers_own_origin() {
        let own = Some(PUBLIC_URL);
        let allowed = [
            change(own, None, "POST"),
            change(own, Some("cross-site"), "POST"),
            change(None, Some("same-origin"), "DELETE"),
            change(Some(&PUBLIC_URL.to_uppercase()), None, "POST"),
        ];
        for request in allowed {
            let method = request.method().clone();
            let response = app().oneshot(request).await.unwrap();
            assert_ne!(response.status(), StatusCode::FORBIDDEN, "{method}");
        }
        let refused = [
            change(Some("https://evil.test"), None, "POST"),
            change(Some("https://evil.test"), Some("same-origin"), "POST"),
            change(
                PUBLIC_URL.rsplit_once(':').map(|(host, _)| host),
                None,
                "POST",
            ),
            change(Some("null"), None, "PUT"),
            change(None, Some("cross-site"), "POST"),
            change(None, Some("same-site"), "PATCH"),
            change(None, Some("none"), "POST"),
            change(None, None, "POST"),
        ];
        for request in refused {
            let response = app().oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
            assert_eq!(code_of(response).await, "request.wrong_origin");
        }
    }

    #[tokio::test]
    async fn sec_06_the_scheme_must_match_too() {
        // An http origin against an https server, and the other way round, is another origin.
        let http_origin = change(Some(&with_scheme("http")), None, "POST");
        let response = app().oneshot(http_origin).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let secure_origin = change_with(Some(PUBLIC_URL), None, "POST", "http");
        let response = app().oneshot(secure_origin).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let plain = change_with(Some(&with_scheme("http")), None, "POST", "http");
        let response = app().oneshot(plain).await.unwrap();
        assert_ne!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn sec_06_the_authority_of_the_address_counts_when_there_is_no_host_header() {
        // HTTP/2 and absolute-form requests have no Host header; the address holds the authority.
        let mut request = Request::builder()
            .method("POST")
            .uri(format!("{PUBLIC_URL}/api/v1/echo"))
            .header(header::ORIGIN, PUBLIC_URL)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from("{}"))
            .unwrap();
        request.extensions_mut().insert(Scheme("https"));
        assert!(request.headers().get(header::HOST).is_none());
        let response = app().oneshot(request).await.unwrap();
        assert_ne!(response.status(), StatusCode::FORBIDDEN);

        let mut other = Request::builder()
            .method("POST")
            .uri(format!("{PUBLIC_URL}/api/v1/echo"))
            .header(header::ORIGIN, "https://evil.test")
            .body(Body::from("{}"))
            .unwrap();
        other.extensions_mut().insert(Scheme("https"));
        let response = app().oneshot(other).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn sec_06_reads_and_pages_need_no_origin() {
        for method in ["GET", "HEAD", "OPTIONS"] {
            let response = app().oneshot(change(None, None, method)).await.unwrap();
            assert_ne!(response.status(), StatusCode::FORBIDDEN, "{method}");
        }
        let page = Request::post("/slow-page").body(Body::empty()).unwrap();
        let response = app().oneshot(page).await.unwrap();
        assert_ne!(response.status(), StatusCode::FORBIDDEN);
    }
    use crate::limits::LimitsConfig;

    async fn echo(ApiJson(value): ApiJson<serde_json::Value>) -> axum::Json<serde_json::Value> {
        axum::Json(value)
    }

    async fn slow() {
        tokio::time::sleep(Duration::from_hours(1)).await;
    }

    /// Routes that read a body and that never finish, with the rules of the API around them.
    fn app() -> Router {
        let routes = Router::new()
            .route("/api/v1/echo", post(echo))
            .route("/api/v1/slow", get(slow))
            .route("/slow-page", get(slow));
        rules(
            routes,
            Limits::new(LimitsConfig::default()),
            AuthHandle::none(),
        )
    }

    async fn code_of(response: Response) -> String {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        body["code"].as_str().unwrap().to_owned()
    }

    fn post_json(body: Vec<u8>) -> Request<Body> {
        Request::post("/api/v1/echo")
            .header(header::HOST, "bugaboo.test")
            .header(header::ORIGIN, "http://bugaboo.test")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap()
    }

    #[tokio::test]
    async fn sec_14_a_body_up_to_the_limit_is_read_and_a_bigger_one_is_refused_as_problem_json() {
        let small = app()
            .oneshot(post_json(br#"{"a":1}"#.to_vec()))
            .await
            .unwrap();
        assert_eq!(small.status(), StatusCode::OK);

        let mut big = br#"{"a":""#.to_vec();
        big.extend(std::iter::repeat_n(b'x', BODY_LIMIT_BYTES));
        big.extend_from_slice(br#""}"#);
        let refused = app().oneshot(post_json(big)).await.unwrap();
        assert_eq!(refused.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(
            refused.headers()[header::CONTENT_TYPE],
            "application/problem+json"
        );
        assert_eq!(code_of(refused).await, "request.too_large");
    }

    #[tokio::test(start_paused = true)]
    async fn sec_10_an_api_request_that_takes_too_long_is_stopped_but_a_page_is_not() {
        let slow = Request::get("/api/v1/slow").body(Body::empty()).unwrap();
        let stopped = app().oneshot(slow).await.unwrap();
        assert_eq!(stopped.status(), StatusCode::REQUEST_TIMEOUT);
        assert_eq!(code_of(stopped).await, "request.timed_out");

        // Not an API address: no time limit of the API applies (the hour passes in paused time).
        let page = Request::get("/slow-page").body(Body::empty()).unwrap();
        let finished = app().oneshot(page).await.unwrap();
        assert_eq!(finished.status(), StatusCode::OK);
    }

    #[test]
    fn dist_01_every_spelling_of_an_api_address_is_an_api_address() {
        for path in [
            "/api",
            "/api/",
            "/api/x",
            "/API/x",
            "/%61pi/x",
            "/./api/x",
            "/%2e/api/x",
            "/api/%zz",
        ] {
            assert!(is_api(path), "{path}");
        }
        for path in ["/", "/health/live", "/apiary", "/assets/api", "/issues/api"] {
            assert!(!is_api(path), "{path}");
        }
    }
}
