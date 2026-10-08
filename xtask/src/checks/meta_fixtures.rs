//! META-FIXTURES: every active check has a failing and a passing example (ADR-0020).
//!
//! A check's source file is the one whose `id()` returns the check's ID. An example is either a
//! pair of tests named `bad_example…` and `good_example…` in that file, or the folders
//! `tools/checks/fixtures/<ID>/bad` and `…/good`.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use super::{Check, Context, Finding};
use crate::catalog::Entry;
use crate::docs::source::sorted_files;
use crate::docs::text::has_extension;
use crate::source::read_text;

const CHECKS_FOLDER: &str = "xtask/src/checks";
const FIXTURES_FOLDER: &str = "tools/checks/fixtures";

pub(crate) struct MetaFixtures;

impl Check for MetaFixtures {
    fn id(&self) -> &'static str {
        "META-FIXTURES"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let repo = context.repo;
        let mut sources = Vec::new();
        for name in sorted_files(repo, CHECKS_FOLDER, |name| has_extension(name, "rs"))? {
            let path = format!("{CHECKS_FOLDER}/{name}");
            sources.push((path.clone(), read_text(repo, &path)?));
        }
        let fixtures = fixture_folders(&repo.path(FIXTURES_FOLDER));
        let active: Vec<&Entry> = context
            .catalog
            .entries()
            .iter()
            .filter(|entry| entry.is_active())
            .collect();
        Ok(evaluate(&active, &sources, &fixtures))
    }
}

/// The IDs that have both a non-empty `bad` and a non-empty `good` fixture folder.
fn fixture_folders(folder: &Path) -> HashSet<String> {
    let non_empty =
        |path: &Path| fs::read_dir(path).is_ok_and(|mut entries| entries.next().is_some());
    let Ok(entries) = fs::read_dir(folder) else {
        return HashSet::new();
    };
    entries
        .flatten()
        .filter(|entry| {
            non_empty(&entry.path().join("bad")) && non_empty(&entry.path().join("good"))
        })
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect()
}

/// The IDs a source file implements: the string each `id()` body returns.
fn implemented_ids(source: &str) -> Vec<&str> {
    // Built from parts so this file's own text does not look like a check.
    let needle = format!("fn {}(&self)", "id");
    source
        .match_indices(&needle)
        .filter_map(|(at, _)| {
            let body = &source[at..];
            let body = &body[body.find('{')?..];
            let start = body.find('"')? + 1;
            let length = body[start..].find('"')?;
            Some(&body[start..start + length])
        })
        .collect()
}

fn evaluate(
    active: &[&Entry],
    sources: &[(String, String)],
    fixtures: &HashSet<String>,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for entry in active {
        if fixtures.contains(&entry.id) {
            continue;
        }
        let implementing = sources
            .iter()
            .find(|(_, text)| implemented_ids(text).contains(&entry.id.as_str()));
        match implementing {
            None => findings.push(Finding::at(
                &entry.id,
                format!("No file in {CHECKS_FOLDER}/ implements this check, so it cannot have examples."),
            )),
            Some((path, text)) if !has_both_examples(text) => findings.push(Finding::at(
                &entry.id,
                format!(
                    "No bad and good example. Add tests `bad_example` and `good_example` to {path}, or the folders {FIXTURES_FOLDER}/{}/bad and /good.",
                    entry.id
                ),
            )),
            Some(_) => {}
        }
    }
    findings
}

fn has_both_examples(text: &str) -> bool {
    text.contains("fn bad_example") && text.contains("fn good_example")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str) -> Entry {
        Entry {
            id: id.to_owned(),
            title: "t".into(),
            rule: "r".into(),
            tool: "x".into(),
            when: vec!["ci".into()],
            files: vec!["*".into()],
            status: "active".into(),
        }
    }

    /// A check's source file; `mention` is another ID that only appears as text.
    fn source(id: &str, mention: &str, tests: &str) -> (String, String) {
        (
            format!("xtask/src/checks/{}.rs", id.to_lowercase()),
            format!(
                "impl Check for X {{ fn {}(&self) -> &'static str {{ \"{id}\" }} }}\n// {mention} \"{mention}\"\n{tests}",
                "id"
            ),
        )
    }

    #[test]
    fn good_example() {
        let a = entry("A-1");
        let b = entry("B-1");
        let sources = vec![source(
            "A-1",
            "B-1",
            "fn bad_example() {} fn good_example() {}",
        )];
        let fixtures = HashSet::from(["B-1".to_owned()]);
        assert!(evaluate(&[&a, &b], &sources, &fixtures).is_empty());
    }

    #[test]
    fn bad_example() {
        let a = entry("A-1");
        let b = entry("B-1");
        let c = entry("C-1");
        let sources = vec![
            source("A-1", "C-1", "fn good_example() {}"),
            source("B-1", "C-1", "fn bad_example_x() {} fn good_example_x() {}"),
        ];
        let findings = evaluate(&[&a, &b, &c], &sources, &HashSet::new());
        assert_eq!(findings.len(), 2);
        assert!(findings[0].message.contains("No bad and good example"));
        assert!(findings[1].message.contains("No file in"));
    }
}
