//! HTTP: routes, OpenAPI, security headers, rate limits and the embedded browser app (ADR-0008).
//!
//! For now: the health endpoint, the browser app (embedded in release builds, read from disk in
//! debug builds or from a folder) and the security headers on every response (SEC-11).

#![forbid(unsafe_code)]

mod headers;
mod router;
mod web;

pub use router::{router, serve};
pub use web::{WEB_APP_MISSING, WebSource};
