//! META-COVERAGE: every file is covered by a formatter or linter (ADR-0020).
//!
//! A new kind of file fails this check until an active FMT-* or LINT-* check covers it. This is
//! the hook that asks for new hooks. Two kinds of files need no such check: generated files
//! (the paths `cargo xtask docs` and the other generators write, kept up to date by DOC-GENERATED) and the files listed,
//! each with a reason, in `tools/checks/coverage-exemptions.toml`.

use std::collections::BTreeMap;

use anyhow::Context as _;
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use serde::Deserialize;

use super::{Check, Context, Finding};
use crate::catalog::Entry;
use crate::generate::all_paths;
use crate::source::read_text;

/// Where the exemptions live, relative to the repository root.
const EXEMPTIONS_PATH: &str = "tools/checks/coverage-exemptions.toml";

#[derive(Deserialize, Default)]
struct Exemptions {
    #[serde(default)]
    exempt: Vec<Exemption>,
}

#[derive(Deserialize)]
struct Exemption {
    /// An exact path or a path glob.
    path: String,
    reason: String,
}

pub(crate) struct MetaCoverage;

impl Check for MetaCoverage {
    fn id(&self) -> &'static str {
        "META-COVERAGE"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let repo = context.repo;
        let exemptions: Exemptions = toml::from_str(&read_text(repo, EXEMPTIONS_PATH)?)
            .with_context(|| format!("{EXEMPTIONS_PATH} is not valid"))?;
        let generated = all_paths();
        let files: Vec<File> = repo
            .all_files()?
            .into_iter()
            .map(|path| {
                let generated = generated.contains(&path.as_str());
                File { path, generated }
            })
            .collect();
        let checks: Vec<&Entry> = context.catalog.entries().iter().collect();
        evaluate(&files, &checks, &exemptions)
    }
}

struct File {
    path: String,
    generated: bool,
}

/// The file type of a path: its extension, or the whole name for dotfiles like `.gitignore`.
fn file_type(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    match name.rsplit_once('.') {
        Some((_, extension)) => extension.to_owned(),
        None => name.to_owned(),
    }
}

fn evaluate(
    files: &[File],
    checks: &[&Entry],
    exemptions: &Exemptions,
) -> anyhow::Result<Vec<Finding>> {
    let mut findings = Vec::new();
    let mut covering: Vec<GlobSet> = Vec::new();
    for entry in checks.iter().filter(|entry| is_formatter_or_linter(entry)) {
        let mut builder = GlobSetBuilder::new();
        for pattern in entry.files.iter().filter(|pattern| *pattern != "*") {
            builder.add(GlobBuilder::new(pattern).literal_separator(true).build()?);
        }
        covering.push(builder.build()?);
    }
    let mut exempt: Vec<(&Exemption, bool, _)> = Vec::new();
    for exemption in &exemptions.exempt {
        if exemption.reason.trim().is_empty() {
            findings.push(Finding::at(
                EXEMPTIONS_PATH,
                format!("The exemption for `{}` has no reason.", exemption.path),
            ));
        }
        let glob = GlobBuilder::new(&exemption.path)
            .literal_separator(true)
            .build()
            .with_context(|| format!("{EXEMPTIONS_PATH}: bad path `{}`", exemption.path))?
            .compile_matcher();
        exempt.push((exemption, false, glob));
    }
    let mut uncovered: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for file in files {
        let mut is_exempt = false;
        for (_, used, glob) in &mut exempt {
            if glob.is_match(&file.path) {
                *used = true;
                is_exempt = true;
            }
        }
        if is_exempt || file.generated || covering.iter().any(|set| set.is_match(&file.path)) {
            continue;
        }
        uncovered
            .entry(file_type(&file.path))
            .or_default()
            .push(&file.path);
    }
    for (exemption, used, _) in &exempt {
        if !used {
            findings.push(Finding::at(
                EXEMPTIONS_PATH,
                format!(
                    "The exemption for `{}` matches no file. Remove it.",
                    exemption.path
                ),
            ));
        }
    }
    findings.extend(uncovered.into_iter().map(|(kind, paths)| {
        Finding::at(
            format!(".{kind} files ({})", paths.len()),
            format!(
                "No active FMT-* or LINT-* check covers {}. Add a formatter or linter check for this file type to tools/checks/catalog.toml, or list the file with a reason in {EXEMPTIONS_PATH}.",
                paths[0]
            ),
        )
    }));
    Ok(findings)
}

/// Active FMT-* and LINT-* checks. A catch-all `*` pattern (such as the spell check) covers
/// nothing on its own, because it does not format or lint the kind of file.
fn is_formatter_or_linter(entry: &Entry) -> bool {
    entry.is_active() && (entry.id.starts_with("FMT-") || entry.id.starts_with("LINT-"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, status: &str, files: &[&str]) -> Entry {
        Entry {
            id: id.to_owned(),
            title: "t".into(),
            rule: "r".into(),
            tool: "x".into(),
            when: vec!["ci".into()],
            files: files.iter().map(|f| (*f).to_owned()).collect(),
            status: status.to_owned(),
        }
    }

    fn files(list: &[&str]) -> Vec<File> {
        list.iter()
            .map(|path| File {
                path: (*path).to_owned(),
                generated: false,
            })
            .collect()
    }

    fn exemptions(paths: &[&str]) -> Exemptions {
        Exemptions {
            exempt: paths
                .iter()
                .map(|path| Exemption {
                    path: (*path).to_owned(),
                    reason: "because".to_owned(),
                })
                .collect(),
        }
    }

    #[test]
    fn good_example() {
        let fmt = entry("FMT-RUST", "active", &["**/*.rs"]);
        let spell = entry("LINT-SPELL", "active", &["*"]);
        let mut tracked = files(&["xtask/src/main.rs", "Cargo.lock", "docs/page.html"]);
        tracked[2].generated = true;
        let findings = evaluate(&tracked, &[&fmt, &spell], &exemptions(&["Cargo.lock"]));
        assert!(findings.unwrap().is_empty());
    }

    #[test]
    fn bad_example() {
        let fmt = entry("FMT-RUST", "active", &["**/*.rs"]);
        let planned = entry("FMT-SQL", "planned", &["**/*.sql"]);
        let spell = entry("LINT-SPELL", "active", &["*"]);
        let tracked = files(&[
            "a/main.rs",
            "db/schema.sql",
            "tool.py",
            "x.yaml",
            "Cargo.lock",
        ]);
        let findings = evaluate(
            &tracked,
            &[&fmt, &planned, &spell],
            &exemptions(&["Cargo.lock", "gone.txt"]),
        )
        .unwrap();
        let places: Vec<&str> = findings
            .iter()
            .filter_map(|f| f.location.as_deref())
            .collect();
        assert_eq!(
            places,
            [
                EXEMPTIONS_PATH,
                ".py files (1)",
                ".sql files (1)",
                ".yaml files (1)"
            ]
        );
        assert!(findings[0].message.contains("`gone.txt` matches no file"));
    }

    #[test]
    fn exemptions_are_by_path_not_by_type() {
        let tracked = files(&["a/.gitignore", "b/.gitignore"]);
        let findings = evaluate(&tracked, &[], &exemptions(&["a/.gitignore"])).unwrap();
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("b/.gitignore"));
    }

    #[test]
    fn file_types() {
        assert_eq!(file_type("a/b/main.rs"), "rs");
        assert_eq!(file_type(".gitignore"), "gitignore");
        assert_eq!(file_type("Makefile"), "Makefile");
    }
}
