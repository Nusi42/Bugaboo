//! HTTP: routes, OpenAPI, security headers, rate limits and the embedded browser app (ADR-0008).
//!
//! The health endpoints, the API below `/api/v1` with its OpenAPI document, problem+json errors,
//! rate limits and the cookie policy, the browser app (embedded in release builds, read from disk
//! in debug builds or from a folder), HTTPS, the security headers on every response (SEC-10,
//! SEC-11), response compression (never for JSON) and one log line per request without personal
//! data (OPS-08).

#![forbid(unsafe_code)]

mod api;
pub mod cookies;
pub mod errors;
mod failures;
mod headers;
mod lean;
pub mod limits;
mod listener;
mod logging;
pub mod openapi;
mod paging;
mod router;
pub mod tls;
mod web;

pub use api::{AuthHandle, BODY_LIMIT_BYTES, ProbeHandle, TIME_LIMIT, VERSION_PATH};
pub use errors::{ApiError, ApiJson};
pub use failures::{DEFAULT_SIGN_IN_FAILURES_PER_MINUTE, FailureLimits, MAX_WAIT};
pub use headers::{HSTS_MAX_AGE_SELF_SIGNED, HSTS_MAX_AGE_TRUSTED};
pub use limits::{
    DEFAULT_REQUESTS_PER_MINUTE, DEFAULT_SIGN_IN_ATTEMPTS_PER_MINUTE, Limits, LimitsConfig,
};
pub use listener::{
    Accepted, ClientAddr, Connection, DEFAULT_MAX_CONNECTIONS, Incoming, ListenerOptions,
};
pub use router::{Options, router, serve};
pub use web::WebSource;

// Used by the integration tests in `tests/` only (`unused_crate_dependencies`).
#[cfg(test)]
use h2 as _;
#[cfg(test)]
use tracing_subscriber as _;
