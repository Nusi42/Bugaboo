//! ARCH-PURE: `bb-core`, `bb-searchlang` and `bb-markdown` do no I/O (architecture §4 rule 1,
//! ADR-0005, ADR-0011).
//!
//! These crates are also built for WebAssembly, and the server runs the same code, so a result
//! depends on the text it is given and on nothing else. Two things hold that:
//!
//! - this check: none of the three has a dependency that does I/O (an async runtime, a database,
//!   an HTTP library, a source of random bytes) in `[dependencies]`;
//! - the compiler: each crate has a `clippy.toml` that disallows the methods and types of the
//!   standard library that reach the files, the network, the clock, the environment, a thread or a
//!   process. Those files are generated (`cargo xtask gen clippy`, held by DOC-GENERATED) from the
//!   root file and one list, so they cannot differ; clippy (LINT-RUST, on push and in CI) reads the
//!   code, and `cargo check --target wasm32-unknown-unknown` (ARCH-WASM) builds it for the browser.

use super::{Check, Context, Finding};
use crate::generate::clippy::PURE_CRATES;
use crate::metadata::{self, Package};

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

pub(crate) struct ArchPure;

impl Check for ArchPure {
    fn id(&self) -> &'static str {
        "ARCH-PURE"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        Ok(dependency_findings(&metadata::load(context.repo.root())?))
    }
}

fn dependency_findings(packages: &[Package]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for package in packages
        .iter()
        .filter(|p| PURE_CRATES.iter().any(|pure| pure.name == p.name))
    {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::fixtures::package;

    #[test]
    fn good_example_pure_crates_with_plain_dependencies() {
        assert!(dependency_findings(&[package("bb-markdown", &["bb-macros"])]).is_empty());
    }

    #[test]
    fn good_example_other_crates_may_do_io() {
        assert!(dependency_findings(&[package("bb-store", &["tokio"])]).is_empty());
    }

    #[test]
    fn bad_example_a_runtime_or_a_database_in_a_pure_crate() {
        assert_eq!(
            dependency_findings(&[package("bb-searchlang", &["tokio", "sqlx"])]).len(),
            2
        );
    }

    #[test]
    fn the_three_pure_crates_are_the_ones_of_the_architecture() {
        let names: Vec<&str> = PURE_CRATES.iter().map(|pure| pure.name).collect();
        assert_eq!(names, ["bb-core", "bb-searchlang", "bb-markdown"]);
    }
}
