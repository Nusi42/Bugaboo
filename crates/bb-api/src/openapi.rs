//! The OpenAPI document of the API (ADR-0008).
//!
//! The document is built from the handlers' `#[utoipa::path]` descriptions (`crate::api`), so
//! it always matches the routes. `GET /api/v1/openapi.json` serves it; `bugaboo openapi` prints
//! it; `cargo xtask gen api` writes it to `web/packages/api-client/openapi.json` and generates
//! the TypeScript types from it. The document is also the input of ARCH-LEAN (list routes), and
//! its error part of ARCH-ERRORS.

use std::sync::OnceLock;

use axum::http::header::RETRY_AFTER;
use bb_core::{ErrorCode, Problem, error_code::PROBLEM_CONTENT_TYPE};
use utoipa::openapi::path::{Operation, PathItem};
use utoipa::openapi::response::{ResponseBuilder, ResponsesBuilder};
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::openapi::{ContentBuilder, HeaderBuilder, ObjectBuilder, Ref, Type};
use utoipa::{Modify, OpenApi};

use crate::api::{AuthHandle, ProbeHandle};
use crate::cookies::{CSRF_HEADER, Cookie};

/// The name of the security scheme for the session cookie.
const SESSION_SCHEME: &str = "sessionCookie";
/// The name of the security scheme for the CSRF header.
const CSRF_SCHEME: &str = "csrfToken";

/// The parts of the document that do not belong to one route.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Bugaboo API",
        description = "The API of Bugaboo. The browser app uses exactly this API. Every error \
            is application/problem+json with a stable code and a message key."
    ),
    components(schemas(Problem, ErrorCode)),
    tags(
        (name = "server", description = "The server itself."),
        (name = "auth", description = "Signing in, sessions, second step, passkeys, invitations and setup."),
        (name = "admin", description = "What only administrators see: the state of the server.")
    )
)]
struct ApiDoc;

/// The document without routes; `crate::api::routes` adds them.
pub(crate) fn base() -> utoipa::openapi::OpenApi {
    let mut document = ApiDoc::openapi();
    // The licence of the product is not decided here; an empty name would be invalid.
    document.info.license = None;
    document
}

/// The state the routes are described with: nothing is served, so nothing is behind the handles.
#[derive(Clone)]
struct Described {
    auth: AuthHandle,
    probe: ProbeHandle,
}

impl axum::extract::FromRef<Described> for AuthHandle {
    fn from_ref(state: &Described) -> Self {
        state.auth.clone()
    }
}

impl axum::extract::FromRef<Described> for ProbeHandle {
    fn from_ref(state: &Described) -> Self {
        state.probe.clone()
    }
}

/// The finished document.
#[must_use]
pub fn document() -> utoipa::openapi::OpenApi {
    // The limits only decide how a route is wrapped; the description is the same for any numbers.
    let limits = crate::limits::Limits::new(crate::limits::LimitsConfig::default());
    let mut document = crate::api::routes::<Described>(&limits).into_openapi();
    // After the routes are in: the standard answers go to every route.
    StandardParts.modify(&mut document);
    document
}

/// The document as text, in the form that is written to disk (pretty, stable order, one final
/// line break).
#[must_use]
pub fn json() -> &'static str {
    static JSON: OnceLock<String> = OnceLock::new();
    JSON.get_or_init(|| {
        // A document of plain data always serializes; the empty text is what an impossible
        // failure would leave, and the contract test would catch it.
        let mut text = document().to_pretty_json().unwrap_or_default();
        text.push('\n');
        text
    })
}

/// What every route has: the answers every request can get, and the security schemes.
struct StandardParts;

impl Modify for StandardParts {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            SESSION_SCHEME,
            SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new(Cookie::Session.name()))),
        );
        components.add_security_scheme(
            CSRF_SCHEME,
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new(CSRF_HEADER))),
        );
        for item in openapi.paths.paths.values_mut() {
            for operation in operations(item) {
                add_standard_answers(operation);
            }
        }
    }
}

fn operations(item: &mut PathItem) -> impl Iterator<Item = &mut Operation> {
    [
        &mut item.get,
        &mut item.put,
        &mut item.post,
        &mut item.delete,
        &mut item.options,
        &mut item.head,
        &mut item.patch,
        &mut item.trace,
    ]
    .into_iter()
    .filter_map(Option::as_mut)
}

/// Any request can be refused for going over a limit, or fail inside the server.
fn add_standard_answers(operation: &mut Operation) {
    let problem = || {
        ContentBuilder::new()
            .schema(Some(Ref::from_schema_name("Problem")))
            .build()
    };
    let answer = |code: ErrorCode| {
        ResponseBuilder::new().description(format!("{} ({})", code.message(), code.code()))
    };
    let mut responses = ResponsesBuilder::new();
    for code in [
        ErrorCode::TooManyRequests,
        ErrorCode::Internal,
        ErrorCode::Unavailable,
    ] {
        let mut response = answer(code).content(PROBLEM_CONTENT_TYPE, problem());
        if code == ErrorCode::TooManyRequests {
            response = response.header(
                RETRY_AFTER.as_str(),
                HeaderBuilder::new()
                    .description(Some("Seconds to wait before trying again."))
                    .schema(Some(
                        ObjectBuilder::new().schema_type(Type::Integer).build(),
                    ))
                    .build(),
            );
        }
        responses = responses.response(code.status().to_string(), response.build());
    }
    for (status, response) in &responses.build().responses {
        operation
            .responses
            .responses
            .entry(status.clone())
            .or_insert_with(|| response.clone());
    }
}
