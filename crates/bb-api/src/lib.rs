//! HTTP: routes, OpenAPI, security headers, rate limits and the embedded browser app (ADR-0008).
//!
//! For now: the health endpoints, the browser app (embedded in release builds, read from disk in
//! debug builds or from a folder), HTTPS, the security headers on every response (SEC-10,
//! SEC-11), response compression and one log line per request without personal data (OPS-08).

#![forbid(unsafe_code)]

mod headers;
mod listener;
mod logging;
mod router;
pub mod tls;
mod web;

pub use listener::{ClientAddr, Connection, Incoming, ListenerOptions};
pub use router::{Options, router, serve};
pub use web::{WEB_APP_MISSING, WebSource};

// Used by the integration tests in `tests/` only (`unused_crate_dependencies`).
#[cfg(test)]
use tracing_subscriber as _;
