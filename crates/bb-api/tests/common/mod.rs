//! The required security headers (SEC-11), written out once for the integration tests.

/// Every response carries these.
pub(crate) const EXPECTED: [(&str, &str); 6] = [
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

/// Added when the connection is HTTPS.
pub(crate) const HSTS: (&str, &str) = (
    "strict-transport-security",
    "max-age=63072000; includeSubDomains",
);
