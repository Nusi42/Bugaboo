//! ARCH-PURE: `bb-core`, `bb-searchlang` and `bb-markdown` do no I/O (architecture §4 rule 1,
//! ADR-0005, ADR-0011).
//!
//! These crates are also built for WebAssembly, and the server runs the same code, so a result
//! depends on the text it is given and on nothing else. Two things are looked for:
//!
//! - a dependency that does I/O (an async runtime, a database, an HTTP library, a source of random
//!   bytes), in `[dependencies]` of one of the crates;
//! - in the code that ships (comments, strings and `#[cfg(test)]` code do not count), a way to
//!   reach the file system, the network, the clock, the environment, a thread or a process.
//!
//! The caller passes in time and random bytes. `cargo check --target wasm32-unknown-unknown`
//! (ARCH-WASM) is the compiler's view of the same rule.

use super::{Check, Context, Finding};
use crate::metadata::{self, Package};
use crate::rust_source::{is_test_module_file, strip, without_tests};
use crate::source::{Kind, Source};

/// The crates that do no I/O.
const PURE: [&str; 3] = ["bb-core", "bb-searchlang", "bb-markdown"];

/// Dependencies that do I/O.
const BANNED_DEPENDENCIES: [&str; 20] = [
    "tokio",
    "async-std",
    "sqlx",
    "sea-query",
    "sea-query-sqlx",
    "axum",
    "hyper",
    "hyper-util",
    "tower",
    "tower-http",
    "reqwest",
    "h2",
    "rustls",
    "tokio-rustls",
    "getrandom",
    "rand",
    "tempfile",
    "rust-embed",
    "mime_guess",
    "ctrlc",
];

/// Ways to reach the world outside the text, as they are written in code.
const BANNED_USES: [(&str, &str); 14] = [
    ("std::fs", "the file system"),
    ("std::net", "the network"),
    ("std::process", "a process"),
    ("std::env", "the environment"),
    ("std::thread", "a thread"),
    ("std::os::", "the operating system"),
    ("std::io::stdin", "the keyboard"),
    ("std::io::stdout", "the screen"),
    ("std::io::stderr", "the screen"),
    ("SystemTime", "the clock"),
    ("Instant::now", "the clock"),
    ("Timestamp::now", "the clock"),
    ("getrandom", "random bytes"),
    ("rand::", "random bytes"),
];

pub(crate) struct ArchPure;

impl Check for ArchPure {
    fn id(&self) -> &'static str {
        "ARCH-PURE"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let packages = metadata::load(context.repo.root())?;
        let sources = context.everything().sources(self.id(), Kind::Rust)?;
        let mut findings = dependency_findings(&packages);
        findings.extend(use_findings(&sources));
        Ok(findings)
    }
}

fn dependency_findings(packages: &[Package]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for package in packages.iter().filter(|p| PURE.contains(&p.name.as_str())) {
        for dependency in package
            .dependencies
            .iter()
            .filter(|d| d.kind.is_none() && BANNED_DEPENDENCIES.contains(&d.name.as_str()))
        {
            findings.push(Finding::at(
                &package.name,
                format!(
                    "depends on `{}`, which does I/O. This crate is also built for WebAssembly and \
                     does no I/O: the caller passes in what it needs (ARCH-PURE, architecture §4).",
                    dependency.name
                ),
            ));
        }
    }
    findings
}

fn use_findings(sources: &[Source]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (path, text) in sources.iter().filter(|(path, _)| {
        !is_test_module_file(path)
            && PURE
                .iter()
                .any(|name| path.starts_with(&format!("crates/{name}/")))
    }) {
        let code = strip(&without_tests(text));
        for (number, line) in code.lines().enumerate() {
            for (pattern, world) in BANNED_USES {
                if line.contains(pattern) {
                    findings.push(Finding::at(
                        format!("{path}:{}", number + 1),
                        format!(
                            "`{pattern}` reaches {world}. This crate does no I/O (ARCH-PURE): take \
                             the value as an argument, and let the caller get it."
                        ),
                    ));
                }
            }
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::fixtures::package;

    fn source(path: &str, text: &str) -> Vec<Source> {
        vec![(path.to_owned(), text.to_owned())]
    }

    #[test]
    fn good_example_pure_code_and_tests_that_read_files() {
        let text = "pub fn add(a: u32, b: u32) -> u32 { a + b }\n\
                    // std::fs in a comment\nconst T: &str = \"std::net\";\n\
                    #[cfg(test)]\nmod tests {\n    use std::fs;\n}\n";
        assert!(use_findings(&source("crates/bb-markdown/src/lib.rs", text)).is_empty());
        assert!(dependency_findings(&[package("bb-markdown", &["bb-macros"])]).is_empty());
    }

    #[test]
    fn good_example_other_crates_may_do_io() {
        let text = "use std::fs;\nfn f() { let _ = std::time::SystemTime::now(); }\n";
        assert!(use_findings(&source("crates/bb-store/src/lib.rs", text)).is_empty());
        assert!(dependency_findings(&[package("bb-store", &["tokio"])]).is_empty());
    }

    #[test]
    fn bad_example_a_file_the_clock_and_a_runtime() {
        let text = "fn f() {\n    let _ = std::fs::read(\"a\");\n    let _ = std::time::SystemTime::now();\n}\n";
        let found = use_findings(&source("crates/bb-core/src/lib.rs", text));
        assert_eq!(found.len(), 2, "the file and the clock");
        assert_eq!(
            dependency_findings(&[package("bb-searchlang", &["tokio"])]).len(),
            1
        );
    }
}
