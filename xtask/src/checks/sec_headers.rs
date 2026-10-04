//! SEC-HEADERS: the server sends the required security headers and CSP (SEC-11).
//!
//! The running behaviour is proven by the integration tests `crates/bb-api/tests/headers.rs` and
//! `https.rs` (they run with `cargo test`, which `cargo xtask ci` runs). This check keeps the code
//! and those tests from drifting from the requirement:
//!
//! * every header of the table in `headers.rs` is written out again in the tests with exactly
//!   the same value (compared as text, not searched for),
//! * HSTS exists in both and never with `includeSubDomains`,
//! * the CSP says what SEC-11 demands and has no source that lets injected or foreign code run.

use std::collections::BTreeMap;

use super::{Check, Context, Finding};
use crate::docs::source::read_text;

const SOURCE: &str = "crates/bb-api/src/headers.rs";
const EXPECTED_IN_TESTS: &str = "crates/bb-api/tests/common/mod.rs";

/// Headers every response carries (SEC-11, architecture §7).
const REQUIRED: [&str; 6] = [
    "content-security-policy",
    "x-content-type-options",
    "referrer-policy",
    "cross-origin-opener-policy",
    "cross-origin-resource-policy",
    "permissions-policy",
];
/// Sent on HTTPS only, so it is not in the table of the headers every response carries.
const HSTS: &str = "strict-transport-security";
/// HSTS must not reach other names of the host.
const HSTS_FORBIDDEN: &str = "includesubdomains";
/// Directives the policy must have, with the exact sources they must have.
const CSP_MUST_HAVE: [(&str, &[&str]); 3] = [
    ("default-src", &["'self'"]),
    ("frame-ancestors", &["'none'"]),
    ("require-trusted-types-for", &["'script'"]),
];
/// Sources that let injected code run or load code and content from elsewhere.
const CSP_FORBIDDEN: [&str; 6] = [
    "'unsafe-inline'",
    "'unsafe-eval'",
    "'unsafe-hashes'",
    "*",
    "http:",
    "https:",
];
/// `data:` and `blob:` can carry script. Only images may use `data:` (architecture §7).
const CSP_DATA_ONLY_IN: &str = "img-src";

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

/// The text of every string literal outside comments, in order. Escapes are resolved and a
/// backslash at the end of a line joins it with the next, as Rust does.
fn string_literals(source: &str) -> Vec<String> {
    let mut literals = Vec::new();
    let mut chars = source.chars().peekable();
    let mut current: Option<String> = None;
    while let Some(c) = chars.next() {
        match (&mut current, c) {
            (None, '/') if chars.peek() == Some(&'/') => {
                for skipped in chars.by_ref() {
                    if skipped == '\n' {
                        break;
                    }
                }
            }
            (None, '"') => current = Some(String::new()),
            (Some(text), '"') => {
                literals.push(std::mem::take(text));
                current = None;
            }
            (Some(text), '\\') => match chars.next() {
                Some('\n') => {
                    while chars.peek().is_some_and(|next| next.is_whitespace()) {
                        chars.next();
                    }
                }
                Some('n') => text.push('\n'),
                Some(other) => text.push(other),
                None => {}
            },
            (Some(text), other) => text.push(other),
            (None, _) => {}
        }
    }
    literals
}

/// Header name -> value, for every name in `names` that is directly followed by a literal.
fn values(literals: &[String], names: &[&str]) -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    for (at, literal) in literals.iter().enumerate() {
        if let Some(value) = literals.get(at + 1)
            && names.contains(&literal.as_str())
        {
            found.insert(literal.clone(), value.clone());
        }
    }
    found
}

fn evaluate(code: &str, tests: &str) -> Vec<Finding> {
    let in_code = string_literals(code);
    let in_tests = string_literals(tests);
    let code_values = values(&in_code, &REQUIRED);
    let test_values = values(&in_tests, &REQUIRED);
    let mut findings = Vec::new();
    for name in REQUIRED {
        match (code_values.get(name), test_values.get(name)) {
            (None, _) => findings.push(Finding::at(
                SOURCE,
                format!("The header `{name}` is required by SEC-11 but not set."),
            )),
            (Some(_), None) => findings.push(Finding::at(
                EXPECTED_IN_TESTS,
                format!("The integration tests do not expect the header `{name}`; add it."),
            )),
            (Some(set), Some(expected)) if set != expected => findings.push(Finding::at(
                EXPECTED_IN_TESTS,
                format!(
                    "The value of `{name}` in {SOURCE} differs from the one the tests expect. \
                     Change both together, and only to match SEC-11."
                ),
            )),
            _ => {}
        }
    }
    for (place, literals) in [(SOURCE, &in_code), (EXPECTED_IN_TESTS, &in_tests)] {
        if !literals.iter().any(|literal| literal == HSTS) {
            findings.push(Finding::at(
                place,
                format!("The header `{HSTS}` (HSTS) is required on HTTPS but is not there."),
            ));
        }
        if literals
            .iter()
            .any(|literal| literal.to_ascii_lowercase().contains(HSTS_FORBIDDEN))
        {
            findings.push(Finding::at(
                place,
                "HSTS must not use includeSubDomains: Bugaboo does not own the other names of its host.",
            ));
        }
    }
    if let Some(policy) = code_values.get("content-security-policy") {
        findings.extend(check_policy(policy));
    }
    findings
}

fn check_policy(policy: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let directives: Vec<(String, Vec<String>)> = policy
        .split(';')
        .filter_map(|directive| {
            let mut parts = directive.split_whitespace().map(str::to_ascii_lowercase);
            parts.next().map(|name| (name, parts.collect()))
        })
        .collect();
    for (name, sources) in CSP_MUST_HAVE {
        let has = directives
            .iter()
            .any(|(have, tokens)| have == name && tokens.iter().eq(sources.iter()));
        if !has {
            findings.push(Finding::at(
                SOURCE,
                format!(
                    "The policy must contain `{name} {}` (SEC-11).",
                    sources.join(" ")
                ),
            ));
        }
    }
    for (name, tokens) in &directives {
        for token in tokens {
            let data = token == "data:" && name != CSP_DATA_ONLY_IN;
            if CSP_FORBIDDEN.contains(&token.as_str()) || data || token == "blob:" {
                findings.push(Finding::at(
                    SOURCE,
                    format!(
                        "The policy has `{token}` in `{name}`: it lets injected or foreign code run (SEC-11)."
                    ),
                ));
            }
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    const POLICY: &str = "default-src 'self'; script-src 'self'; img-src 'self' data:; \
                          frame-ancestors 'none'; require-trusted-types-for 'script'";

    /// A table in the shape of `headers.rs`.
    fn table(policy: &str, referrer: &str) -> String {
        format!(
            r#"
// "includeSubDomains" is mentioned in a comment only
const T: [(&str, &str); 6] = [
    ("content-security-policy", "{policy_line}"),
    ("x-content-type-options", "nosniff"),
    ("referrer-policy", "{referrer}"),
    ("cross-origin-opener-policy", "same-origin"),
    ("cross-origin-resource-policy", "same-origin"),
    ("permissions-policy", "camera=()"),
];
const H: &str = "strict-transport-security";
"#,
            policy_line = policy.replace("; ", "; \\\n        ")
        )
    }

    #[test]
    fn good_example() {
        let code = table(POLICY, "same-origin");
        // The same values, written differently (line breaks) and found by exact comparison.
        let tests = table(POLICY, "same-origin").replace("\\\n        ", "");
        assert!(evaluate(&code, &tests).is_empty(), "{}", {
            evaluate(&code, &tests)
                .iter()
                .map(|f| f.message.clone())
                .collect::<Vec<_>>()
                .join("\n")
        });
    }

    #[test]
    fn bad_example_values_differ_headers_missing_and_weak_policy() {
        let code = table(POLICY, "same-origin");
        let different = table(POLICY, "no-referrer");
        assert_eq!(evaluate(&code, &different).len(), 1, "value differs");

        let missing_in_code = code.replace("\"permissions-policy\"", "\"x\"");
        assert_eq!(evaluate(&missing_in_code, &code).len(), 1);
        let missing_in_tests = code.replace("\"permissions-policy\"", "\"x\"");
        assert_eq!(evaluate(&code, &missing_in_tests).len(), 1);

        let no_hsts = code.replace("strict-transport-security", "x");
        assert_eq!(evaluate(&no_hsts, &code).len(), 1);
        let subdomains = format!("{code}\nconst S: &str = \"max-age=1; includeSubDomains\";");
        assert_eq!(evaluate(&subdomains, &code).len(), 1);

        for weak in [
            "script-src 'self' 'unsafe-inline'",
            "script-src 'self' 'unsafe-hashes'",
            "script-src *",
            "script-src https:",
            "script-src 'self' data:",
            "style-src blob:",
        ] {
            let policy = format!("{POLICY}; {weak}");
            let weak_code = table(&policy, "same-origin");
            let weak_tests = table(&policy, "same-origin");
            assert_eq!(evaluate(&weak_code, &weak_tests).len(), 1, "{weak}");
        }
        let lax = POLICY.replace("frame-ancestors 'none'", "frame-ancestors 'self'");
        let lax_code = table(&lax, "same-origin");
        assert_eq!(evaluate(&lax_code, &lax_code).len(), 1);
    }

    #[test]
    fn literals_join_continued_lines_and_skip_comments() {
        let source = "// \"no\"\nlet a = \"one \\\n      two\"; let b = \"x\\\"y\";";
        assert_eq!(string_literals(source), ["one two", "x\"y"]);
    }
}
