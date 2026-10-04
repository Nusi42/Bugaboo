//! SEC-HEADERS: the server sends the required security headers and CSP (SEC-11).
//!
//! The running behaviour is proven by the integration tests `crates/bb-api/tests/headers.rs` and
//! `https.rs` (they run with `cargo test`, which `cargo xtask ci` runs). This check keeps the code
//! and those tests from drifting from the requirement: every required header is defined in the
//! header table and written out again in the tests, and the CSP has what SEC-11 demands and
//! nothing that weakens it.

use super::{Check, Context, Finding};
use crate::docs::source::read_text;

const SOURCE: &str = "crates/bb-api/src/headers.rs";
const EXPECTED_IN_TESTS: &str = "crates/bb-api/tests/common/mod.rs";

/// Headers every response carries, or every HTTPS response (SEC-11, architecture §7).
const REQUIRED: [&str; 7] = [
    "content-security-policy",
    "x-content-type-options",
    "referrer-policy",
    "cross-origin-opener-policy",
    "cross-origin-resource-policy",
    "permissions-policy",
    "strict-transport-security",
];
/// What the header table must say.
const MUST_HAVE: [&str; 4] = [
    "default-src 'self'",
    "frame-ancestors 'none'",
    "require-trusted-types-for 'script'",
    "nosniff",
];
/// What would let injected code run, or load code from elsewhere.
const MUST_NOT_HAVE: [&str; 3] = ["'unsafe-inline'", "'unsafe-eval'", "script-src *"];

pub(crate) struct SecHeaders;

impl Check for SecHeaders {
    fn id(&self) -> &'static str {
        "SEC-HEADERS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let code = read_text(context.repo, SOURCE)?;
        let tests = read_text(context.repo, EXPECTED_IN_TESTS)?;
        Ok(evaluate(&code, &tests))
    }
}

fn quoted(name: &str) -> String {
    format!("\"{name}\"")
}

fn evaluate(code: &str, tests: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    for name in REQUIRED {
        if !code.contains(&quoted(name)) {
            findings.push(Finding::at(
                SOURCE,
                format!("The header `{name}` is required by SEC-11 but not set."),
            ));
        }
        if !tests.contains(&quoted(name)) {
            findings.push(Finding::at(
                EXPECTED_IN_TESTS,
                format!("The integration tests do not expect the header `{name}`; add it."),
            ));
        }
    }
    for part in MUST_HAVE {
        if !code.contains(part) {
            findings.push(Finding::at(
                SOURCE,
                format!("The headers must contain `{part}` (SEC-11)."),
            ));
        }
    }
    for part in MUST_NOT_HAVE {
        if code.contains(part) {
            findings.push(Finding::at(
                SOURCE,
                format!("The policy must not contain `{part}`: it allows injected code (SEC-11)."),
            ));
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complete() -> String {
        let mut text = String::from(
            "default-src 'self'; frame-ancestors 'none'; require-trusted-types-for 'script'; nosniff\n",
        );
        for name in REQUIRED {
            text.push_str(&quoted(name));
            text.push('\n');
        }
        text
    }

    #[test]
    fn good_example() {
        let all = complete();
        assert!(evaluate(&all, &all).is_empty());
    }

    #[test]
    fn bad_example_missing_header_and_weak_policy() {
        let without = complete().replace(&quoted("strict-transport-security"), "");
        let weak = format!("{without} script-src 'unsafe-inline'");
        let findings = evaluate(&weak, &complete());
        assert_eq!(findings.len(), 2, "missing HSTS and unsafe-inline");
        let untested = evaluate(&complete(), &without);
        assert_eq!(untested.len(), 1);
    }
}
