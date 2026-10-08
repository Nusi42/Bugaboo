//! SEC-HEADERS: the server sends the required security headers and CSP (SEC-11).
//!
//! The running behaviour is proven by the integration tests `crates/bb-api/tests/headers.rs` and
//! `https.rs` (they run with `cargo test`, which `cargo xtask ci` runs); they read the values from
//! the one table in `headers.rs`, so the table is not written out a second time. This check holds
//! the table against the requirement:
//!
//! * every header SEC-11 requires is in the table with a value that is allowed for it (`nosniff`;
//!   a referrer policy that sends no address to other sites; `same-origin` for COOP and CORP; a
//!   permissions policy that turns off camera, microphone and location),
//! * HSTS exists, never with `includeSubDomains`, and its lifetimes stay within one day for a
//!   certificate Bugaboo made and one year for a provided one (architecture §7),
//! * the CSP says what SEC-11 demands and has no source that lets injected or foreign code run.

use std::collections::BTreeMap;

use super::{Check, Context, Finding};
use crate::source::read_text;

const SOURCE: &str = "crates/bb-api/src/headers.rs";

/// Headers every response carries (SEC-11, architecture §7).
const REQUIRED: [&str; 6] = [
    "content-security-policy",
    "x-content-type-options",
    "referrer-policy",
    "cross-origin-opener-policy",
    "cross-origin-resource-policy",
    "permissions-policy",
];
/// The values a header may have, besides which it is not what SEC-11 demands (architecture §7).
const ALLOWED_VALUES: [(&str, &[&str]); 4] = [
    ("x-content-type-options", &["nosniff"]),
    (
        "referrer-policy",
        &[
            "no-referrer",
            "same-origin",
            "strict-origin",
            "strict-origin-when-cross-origin",
        ],
    ),
    ("cross-origin-opener-policy", &["same-origin"]),
    ("cross-origin-resource-policy", &["same-origin"]),
];
/// Features the permissions policy must turn off for the page and everything in it.
const PERMISSIONS_OFF: [&str; 3] = ["camera", "microphone", "geolocation"];
/// The lifetime of HSTS for a certificate Bugaboo made itself: at most one day.
const HSTS_SELF_SIGNED: (&str, u64) = ("HSTS_MAX_AGE_SELF_SIGNED", 86_400);
/// The lifetime of HSTS for a provided certificate or a trusted proxy: at most one year.
const HSTS_TRUSTED: (&str, u64) = ("HSTS_MAX_AGE_TRUSTED", 31_536_000);
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
        Ok(evaluate(&code))
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

fn evaluate(code: &str) -> Vec<Finding> {
    let in_code = string_literals(code);
    let code_values = values(&in_code, &REQUIRED);
    let mut findings = Vec::new();
    for name in REQUIRED {
        if !code_values.contains_key(name) {
            findings.push(Finding::at(
                SOURCE,
                format!("The header `{name}` is required by SEC-11 but not set."),
            ));
        }
    }
    if !in_code.iter().any(|literal| literal == HSTS) {
        findings.push(Finding::at(
            SOURCE,
            format!("The header `{HSTS}` (HSTS) is required on HTTPS but is not there."),
        ));
    }
    if in_code
        .iter()
        .any(|literal| literal.to_ascii_lowercase().contains(HSTS_FORBIDDEN))
    {
        findings.push(Finding::at(
            SOURCE,
            "HSTS must not use includeSubDomains: Bugaboo does not own the other names of its host.",
        ));
    }
    findings.extend(check_values(&code_values));
    findings.extend(check_lifetimes(code));
    if let Some(policy) = code_values.get("content-security-policy") {
        findings.extend(check_policy(policy));
    }
    findings
}

/// The headers whose value must be one of a few, and the permissions policy.
fn check_values(values: &BTreeMap<String, String>) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (name, allowed) in ALLOWED_VALUES {
        if let Some(value) = values.get(name)
            && !allowed.contains(&value.trim().to_ascii_lowercase().as_str())
        {
            findings.push(Finding::at(
                SOURCE,
                format!(
                    "The header `{name}` is `{value}`; SEC-11 allows only {}.",
                    allowed.join(", ")
                ),
            ));
        }
    }
    if let Some(policy) = values.get("permissions-policy") {
        for feature in PERMISSIONS_OFF {
            let off = policy.split(',').any(|directive| {
                directive.split_whitespace().collect::<String>() == format!("{feature}=()")
            });
            if !off {
                findings.push(Finding::at(
                    SOURCE,
                    format!("The permissions policy must turn off `{feature}` (`{feature}=()`)."),
                ));
            }
        }
    }
    findings
}

/// The number a constant of the table is set to, from its line `const NAME: u64 = 86_400;`.
fn constant_value(code: &str, name: &str) -> Option<u64> {
    let after = code.split_once(&format!("const {name}"))?.1;
    let number = after.split_once('=')?.1.split_once(';')?.0;
    number.trim().replace('_', "").parse().ok()
}

/// HSTS lifetimes: a self-made certificate gets at most a day, a provided one more than that and
/// at most a year.
fn check_lifetimes(code: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let own = constant_value(code, HSTS_SELF_SIGNED.0);
    let trusted = constant_value(code, HSTS_TRUSTED.0);
    for (name, value, within) in [
        (HSTS_SELF_SIGNED.0, own, 1..=HSTS_SELF_SIGNED.1),
        (
            HSTS_TRUSTED.0,
            trusted,
            own.unwrap_or(0) + 1..=HSTS_TRUSTED.1,
        ),
    ] {
        if !value.is_some_and(|seconds| within.contains(&seconds)) {
            findings.push(Finding::at(
                SOURCE,
                format!(
                    "`{name}` must be a number of seconds from {} to {} (architecture §7: one day for a certificate Bugaboo made, one year for a provided one).",
                    within.start(),
                    within.end()
                ),
            ));
        }
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

    /// The values of a table in the shape of `headers.rs`; the default is a good one.
    struct Table {
        policy: &'static str,
        nosniff: &'static str,
        referrer: &'static str,
        opener: &'static str,
        resource: &'static str,
        permissions: &'static str,
        own: &'static str,
        trusted: &'static str,
    }

    impl Table {
        const GOOD: Self = Self {
            policy: POLICY,
            nosniff: "nosniff",
            referrer: "same-origin",
            opener: "same-origin",
            resource: "same-origin",
            permissions: "camera=(), microphone=(), geolocation=(), payment=()",
            own: "86_400",
            trusted: "31_536_000",
        };

        fn code(&self) -> String {
            let policy_line = self.policy.replace("; ", "; \\\n        ");
            format!(
                r#"
// "includeSubDomains" is mentioned in a comment only
const T: [(&str, &str); 6] = [
    ("content-security-policy", "{policy_line}"),
    ("x-content-type-options", "{}"),
    ("referrer-policy", "{}"),
    ("cross-origin-opener-policy", "{}"),
    ("cross-origin-resource-policy", "{}"),
    ("permissions-policy", "{}"),
];
const H: &str = "strict-transport-security";
pub const HSTS_MAX_AGE_SELF_SIGNED: u64 = {};
pub const HSTS_MAX_AGE_TRUSTED: u64 = {};
"#,
                self.nosniff,
                self.referrer,
                self.opener,
                self.resource,
                self.permissions,
                self.own,
                self.trusted
            )
        }
    }

    fn messages(code: &str) -> Vec<String> {
        evaluate(code).into_iter().map(|f| f.message).collect()
    }

    #[test]
    fn good_example() {
        let found = messages(&Table::GOOD.code());
        assert!(found.is_empty(), "{}", found.join("\n"));
    }

    #[test]
    fn bad_example_headers_missing_and_weak_policy() {
        let code = Table::GOOD.code();
        let missing = code.replace("\"permissions-policy\"", "\"x\"");
        assert_eq!(evaluate(&missing).len(), 1);

        let no_hsts = code.replace("strict-transport-security", "x");
        assert_eq!(evaluate(&no_hsts).len(), 1);
        let subdomains = format!("{code}\nconst S: &str = \"max-age=1; includeSubDomains\";");
        assert_eq!(evaluate(&subdomains).len(), 1);

        for weak in [
            "script-src 'self' 'unsafe-inline'",
            "script-src 'self' 'unsafe-hashes'",
            "script-src *",
            "script-src https:",
            "script-src 'self' data:",
            "style-src blob:",
        ] {
            let policy = format!("{POLICY}; {weak}");
            let weak_code = Table {
                policy: Box::leak(policy.into_boxed_str()),
                ..Table::GOOD
            }
            .code();
            assert_eq!(evaluate(&weak_code).len(), 1, "{weak}");
        }
        let lax = POLICY.replace("frame-ancestors 'none'", "frame-ancestors 'self'");
        let lax_code = Table {
            policy: Box::leak(lax.into_boxed_str()),
            ..Table::GOOD
        }
        .code();
        assert_eq!(evaluate(&lax_code).len(), 1);
    }

    #[test]
    fn bad_example_a_header_with_a_value_that_sec_11_does_not_allow() {
        let cases = [
            (
                Table {
                    nosniff: "sniff",
                    ..Table::GOOD
                },
                "x-content-type-options",
            ),
            (
                Table {
                    referrer: "unsafe-url",
                    ..Table::GOOD
                },
                "referrer-policy",
            ),
            (
                Table {
                    opener: "unsafe-none",
                    ..Table::GOOD
                },
                "cross-origin-opener-policy",
            ),
            (
                Table {
                    resource: "cross-origin",
                    ..Table::GOOD
                },
                "cross-origin-resource-policy",
            ),
        ];
        for (table, header) in cases {
            let found = messages(&table.code());
            assert_eq!(found.len(), 1, "{header}: {found:?}");
            assert!(found[0].contains(header), "{found:?}");
        }
    }

    #[test]
    fn bad_example_a_permissions_policy_that_leaves_a_feature_on() {
        for feature in PERMISSIONS_OFF {
            let policy = Table::GOOD
                .permissions
                .replace(&format!("{feature}=()"), &format!("{feature}=(self)"));
            let found = messages(
                &Table {
                    permissions: Box::leak(policy.into_boxed_str()),
                    ..Table::GOOD
                }
                .code(),
            );
            assert_eq!(found.len(), 1, "{feature}: {found:?}");
            assert!(found[0].contains(feature), "{found:?}");
        }
    }

    #[test]
    fn bad_example_hsts_lifetimes_outside_one_day_and_one_year() {
        for table in [
            Table {
                own: "172_800",
                ..Table::GOOD
            },
            Table {
                own: "0",
                ..Table::GOOD
            },
            Table {
                trusted: "63_072_000",
                ..Table::GOOD
            },
            Table {
                trusted: "3_600",
                ..Table::GOOD
            },
        ] {
            let found = messages(&table.code());
            assert_eq!(found.len(), 1, "{found:?}");
            assert!(found[0].contains("HSTS_MAX_AGE"), "{found:?}");
        }
        let gone = Table::GOOD.code().replace("HSTS_MAX_AGE_TRUSTED", "OTHER");
        assert_eq!(evaluate(&gone).len(), 1);
    }

    #[test]
    fn literals_join_continued_lines_and_skip_comments() {
        let source = "// \"no\"\nlet a = \"one \\\n      two\"; let b = \"x\\\"y\";";
        assert_eq!(string_literals(source), ["one two", "x\"y"]);
    }
}
