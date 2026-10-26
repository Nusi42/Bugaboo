//! The sign-in routes below `/api/v1/auth` (AUTH-01…03, AUTH-06, AUTH-09, SEC-12, SEC-14): what
//! they share. The routes themselves are in `auth_sign_in.rs`, `auth_factors.rs` and
//! `auth_people.rs`; the use cases are in `bb-app`, behind [`AuthApi`].
//!
//! Three things happen here for every route:
//! - **Who is asking:** [`Session`] reads the session cookie, [`Info`] the client network and the
//!   browser, so the use cases can write them into sessions and the security log.
//! - **CSRF (SEC-12, layer 3):** [`csrf_check`] refuses every change below `/api` that carries a
//!   session cookie but not the token of that session in `x-csrf-token`. It is a middleware, not a
//!   check in each handler, so a route added later is protected without anyone remembering. Only
//!   the public routes that change something are left out (`crate::permissions::csrf_exempt`, from
//!   the route table): people who sign in or sign up have no session yet. They are protected by
//!   the origin check and carry no secret a forged request could use.
//! - **Cookies:** a new session sets the session cookie and the CSRF cookie together, in one
//!   place ([`signed_in`]); both come from the cookie table (`crate::cookies`).

use std::sync::Arc;

use axum::extract::{FromRequestParts, MatchedPath, Request, State};
use axum::http::request::Parts;
use axum::http::{StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse as _, Response};
use bb_app::auth::{AppError, AuthApi, RequestInfo, Started};
use bb_auth::Device;
use bb_core::ErrorCode;
use bb_core::error_code::PROBLEM_CONTENT_TYPE;
use bb_core::network::NetworkPrefix;
use bb_policy::{Authorized, Masked, Permission};
use serde::Serialize;
use std::collections::BTreeMap;

use utoipa::openapi::response::{Response as ResponseDescription, ResponseBuilder};
use utoipa::openapi::{ContentBuilder, Ref, RefOr};
use utoipa::{IntoResponses, ToSchema};
use utoipa_axum::router::OpenApiRouter;

use super::is_api;
use crate::conflict::ConflictBody;
use crate::cookies::{BACKGROUND_HEADER, CSRF_HEADER, Cookie};
use crate::errors::ApiError;
use crate::limits::{Limits, limit_sign_in, limit_sign_up};
use crate::logging::ClientIp;
use crate::permissions::csrf_exempt;

/// What the sign-in routes need: the use cases, if the server has a database. Without them
/// (tests of the other parts of the API) every sign-in route answers that the server cannot
/// answer right now.
#[derive(Clone)]
pub struct AuthHandle(pub(crate) Option<Arc<dyn AuthApi>>);

impl std::fmt::Debug for AuthHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AuthHandle")
            .field("ready", &self.0.is_some())
            .finish()
    }
}

impl AuthHandle {
    /// The use cases of signing in.
    #[must_use]
    pub fn new(api: Arc<dyn AuthApi>) -> Self {
        Self(Some(api))
    }

    /// No sign-in: the routes answer `server.unavailable`.
    #[must_use]
    pub const fn none() -> Self {
        Self(None)
    }

    pub(crate) fn api(&self) -> Result<&dyn AuthApi, ApiError> {
        self.0
            .as_deref()
            .ok_or_else(|| ApiError::new(ErrorCode::Unavailable))
    }
}

impl From<AppError> for ApiError {
    fn from(error: AppError) -> Self {
        match error {
            AppError::Refused(code) => Self::new(code),
            AppError::RefusedAtField { code, field } => Self::new(code).with_field(field),
            AppError::SessionEnded => Self::new(ErrorCode::NotSignedIn).with_session_ended(),
            AppError::Conflict(conflict) => {
                Self::new(ErrorCode::VersionChanged).with_conflict(ConflictBody::from(&*conflict))
            }
            // Only a use case that is called without `AuthApi` can leave a stale version unnamed.
            AppError::Stale(_) => Self::new(ErrorCode::VersionChanged),
            AppError::TakenOver { by } => {
                Self::new(ErrorCode::EditTakenOver).with_taken_over_by(by)
            }
            AppError::Mistakes(found) => Self::new(ErrorCode::SearchInvalid).with_mistakes(&found),
            AppError::Limited { retry_after } => {
                Self::new(ErrorCode::TooManyRequests).with_retry_after(retry_after)
            }
            AppError::Internal(cause) => Self::internal(&cause),
        }
    }
}

/// The client network and the browser of the request.
pub(crate) struct Info(pub(crate) RequestInfo);

impl<S: Send + Sync> FromRequestParts<S> for Info {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, ApiError> {
        let network = parts
            .extensions
            .get::<ClientIp>()
            .map(|ClientIp(address)| NetworkPrefix::from_address(*address));
        let device = parts
            .headers
            .get(header::USER_AGENT)
            .and_then(|value| value.to_str().ok())
            .map(Device::from_user_agent)
            .unwrap_or_default();
        let background = parts.headers.contains_key(BACKGROUND_HEADER)
            && parts.extensions.get::<MatchedPath>().is_some_and(|route| {
                crate::openapi::may_be_background_read(&parts.method, route.as_str())
            });
        Ok(Self(RequestInfo {
            network,
            device,
            background,
        }))
    }
}

/// What the request signs in with: the secret in the session cookie, or else the access key of
/// a script (`Authorization: Bearer bb_…`). A request with neither is not signed in. The use case
/// tells the two apart: what judges a permission takes both (a key is narrowed to what it may do),
/// and what belongs to the person's own sign-in data takes sessions only and turns a key away, so
/// a route cannot forget to keep keys out of it (`bb_app::auth`, `Ctx::actor` and `Ctx::session`).
pub(crate) struct Session(pub(crate) String);

impl<S: Send + Sync> FromRequestParts<S> for Session {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, ApiError> {
        Cookie::Session
            .read(&parts.headers)
            .or_else(|| super::presented_key(&parts.headers).map(str::to_owned))
            .map(Self)
            .ok_or_else(|| ApiError::new(ErrorCode::NotSignedIn))
    }
}

/// The secret in the session cookie when there is one: signing in from a browser that still has
/// a session ends that old session. No cookie is no problem.
pub(crate) struct PreviousSession(pub(crate) Option<String>);

impl<S: Send + Sync> FromRequestParts<S> for PreviousSession {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, ApiError> {
        Ok(Self(Cookie::Session.read(&parts.headers)))
    }
}

/// Who a person is, as the browser app shows them.
#[derive(Serialize, ToSchema)]
pub(crate) struct PersonBody {
    /// The ID of the person.
    id: String,
    /// The name others see.
    name: String,
    /// The e-mail address used to sign in.
    email: String,
}

impl From<&bb_app::auth::PersonView> for PersonBody {
    fn from(person: &bb_app::auth::PersonView) -> Self {
        Self {
            id: person.id.to_string(),
            name: person.display_name.clone(),
            email: person.email.clone(),
        }
    }
}

/// Who is signed in, and what they may do for the whole organization (the menus of the browser app
/// follow it; the server decides again at every request).
#[derive(Serialize, ToSchema)]
pub(crate) struct MeBody {
    /// The ID of the person.
    pub(crate) id: String,
    /// The name others see.
    pub(crate) name: String,
    /// The e-mail address used to sign in.
    pub(crate) email: String,
    /// What the person may do for the whole organization. Nothing while the person still has to
    /// set up a second step.
    pub(crate) permissions: Vec<Permission>,
    /// The person has to set up a second step before they can do anything else (their role or
    /// the organization asks for two-step sign-in and they have none).
    pub(crate) second_step_setup_required: bool,
}

impl Masked for MeBody {}

/// Which ways there are to finish signing in.
#[derive(Serialize, ToSchema)]
pub(crate) struct SecondStepBody {
    /// A code from the code app.
    code_app: bool,
    /// A passkey.
    passkey: bool,
    /// A backup code (only with a code app).
    backup_code: bool,
}

/// The answer to signing in. The cookies come with it.
#[derive(Serialize, ToSchema)]
pub(crate) struct SignedInBody {
    /// Who signed in. Left out while the second step is still to do: a password alone shows
    /// nothing about the person.
    person: Option<PersonBody>,
    /// When present, the password was right but the person still has to finish with one of
    /// these ways before anything else opens.
    second_step: Option<SecondStepBody>,
}

/// The answer for a new session: the body, the session cookie and the CSRF cookie.
pub(crate) fn signed_in(started: &Started) -> Result<Response, ApiError> {
    let body = SignedInBody {
        person: started
            .second_step_needed
            .is_none()
            .then(|| PersonBody::from(&started.person)),
        second_step: started.second_step_needed.map(|methods| SecondStepBody {
            code_app: methods.code_app,
            passkey: methods.passkey,
            backup_code: methods.backup_code,
        }),
    };
    // The answer sets the cookies of a session: `crate::etag` treats every answer that sets a
    // cookie as a secret, so no cache keeps it.
    let mut response = answer(body);
    for (cookie, value) in [
        (Cookie::Session, &started.session_token),
        (Cookie::Csrf, &started.csrf_token),
    ] {
        cookie
            .put(&mut response, value, Some(started.lifetime))
            .map_err(|error| ApiError::internal(&error))?;
    }
    Ok(response)
}

/// The answer for "done, nothing to say": `204`. Never cached.
pub(crate) fn done() -> Response {
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    response
}

/// A JSON answer. Who may keep it, and for how long, is decided in one place for every answer
/// (`crate::etag`), from the marks in the API description.
pub(crate) fn answer<T: Serialize>(body: T) -> Response {
    axum::Json(body).into_response()
}

/// The one answer wrapper of data that leaves a use case: an `Authorized<T>` is sent with its
/// hidden fields left out (SEC-04, ADR-0004). Handlers that return what a use case gave them use
/// this and nothing else.
pub(crate) fn answer_authorized<T: Masked + Serialize>(
    body: &Authorized<T>,
) -> Result<Response, ApiError> {
    let value = body
        .masked_value()
        .map_err(|error| ApiError::internal(&error))?;
    Ok(answer(value))
}

/// Refuses a change that carries a session cookie without the token of that session. A change
/// without a session cookie is not signed in (`401`), except on the routes that are open without
/// a token ([`csrf_exempt`]).
pub(crate) async fn csrf_check(
    State(auth): State<AuthHandle>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path();
    if !super::changes(request.method()) || !is_api(path) || csrf_exempt(request.method(), path) {
        return next.run(request).await;
    }
    // Without a database there is no session to protect; the routes answer "unavailable".
    let Some(api) = auth.0.as_deref() else {
        return next.run(request).await;
    };
    // A script with an access key has no cookie for a forged request to ride on.
    if super::is_key_request(request.headers()) {
        return next.run(request).await;
    }
    let Some(session) = Cookie::Session.read(request.headers()) else {
        return ApiError::new(ErrorCode::NotSignedIn).into_response();
    };
    let presented = request
        .headers()
        .get(CSRF_HEADER)
        .and_then(|value| value.to_str().ok());
    if presented.is_some_and(|token| api.csrf_matches(&session, token)) {
        next.run(request).await
    } else {
        ApiError::new(ErrorCode::WrongToken).into_response()
    }
}

/// Declares an answer that is problem+json with the code(s) in the description, for the API
/// description. These are the errors a route can add to the standard ones (`crate::openapi`).
macro_rules! problem_answers {
    ($($(#[$doc:meta])* $name:ident => ($status:literal, $description:literal)),+ $(,)?) => {
        $($(#[$doc])*
        pub(crate) struct $name;

        impl IntoResponses for $name {
            fn responses() -> BTreeMap<String, RefOr<ResponseDescription>> {
                let content = ContentBuilder::new()
                    .schema(Some(Ref::from_schema_name("Problem")))
                    .build();
                let response = ResponseBuilder::new()
                    .description($description)
                    .content(PROBLEM_CONTENT_TYPE, content)
                    .build();
                BTreeMap::from([($status.to_owned(), RefOr::T(response))])
            }
        })+
    };
}

pub(crate) use problem_answers;

problem_answers! {
    /// The request is not valid.
    InvalidAnswer => ("400", "The request is not valid (request.invalid, auth.password-too-short, auth.password-breached, auth.invitation-invalid, auth.passkey-rejected)."),
    /// Not signed in.
    NotSignedInAnswer => ("401", "Not signed in (auth.not-signed-in)."),
    /// A change without the right token, or not allowed.
    ForbiddenAnswer => ("403", "Not allowed (access.forbidden, request.wrong-token, auth.second-step-required, auth.access-key-not-allowed)."),
    /// Unknown.
    NotFoundAnswer => ("404", "There is no such thing (resource.not-found)."),
    /// Something stands in the way of a change.
    ConflictAnswer => ("409", "Something stands in the way of the change: a name that is taken, a change somebody made first, or a rule about moving projects (name.taken, edit.version-changed, project.move-lifts-no-access, project.move-into-itself, project.too-deep, project.preview-changed)."),
    /// Too soon after the last try.
    TooSoonAnswer => ("429", "Too soon after the last time (rate-limit.exceeded, handle.changed-lately)."),
}

/// The routes of the sign-in family, with the small per-network limit of SEC-14 on those that
/// try a password, a code, a secret or an address (`limits`).
pub(crate) fn routes<S>(limits: &Arc<Limits>) -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    let tried = super::auth_sign_in::tried_routes::<S>()
        .merge(super::auth_people::tried_routes::<S>())
        .route_layer(axum::middleware::from_fn_with_state(
            Arc::clone(limits),
            limit_sign_in,
        ));
    // Signing up sends an e-mail to whatever address is typed: a small limit of its own.
    let sign_up = super::auth_people::sign_up_routes::<S>()
        .route_layer(axum::middleware::from_fn_with_state(
            Arc::clone(limits),
            limit_sign_up,
        ))
        .route_layer(axum::middleware::from_fn_with_state(
            Arc::clone(limits),
            limit_sign_in,
        ));
    tried
        .merge(sign_up)
        .merge(super::auth_sign_in::session_routes::<S>())
        .merge(super::auth_factors::session_routes::<S>())
        .merge(super::auth_people::session_routes::<S>())
}
