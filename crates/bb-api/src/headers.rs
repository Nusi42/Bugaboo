//! Security headers on every response (SEC-11, architecture §7 "Browser protections").
//!
//! The values are defined once, here. The integration test `tests/headers.rs` writes them out
//! again on purpose, so changing a header without changing the requirement fails (SEC-HEADERS).

use axum::Router;
use axum::http::{HeaderName, HeaderValue};
use tower_http::set_header::SetResponseHeaderLayer;

/// Headers every response carries.
const SECURITY_HEADERS: [(&str, &str); 6] = [
    (
        "content-security-policy",
        "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; \
         font-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; \
         form-action 'self'; require-trusted-types-for 'script'",
    ),
    ("x-content-type-options", "nosniff"),
    ("referrer-policy", "same-origin"),
    ("cross-origin-opener-policy", "same-origin"),
    ("cross-origin-resource-policy", "same-origin"),
    (
        "permissions-policy",
        "accelerometer=(), camera=(), display-capture=(), geolocation=(), gyroscope=(), \
         magnetometer=(), microphone=(), payment=(), usb=(), browsing-topics=()",
    ),
];

/// Only sent when the browser reaches us over HTTPS: two years, subdomains included.
pub(crate) const HSTS: (&str, &str) = (
    "strict-transport-security",
    "max-age=63072000; includeSubDomains",
);

/// Wraps the router so every response, including errors and fallbacks, carries the headers.
/// `https` adds HSTS (SEC-10: the connection is HTTPS, here or at a trusted proxy).
pub(crate) fn apply(router: Router, https: bool) -> Router {
    let hsts = https.then_some(HSTS);
    SECURITY_HEADERS
        .iter()
        .copied()
        .chain(hsts)
        .fold(router, |router, (name, value)| {
            router.layer(SetResponseHeaderLayer::overriding(
                HeaderName::from_static(name),
                HeaderValue::from_static(value),
            ))
        })
}
