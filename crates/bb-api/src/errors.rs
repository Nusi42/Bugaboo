//! Error answers: `application/problem+json` with a code, a message key and the request ID
//! (architecture §7 "Errors", ADR-0008, UX-07).
//!
//! Every error the API sends is an [`ApiError`], made from a [`bb_core::ErrorCode`]. The code
//! decides the status and the message; nothing else can reach the client. Details of what went
//! wrong inside stay in the log ([`ApiError::internal`]). ARCH-ERRORS finds error answers that
//! are built from a text written in the code, and error statuses named outside this file.

use std::fmt::Display;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRequest, FromRequestParts, Request};
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use bb_core::{ErrorCode, Problem, error_code::PROBLEM_CONTENT_TYPE};
use serde::de::DeserializeOwned;

use crate::api::is_api;
use crate::conflict::{ConflictBody, ConflictProblem};
use crate::cookies::Cookie;
use crate::logging::RequestId;

/// An error answer in the making.
///
/// The body is the [`Problem`] of the code. The middleware [`add_request_id`] adds the ID of the
/// request, which only it knows.
#[derive(Clone, Debug)]
pub struct ApiError {
    code: ErrorCode,
    retry_after: Option<u64>,
    /// The session of the request has ended: the answer also makes the browser forget the
    /// session cookie and what it kept (M2-F06).
    ends_session: bool,
    /// What changed the record, for `edit.version_changed` (ADR-0019 amendment M3).
    conflict: Option<Arc<ConflictBody>>,
    /// The field a `field.*` refusal is about.
    field: Option<String>,
}

/// Tells [`add_request_id`] which answers to rewrite, and with what.
#[derive(Clone, Debug)]
struct ProblemOf(ErrorCode, Option<Arc<ConflictBody>>, Option<String>);

impl ApiError {
    /// The answer for `code`.
    #[must_use]
    pub const fn new(code: ErrorCode) -> Self {
        Self {
            code,
            retry_after: None,
            ends_session: false,
            conflict: None,
            field: None,
        }
    }

    /// Names the field a `field.*` refusal is about.
    #[must_use]
    pub(crate) fn with_field(mut self, field: String) -> Self {
        self.field = Some(field);
        self
    }

    /// Adds what changed the record to an `edit.version_changed` answer.
    #[must_use]
    pub(crate) fn with_conflict(mut self, conflict: ConflictBody) -> Self {
        self.conflict = Some(Arc::new(conflict));
        self
    }

    /// The server failed. `cause` goes to the log and nowhere else; the person gets the
    /// standard message, which blames no one and reveals nothing.
    #[must_use]
    pub fn internal(cause: &dyn Display) -> Self {
        tracing::error!(cause = %cause, "request failed inside the server");
        Self::new(ErrorCode::Internal)
    }

    /// Tells the client how many seconds to wait before it tries again (`Retry-After`).
    #[must_use]
    pub const fn with_retry_after(mut self, seconds: u64) -> Self {
        self.retry_after = Some(seconds);
        self
    }

    /// Marks the answer as the one for a session that has ended: it clears both cookies, and
    /// with them the cache of the browser (`crate::etag` adds `Clear-Site-Data` to every answer
    /// that clears the session cookie).
    #[must_use]
    pub const fn with_session_ended(mut self) -> Self {
        self.ends_session = true;
        self
    }

    /// The error code.
    #[must_use]
    pub const fn code(&self) -> ErrorCode {
        self.code
    }
}

impl From<ErrorCode> for ApiError {
    fn from(code: ErrorCode) -> Self {
        Self::new(code)
    }
}

/// The HTTP status of a code.
fn status_of(code: ErrorCode) -> StatusCode {
    StatusCode::from_u16(code.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}

/// The problem of a code, naming the field when the refusal is about one.
fn problem_of(code: ErrorCode, field: Option<String>) -> Problem {
    let problem = Problem::new(code);
    match field {
        Some(field) => problem.with_field(field),
        None => problem,
    }
}

fn problem_response(problem: &Problem, conflict: Option<&ConflictBody>) -> Response {
    // A `Problem` holds text and numbers only, so it always serializes; the empty body is what
    // a (impossible) failure would leave.
    let body = serde_json::to_vec(&ConflictProblem::new(problem.clone(), conflict.cloned()))
        .unwrap_or_default();
    let mut response = Response::new(Body::from(body));
    *response.status_mut() = status_of(problem.code());
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(PROBLEM_CONTENT_TYPE),
    );
    // An error answer is about one request; no cache may keep it.
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut response = problem_response(
            &problem_of(self.code, self.field.clone()),
            self.conflict.as_deref(),
        );
        if let Some(seconds) = self.retry_after {
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from(seconds));
        }
        if self.ends_session {
            Cookie::Session.forget(&mut response);
            Cookie::Csrf.forget(&mut response);
        }
        response
            .extensions_mut()
            .insert(ProblemOf(self.code, self.conflict, self.field));
        response
    }
}

/// Puts the ID of the request into every error answer. It runs inside the logging layer, which
/// makes the ID, and rewrites the body of the answers [`ApiError`] made.
pub(crate) async fn add_request_id(request: Request, next: Next) -> Response {
    let id = request.extensions().get::<RequestId>().cloned();
    let api = is_api(request.uri().path());
    let mut response = next.run(request).await;
    let marked = response.extensions().get::<ProblemOf>().cloned();
    // Anything below /api that fails without a problem body (a rejection of an extractor, a
    // layer) is turned into one by its status; the text it came with is dropped.
    let unmarked = (api && marked.is_none() && is_failure(response.status()))
        .then(|| ProblemOf(code_for_status(response.status()), None, None));
    let (Some(RequestId(id)), Some(ProblemOf(code, conflict, field))) = (id, marked.or(unmarked))
    else {
        return response;
    };
    let rewritten = problem_response(
        &problem_of(code, field).with_request_id(id),
        conflict.as_deref(),
    );
    let (parts, body) = rewritten.into_parts();
    *response.status_mut() = parts.status;
    // Keep what the answer already says (`Retry-After`, `Allow`); the new body replaces the old.
    response.headers_mut().remove(header::CONTENT_LENGTH);
    response.headers_mut().extend(parts.headers);
    *response.body_mut() = body;
    response
}

fn is_failure(status: StatusCode) -> bool {
    status.is_client_error() || status.is_server_error()
}

/// The code for a failure that did not come with one.
fn code_for_status(status: StatusCode) -> ErrorCode {
    match status {
        StatusCode::BAD_REQUEST
        | StatusCode::UNSUPPORTED_MEDIA_TYPE
        | StatusCode::UNPROCESSABLE_ENTITY => ErrorCode::InvalidRequest,
        StatusCode::PAYLOAD_TOO_LARGE => ErrorCode::TooLarge,
        StatusCode::REQUEST_TIMEOUT => ErrorCode::TimedOut,
        StatusCode::NOT_FOUND => ErrorCode::NotFound,
        StatusCode::METHOD_NOT_ALLOWED => ErrorCode::MethodNotAllowed,
        _ => ErrorCode::Internal,
    }
}

/// The query string of the request, with a bad one answered as problem+json.
///
/// Use this instead of `axum::extract::Query` (ARCH-ERRORS).
#[derive(Debug)]
pub struct ApiQuery<T>(pub T);

impl<S, T> FromRequestParts<S> for ApiQuery<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, ApiError> {
        axum::extract::Query::<T>::from_request_parts(parts, state)
            .await
            .map(|axum::extract::Query(value)| Self(value))
            .map_err(|_| ApiError::new(ErrorCode::InvalidRequest))
    }
}

/// The parts of the address, with a bad one answered as problem+json.
///
/// Use this instead of `axum::extract::Path` (ARCH-ERRORS). An address part that does not fit
/// answers like an address that does not exist.
#[derive(Debug)]
pub struct ApiPath<T>(pub T);

impl<S, T> FromRequestParts<S> for ApiPath<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Send,
{
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, ApiError> {
        axum::extract::Path::<T>::from_request_parts(parts, state)
            .await
            .map(|axum::extract::Path(value)| Self(value))
            .map_err(|_| ApiError::new(ErrorCode::NotFound))
    }
}

/// A JSON body from the request, with the errors of a bad body answered as problem+json.
///
/// Use this instead of `axum::Json` for request bodies, so a malformed, too large or too slow
/// body never gets a plain-text answer.
#[derive(Debug)]
pub struct ApiJson<T>(pub T);

impl<S, T> FromRequest<S> for ApiJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = ApiError;

    async fn from_request(request: Request, state: &S) -> Result<Self, ApiError> {
        match axum::Json::<T>::from_request(request, state).await {
            Ok(axum::Json(value)) => Ok(Self(value)),
            Err(rejection) => Err(ApiError::new(code_of(&rejection))),
        }
    }
}

/// What a rejected body means for the person: too big has its own answer, everything else is a
/// request that is not valid. The text of the rejection is not sent. (A body that arrives too
/// slowly is stopped by the time limit of the whole request, `api::time_limit`.)
fn code_of(rejection: &JsonRejection) -> ErrorCode {
    match rejection.status() {
        StatusCode::PAYLOAD_TOO_LARGE => ErrorCode::TooLarge,
        _ => ErrorCode::InvalidRequest,
    }
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::routing::post;
    use tower::ServiceExt as _;

    use super::*;

    async fn body_json(response: Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn sec_14_every_code_answers_with_its_status_and_problem_json() {
        for code in ErrorCode::ALL {
            let response = ApiError::new(*code).into_response();
            assert_eq!(response.status().as_u16(), code.status());
            assert_eq!(
                response.headers()[header::CONTENT_TYPE],
                "application/problem+json"
            );
            assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
            let body = body_json(response).await;
            assert_eq!(body["code"], code.code());
            assert_eq!(body["message_key"], code.message().as_str());
            assert_eq!(body["status"], code.status());
        }
    }

    #[tokio::test]
    async fn sec_14_retry_after_is_a_header_of_the_answer() {
        let response = ApiError::new(ErrorCode::TooManyRequests)
            .with_retry_after(17)
            .into_response();
        assert_eq!(response.headers()[header::RETRY_AFTER], "17");
    }

    #[tokio::test]
    async fn the_request_id_is_added_to_the_body_and_nothing_else_changes() {
        async fn fail() -> ApiError {
            ApiError::new(ErrorCode::TooManyRequests).with_retry_after(3)
        }
        let app = Router::new()
            .route("/", axum::routing::get(fail))
            .layer(axum::middleware::from_fn(add_request_id));
        let mut request = Request::get("/").body(Body::empty()).unwrap();
        request
            .extensions_mut()
            .insert(RequestId("abc123".to_owned()));
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers()[header::RETRY_AFTER], "3");
        assert_eq!(
            response.headers()[header::CONTENT_TYPE],
            "application/problem+json"
        );
        let body = body_json(response).await;
        assert_eq!(body["request_id"], "abc123");
        assert_eq!(body["code"], "rate_limit.exceeded");
    }

    #[tokio::test]
    async fn answers_that_are_not_errors_pass_through_untouched() {
        let app = Router::new()
            .route("/", axum::routing::get(|| async { "fine" }))
            .layer(axum::middleware::from_fn(add_request_id));
        let mut request = Request::get("/").body(Body::empty()).unwrap();
        request.extensions_mut().insert(RequestId("x".to_owned()));
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn internal_causes_never_reach_the_client() {
        let response = ApiError::internal(&"password=hunter2 in table people").into_response();
        let text = body_json(response).await.to_string();
        assert!(!text.contains("hunter2"), "{text}");
        assert!(text.contains("server.internal"));
    }

    #[tokio::test]
    async fn a_plain_failure_below_api_becomes_a_problem_by_its_status() {
        async fn plain() -> (StatusCode, &'static str) {
            (StatusCode::UNPROCESSABLE_ENTITY, "bad field x")
        }
        let app = Router::new()
            .route("/api/v1/x", axum::routing::get(plain))
            .route("/page", axum::routing::get(plain))
            .layer(axum::middleware::from_fn(add_request_id));
        let get = |path: &str| {
            let mut request = Request::get(path).body(Body::empty()).unwrap();
            request.extensions_mut().insert(RequestId("r1".to_owned()));
            request
        };
        let api = app.clone().oneshot(get("/api/v1/x")).await.unwrap();
        assert_eq!(api.status(), StatusCode::BAD_REQUEST);
        let body = body_json(api).await;
        assert_eq!(body["code"], "request.invalid");
        assert_eq!(body["request_id"], "r1");
        assert!(!body.to_string().contains("bad field"));
        // Not the API: left alone.
        let page = app.oneshot(get("/page")).await.unwrap();
        assert_eq!(page.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn plain_statuses_map_to_codes() {
        for (status, code) in [
            (StatusCode::BAD_REQUEST, ErrorCode::InvalidRequest),
            (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                ErrorCode::InvalidRequest,
            ),
            (StatusCode::PAYLOAD_TOO_LARGE, ErrorCode::TooLarge),
            (StatusCode::REQUEST_TIMEOUT, ErrorCode::TimedOut),
            (StatusCode::BAD_GATEWAY, ErrorCode::Internal),
        ] {
            assert_eq!(code_for_status(status), code);
        }
    }

    #[tokio::test]
    async fn bad_query_and_path_values_are_problems() {
        #[derive(serde::Deserialize)]
        struct Page {
            limit: u32,
        }
        async fn list(ApiQuery(page): ApiQuery<Page>) -> String {
            page.limit.to_string()
        }
        async fn one(ApiPath(id): ApiPath<u32>) -> String {
            id.to_string()
        }
        let app = || {
            Router::new()
                .route("/q", axum::routing::get(list))
                .route("/p/{id}", axum::routing::get(one))
        };
        let get = |path: &str| Request::get(path).body(Body::empty()).unwrap();
        let ok = app().oneshot(get("/q?limit=5")).await.unwrap();
        assert_eq!(ok.status(), StatusCode::OK);
        let bad = app().oneshot(get("/q?limit=many")).await.unwrap();
        assert_eq!(body_json(bad).await["code"], "request.invalid");
        let missing = app().oneshot(get("/p/abc")).await.unwrap();
        assert_eq!(body_json(missing).await["code"], "resource.not_found");
        let fine = app().oneshot(get("/p/7")).await.unwrap();
        assert_eq!(fine.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn a_bad_json_body_is_a_problem_not_plain_text() {
        #[derive(serde::Deserialize)]
        struct Input {
            name: String,
        }
        async fn take(ApiJson(input): ApiJson<Input>) -> String {
            input.name
        }
        let app = || {
            Router::new()
                .route("/", post(take))
                .layer(axum::middleware::from_fn(add_request_id))
        };
        for (content_type, body) in [
            ("application/json", "{not json"),
            ("application/json", r#"{"other":1}"#),
            ("text/plain", "{}"),
        ] {
            let request = Request::post("/")
                .header(header::CONTENT_TYPE, content_type)
                .body(Body::from(body))
                .unwrap();
            let response = app().oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{body}");
            assert_eq!(body_json(response).await["code"], "request.invalid");
        }
    }
}
