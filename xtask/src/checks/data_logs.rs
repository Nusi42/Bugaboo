//! DATA-LOGS: no names, e-mail addresses or free text in log statements (PRIV-01,
//! architecture §7 "Observability").
//!
//! Every `tracing` macro in the product code is read. It fails when a field or a value in it
//! carries a name that stands for personal data or free text (see [`PERSONAL`]), either as a
//! field (`email = %x`), as a value (`user.name`) or as a captured name in the message
//! (`"{email}"`). IDs, counts, codes and shortened networks are what logs may hold. Test-only
//! items (`#[cfg(test)]` and the item after it) and files in `tests/` are not checked.

use std::path::Path;

use super::{Check, Context, Finding};

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
/// The attribute of test-only items; the item that follows it is not scanned.
const TEST_MARKER: &str = "#[cfg(test)]";

pub(crate) struct DataLogs;

impl Check for DataLogs {
    fn id(&self) -> &'static str {
        "DATA-LOGS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let mut findings = Vec::new();
        for file in context.repo.all_files()? {
            let rust = Path::new(&file)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"));
            let product = file.starts_with("crates/") && rust && !file.contains("/tests/");
            if product {
                let text = std::fs::read_to_string(context.repo.path(&file))?;
                findings.extend(scan(&file, &text));
            }
        }
        Ok(findings)
    }
}

/// The text as the scan reads it: comments, char literals and test-only items (`#[cfg(test)]`
/// and the one item that follows) are blanked out, line breaks stay so line numbers are right.
/// Strings stay (captured names are checked); raw strings become ordinary strings.
fn code_of(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    let blank = |out: &mut String, from: usize, to: usize| {
        for &c in &chars[from..to] {
            out.push(if c == '\n' { '\n' } else { ' ' });
        }
    };
    while index < chars.len() {
        let c = chars[index];
        let next = chars.get(index + 1).copied();
        if c == '/' && next == Some('/') {
            let end = chars[index..]
                .iter()
                .position(|c| *c == '\n')
                .map_or(chars.len(), |at| index + at);
            blank(&mut out, index, end);
            index = end;
        } else if c == '/' && next == Some('*') {
            let end = block_comment_end(&chars, index);
            blank(&mut out, index, end);
            index = end;
        } else if starts_with(&chars, index, TEST_MARKER) {
            let end = item_end(&chars, index + TEST_MARKER.chars().count());
            blank(&mut out, index, end);
            index = end;
        } else if let Some((end, content)) = raw_string(&chars, index) {
            out.push('"');
            out.extend(
                content
                    .iter()
                    .map(|c| if matches!(c, '"' | '\\') { ' ' } else { *c }),
            );
            out.push('"');
            index = end;
        } else if c == '"' {
            let end = string_end(&chars, index);
            out.extend(&chars[index..end]);
            index = end;
        } else if let Some(end) = char_literal_end(&chars, index) {
            blank(&mut out, index, end);
            index = end;
        } else {
            out.push(c);
            index += 1;
        }
    }
    out
}

fn starts_with(chars: &[char], at: usize, pattern: &str) -> bool {
    pattern
        .chars()
        .enumerate()
        .all(|(offset, c)| chars.get(at + offset) == Some(&c))
}

/// The position after `/* … */` (comments nest in Rust).
fn block_comment_end(chars: &[char], start: usize) -> usize {
    let mut depth = 0_usize;
    let mut index = start;
    while index < chars.len() {
        if starts_with(chars, index, "/*") {
            depth += 1;
            index += 2;
        } else if starts_with(chars, index, "*/") {
            depth -= 1;
            index += 2;
            if depth == 0 {
                return index;
            }
        } else {
            index += 1;
        }
    }
    chars.len()
}

/// The position after the string that starts at `start` (a `"`).
fn string_end(chars: &[char], start: usize) -> usize {
    let mut index = start + 1;
    while let Some(&c) = chars.get(index) {
        index += if c == '\\' { 2 } else { 1 };
        if c == '"' {
            return index;
        }
    }
    chars.len()
}

/// `r"…"`, `r#"…"#`, `br"…"`: the position after it and its content.
fn raw_string(chars: &[char], start: usize) -> Option<(usize, &[char])> {
    if start > 0 && is_ident_char(chars[start - 1]) {
        return None;
    }
    let mut index = start;
    if chars.get(index) == Some(&'b') {
        index += 1;
    }
    if chars.get(index) != Some(&'r') {
        return None;
    }
    index += 1;
    let hashes = chars[index..].iter().take_while(|c| **c == '#').count();
    index += hashes;
    if chars.get(index) != Some(&'"') {
        return None;
    }
    let content_start = index + 1;
    let closing: Vec<char> = std::iter::once('"')
        .chain(std::iter::repeat_n('#', hashes))
        .collect();
    let end = (content_start..chars.len()).find(|at| chars[*at..].starts_with(&closing))?;
    Some((end + closing.len(), &chars[content_start..end]))
}

/// The position after a char literal (`'x'`, `'\n'`, `'"'`); `None` for a lifetime (`'a`).
fn char_literal_end(chars: &[char], start: usize) -> Option<usize> {
    if chars[start] != '\'' || (start > 0 && is_ident_char(chars[start - 1])) {
        return None;
    }
    if chars.get(start + 1) == Some(&'\\') {
        let close = chars[start + 2..].iter().position(|c| *c == '\'')?;
        return Some(start + 2 + close + 1);
    }
    (chars.get(start + 2) == Some(&'\'')).then_some(start + 3)
}

/// The end of the item that starts at `start`: after its `;` or its closing `}`.
fn item_end(chars: &[char], start: usize) -> usize {
    let mut index = start;
    let mut round = 0_usize;
    while let Some(&c) = chars.get(index) {
        match c {
            '"' => {
                index = string_end(chars, index);
                continue;
            }
            '(' | '[' => round += 1,
            ')' | ']' => round = round.saturating_sub(1),
            ';' if round == 0 => return index + 1,
            '{' if round == 0 => return brace_end(chars, index),
            _ => {}
        }
        index += 1;
    }
    chars.len()
}

/// The position after the `}` that closes the `{` at `start`.
fn brace_end(chars: &[char], start: usize) -> usize {
    let mut depth = 0_usize;
    let mut index = start;
    while let Some(&c) = chars.get(index) {
        match c {
            '"' => {
                index = string_end(chars, index);
                continue;
            }
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return index + 1;
                }
            }
            _ => {}
        }
        index += 1;
    }
    chars.len()
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Every `name!(…)` log call: the byte position of the macro name and the text between its
/// brackets.
fn log_calls(code: &str) -> Vec<(usize, &str)> {
    let mut calls = Vec::new();
    for name in MACROS {
        let needle = format!("{name}!");
        for (at, _) in code.match_indices(&needle) {
            if code[..at].chars().next_back().is_some_and(is_ident_char) {
                continue;
            }
            let rest = &code[at + needle.len()..];
            let Some(open) = rest.chars().next().filter(|c| matches!(c, '(' | '[' | '{')) else {
                continue;
            };
            if let Some(body) = balanced(&rest[1..], open) {
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
        if is_ident_char(c) {
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
                .split(|c: char| !is_ident_char(c))
                .filter_map(personal)
                .collect::<Vec<_>>()
        })
        .collect()
}

fn scan(file: &str, text: &str) -> Vec<Finding> {
    let code = code_of(text);
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
