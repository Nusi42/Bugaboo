//! SEC-PATTERNS: code that crashes the server or runs text as code is not written (coding-standards
//! §3, §4).
//!
//! In the Rust code that ships (comments, strings and `#[cfg(test)]` code do not count): no
//! `unwrap()`, `expect(`, `unwrap_err()`, `expect_err(`, `panic!`, `unreachable!`, `todo!`,
//! `unimplemented!` or `dbg!`. Errors are typed values
//! and the caller decides (clippy says the same in its own words; this check holds it without a
//! suppression).
//!
//! In the browser code: no `eval(`, `new Function(`, `document.write(` and no timer given a string.
//! Setting HTML from a string (`innerHTML`, `dangerouslySetInnerHTML`, …) belongs
//! to SEC-RICHTEXT, which names the one place that may.

use super::web_source::code_of;
use super::{Check, Context, Finding};
use crate::rust_source::{is_test_module_file, strip, without_tests};
use crate::source::{Kind, Source, word_positions};

/// What must not be written in Rust code that ships, with what to do instead.
const RUST_PATTERNS: [(&str, &str); 9] = [
    (
        ".unwrap()",
        "return the error (`?`) or handle the empty case",
    ),
    (
        ".expect(",
        "return the error (`?`) or handle the empty case",
    ),
    ("panic!(", "return a typed error"),
    ("todo!(", "write the code, or leave it out of the change"),
    (
        "unreachable!(",
        "return a typed error, or make the case impossible in the types",
    ),
    (
        ".unwrap_err()",
        "return the error (`?`) or handle the empty case",
    ),
    (
        ".expect_err(",
        "return the error (`?`) or handle the empty case",
    ),
    (
        "unimplemented!(",
        "write the code, or leave it out of the change",
    ),
    (
        "dbg!(",
        "use a test, or a `tracing` event without personal data",
    ),
];

pub(crate) struct SecPatterns;

impl Check for SecPatterns {
    fn id(&self) -> &'static str {
        "SEC-PATTERNS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let mut findings = rust_findings(&context.sources(self.id(), Kind::Rust)?);
        findings.extend(web_findings(&context.sources(self.id(), Kind::Web)?));
        Ok(findings)
    }
}

fn rust_findings(sources: &[Source]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (path, text) in sources
        .iter()
        .filter(|(path, _)| !is_test_module_file(path))
    {
        let code = strip(&without_tests(text));
        for (number, line) in code.lines().enumerate() {
            for (pattern, instead) in RUST_PATTERNS {
                if line.contains(pattern) {
                    findings.push(Finding::at(
                        format!("{path}:{}", number + 1),
                        format!(
                            "`{pattern}` in code that ships can crash the server (coding-standards §3). Instead: {instead}."
                        ),
                    ));
                }
            }
        }
    }
    findings
}

/// Whether the line calls `eval(`, `new Function(` or `document.write(`.
fn runs_text_as_code(code: &str) -> bool {
    let eval = word_positions(code, "eval")
        .any(|at| code[at + "eval".len()..].starts_with('(') && !code[..at].ends_with('.'));
    eval || code.contains("new Function(") || code.contains("document.write(")
}

/// A timer given a string runs it as code.
fn string_timer(code: &str) -> bool {
    ["setTimeout", "setInterval"].iter().any(|timer| {
        word_positions(code, timer).any(|at| {
            let rest = code[at + timer.len()..].trim_start();
            rest.strip_prefix('(')
                .is_some_and(|argument| argument.trim_start().starts_with(['"', '\'', '`']))
        })
    })
}

fn web_findings(sources: &[Source]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (path, text) in sources {
        for (number, line) in text.lines().enumerate() {
            let code = code_of(line);
            if runs_text_as_code(code) || string_timer(code) {
                findings.push(Finding::at(
                    format!("{path}:{}", number + 1),
                    "This runs text as code (coding-standards §4). Write the thing as code.",
                ));
            }
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(path: &str, text: &str) -> Vec<Source> {
        vec![(path.to_owned(), text.to_owned())]
    }

    #[test]
    fn good_example_typed_errors_and_tests_that_unwrap() {
        let rust = "fn f(x: Option<u8>) -> Result<u8, E> { x.ok_or(E) }\n\
                    // x.unwrap() in a comment\nconst T: &str = \"panic!(\";\n\
                    #[cfg(test)]\nmod tests {\n    fn t() { Some(1).unwrap(); }\n}\n";
        assert!(rust_findings(&source("crates/bb-core/src/lib.rs", rust)).is_empty());
        let web =
            "page.evaluate(() => 1);\nconst evaluation = 2;\n// eval(x)\nsetTimeout(() => 1, 5);\n";
        assert!(web_findings(&source("web/app/src/a.ts", web)).is_empty());
    }

    #[test]
    fn bad_example_unwrap_expect_panic_in_code_that_ships() {
        let rust = "fn f() { a.unwrap(); b.expect(\"x\"); panic!(\"no\"); dbg!(1); todo!(); }\n";
        let found = rust_findings(&source("crates/bb-api/src/x.rs", rust));
        assert_eq!(found.len(), 5);
    }

    #[test]
    fn bad_example_text_run_as_code_in_the_browser() {
        for line in [
            "eval(text);",
            "const f = new Function(\"return 1\");",
            "document.write(html);",
            "setTimeout(\"alert(1)\", 10);",
        ] {
            assert_eq!(
                web_findings(&source("web/app/src/a.tsx", line)).len(),
                1,
                "{line}"
            );
        }
    }
}
