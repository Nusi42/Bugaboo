//! Security headers on every response (SEC-11, architecture §7 "Browser protections").
//!
//! The values are defined once, here. The integration tests in `tests/` write them out again on
//! purpose, so changing a header without changing the requirement fails (SEC-HEADERS).

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

/// Sent only when browsers reach us over HTTPS (here, or at a proxy we trust). Never with
/// `includeSubDomains`: Bugaboo does not own the other names of its host.
const HSTS_NAME: &str = "strict-transport-security";

/// With a certificate Bugaboo made itself: one day. A person who trusts that certificate on
/// `localhost` must not pin every other local tool to HTTPS for a long time.
pub const HSTS_MAX_AGE_SELF_SIGNED: u64 = 86_400;

/// With a certificate the owner provided, or HTTPS at a trusted proxy: one year.
pub const HSTS_MAX_AGE_TRUSTED: u64 = 31_536_000;

/// Wraps the router so every response, including errors and fallbacks, carries the headers.
/// `hsts_max_age` is the HSTS lifetime in seconds, or `None` when the connection is not HTTPS.
pub(crate) fn apply(router: Router, hsts_max_age: Option<u64>) -> Router {
    let hsts = hsts_max_age.map(|seconds| (HSTS_NAME, format!("max-age={seconds}")));
    let fixed = SECURITY_HEADERS
        .iter()
        .map(|&(name, value)| (name, value.to_owned()));
    fixed.chain(hsts).fold(router, |router, (name, value)| {
        match HeaderValue::from_str(&value) {
            Ok(value) => router.layer(SetResponseHeaderLayer::overriding(
                HeaderName::from_static(name),
                value,
            )),
            // Every value is ASCII text written above; an invalid one would be a coding
            // mistake that the header tests catch.
            Err(_) => router,
        }
    })
}
