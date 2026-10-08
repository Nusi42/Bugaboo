//! DATA-LOGS: no names, e-mail addresses or free text in log statements (PRIV-01,
//! architecture §7 "Observability").
//!
//! Every `tracing` macro in the product code is read. It fails when a field or a value in it
//! carries a name that stands for personal data or free text (see [`PERSONAL`]), either as a
//! field (`email = %x`), as a value (`user.name`) or as a captured name in the message
//! (`"{email}"`). IDs, counts, codes and shortened networks are what logs may hold. Test-only
//! items (`#[cfg(test)]` and the item after it) and files in `tests/` are not checked.

use super::{Check, Context, Finding};
use crate::rust_source::{strip_comments, without_tests};
use crate::source::{Kind, is_word_char, word_positions};

/// Names (lower case) that must not appear in a log statement.
const PERSONAL: [&str; 35] = [
    "name",
    "names",
    "username",
    "user_name",
    "display_name",
    "email",
    "e_mail",
    "mail",
    "phone",
    "password",
    "passphrase",
    "secret",
    "token",
    "cookie",
    "authorization",
    "title",
    "summary",
    "description",
    "comment",
    "text",
    "content",
    "body",
    "payload",
    "query",
    "headers",
    "path",
    "uri",
    "url",
    "ip",
    "addr",
    "address",
    "peer",
    "remote",
    "client",
    "host",
];
/// Macros that write log events or spans.
const MACROS: [&str; 10] = [
    "trace",
    "debug",
    "info",
    "warn",
    "error",
    "event",
    "span",
    "info_span",
    "debug_span",
    "warn_span",
];
pub(crate) struct DataLogs;

impl Check for DataLogs {
    fn id(&self) -> &'static str {
        "DATA-LOGS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let mut findings = Vec::new();
        for (file, text) in context.sources(self.id(), Kind::Rust)? {
            findings.extend(scan(&file, &text));
        }
        Ok(findings)
    }
}

/// Every `name!(…)` log call: the byte position of the macro name and the text between its
/// brackets.
fn log_calls(code: &str) -> Vec<(usize, &str)> {
    let mut calls = Vec::new();
    for name in MACROS {
        for at in word_positions(code, name) {
            let rest = &code[at + name.len()..];
            let Some(open) = rest
                .strip_prefix('!')
                .and_then(|after| after.chars().next())
                .filter(|c| matches!(c, '(' | '[' | '{'))
            else {
                continue;
            };
            if let Some(body) = balanced(&rest[2..], open) {
                calls.push((at, body));
            }
        }
    }
    calls
}

/// The text up to the bracket that closes `open`, skipping strings.
fn balanced(text: &str, open: char) -> Option<&str> {
    let close = match open {
        '(' => ')',
        '[' => ']',
        _ => '}',
    };
    let mut depth = 1_usize;
    let mut in_string = false;
    let mut escaped = false;
    for (at, c) in text.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
        } else if c == '"' {
            in_string = true;
        } else if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                return Some(&text[..at]);
            }
        }
    }
    None
}

fn personal(word: &str) -> Option<&'static str> {
    let lower = word.to_ascii_lowercase();
    PERSONAL.iter().find(|name| **name == lower).copied()
}

/// The personal names a call mentions: as words outside strings, and as `{name…}` captures
/// inside strings.
fn personal_names(body: &str) -> Vec<&'static str> {
    let mut found = Vec::new();
    let mut word = String::new();
    let mut literal = String::new();
    let mut in_string = false;
    let mut escaped = false;
    for c in body.chars().chain(std::iter::once(' ')) {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
                found.extend(captured(&literal));
                literal.clear();
            } else {
                literal.push(c);
            }
            continue;
        }
        if is_word_char(c) {
            word.push(c);
            continue;
        }
        found.extend(personal(&word));
        word.clear();
        in_string = c == '"';
    }
    found
}

/// Names captured in a format string: `{email}`, `{name:?}`, `{user.name}`.
fn captured(literal: &str) -> Vec<&'static str> {
    literal
        .split('{')
        .skip(1)
        .filter_map(|part| part.split('}').next())
        .flat_map(|inner| {
            inner
                .split(':')
                .next()
                .unwrap_or_default()
                .split(|c: char| !is_word_char(c))
                .filter_map(personal)
                .collect::<Vec<_>>()
        })
        .collect()
}

fn scan(file: &str, text: &str) -> Vec<Finding> {
    // Strings stay (captured names are checked); comments and test-only items are gone.
    let code = strip_comments(&without_tests(text));
    let mut findings = Vec::new();
    for (at, body) in log_calls(&code) {
        let line = code[..at].matches('\n').count() + 1;
        for name in personal_names(body) {
            findings.push(Finding::at(
                format!("{file}:{line}"),
                format!(
                    "A log statement mentions `{name}`. Logs hold IDs, counts and codes, not names, \
                     e-mail addresses, paths or free text (PRIV-01). Log an ID instead."
                ),
            ));
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn good_example() {
        let code = r#"
fn run() {
    // info!(email = %x) in a comment is ignored
    tracing::info!(request_id = %id, status = 200, net = %shown, "request {id}");
    tracing::warn!(error = %error, "accepting a connection failed");
}
#[cfg(test)]
mod tests {
    fn t() { tracing::info!(email = "a@b.test"); }
}
"#;
        assert!(scan("crates/x/src/lib.rs", code).is_empty());
    }

    #[test]
    fn bad_example_field_value_and_captured_name() {
        let field = r#"fn run() { tracing::info!(email = %user.email, "signed in"); }"#;
        assert_eq!(scan("crates/x/src/lib.rs", field).len(), 2);
        let value = "fn run() { info!(who = %person.name); }";
        assert_eq!(scan("crates/x/src/lib.rs", value).len(), 1);
        let inline = r#"fn run() { warn!("could not open {path:?}"); }"#;
        assert_eq!(scan("crates/x/src/lib.rs", inline).len(), 1);
        let multi = "fn run() {\n    error!(\n        title,\n    );\n}";
        let findings = scan("crates/x/src/lib.rs", multi);
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].location.as_deref(),
            Some("crates/x/src/lib.rs:2")
        );
    }

    #[test]
    fn code_after_a_test_module_is_still_scanned() {
        let code =
            "#[cfg(test)]\nmod tests { fn t() { info!(email); } }\nfn run() { info!(email); }\n";
        let findings = scan("crates/x/src/lib.rs", code);
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].location.as_deref(),
            Some("crates/x/src/lib.rs:3")
        );
        let one_item = "#[cfg(test)]\nuse std::io;\nfn run() { warn!(peer = 1); }";
        assert_eq!(scan("crates/x/src/lib.rs", one_item).len(), 1);
    }

    #[test]
    fn raw_strings_and_char_literals_do_not_hide_or_confuse() {
        let raw = r##"fn run() { info!(r#"signed in {email}"#); }"##;
        assert_eq!(scan("crates/x/src/lib.rs", raw).len(), 1);
        let quote = "fn run() { let q = '\"'; info!(host = 1); }";
        assert_eq!(scan("crates/x/src/lib.rs", quote).len(), 1);
        let clean = "fn run<'a>(x: &'a str) { info!(request_id = 1); let c = '{'; }";
        assert!(scan("crates/x/src/lib.rs", clean).is_empty());
    }

    #[test]
    fn network_names_are_personal() {
        for name in ["addr", "address", "peer", "remote", "client", "host"] {
            let code = format!("fn run() {{ info!({name} = 1); }}");
            assert_eq!(scan("crates/x/src/lib.rs", &code).len(), 1, "{name}");
        }
    }

    #[test]
    fn similar_words_and_other_macros_are_not_flagged() {
        let code = r#"fn run() { tracing::info!(request_id = %id, names_count = 3); println!("{name}"); format!("{text}"); }"#;
        assert!(scan("crates/x/src/lib.rs", code).is_empty());
    }
}
