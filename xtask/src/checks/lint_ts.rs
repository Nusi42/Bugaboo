//! LINT-TS: oxlint with type-aware rules (oxlint-tsgolint, built on TypeScript 7).

use std::path::Path;

use super::node_tool::{Selection, run, unix_findings};
use super::{Check, Context, Finding};

const SELECTION: Selection = Selection {
    folders: &["web", "docs/design"],
    files: &[],
    endings: &[".ts", ".tsx", ".js"],
};

pub(crate) struct LintTs;

impl Check for LintTs {
    fn id(&self) -> &'static str {
        "LINT-TS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        lint(context.repo.root(), &SELECTION.pick(context))
    }
}

/// Warnings count as errors: the rule is zero warnings, zero errors.
///
/// TypeScript files get the type-aware rules. Plain JavaScript (the docs page script) has no
/// types to check, so it gets every other rule.
fn lint(root: &Path, files: &[String]) -> anyhow::Result<Vec<Finding>> {
    let (scripts, typed): (Vec<&String>, Vec<&String>) = files
        .iter()
        .partition(|file| Path::new(file).extension().is_some_and(|e| e == "js"));
    let mut findings = lint_group(root, &typed, true)?;
    findings.extend(lint_group(root, &scripts, false)?);
    Ok(findings)
}

fn lint_group(root: &Path, files: &[&String], type_aware: bool) -> anyhow::Result<Vec<Finding>> {
    if files.is_empty() {
        return Ok(Vec::new());
    }
    let mut args = vec![
        "--config",
        ".oxlintrc.json",
        "--deny-warnings",
        "--format",
        "unix",
    ];
    if type_aware {
        args.push("--type-aware");
    }
    args.extend(files.iter().map(|file| file.as_str()));
    Ok(unix_findings(&run(root, "oxlint", &args)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::Repo;

    fn findings(files: &[&str]) -> Vec<Finding> {
        let repo = Repo::discover().unwrap();
        let files: Vec<String> = files
            .iter()
            .map(|file| format!("tools/checks/fixtures/LINT-TS/{file}"))
            .collect();
        lint(repo.root(), &files).unwrap()
    }

    #[test]
    fn bad_example() {
        let messages: String = findings(&["bad/bad.ts", "bad/bad.tsx"])
            .iter()
            .map(|finding| finding.message.clone())
            .collect();
        for rule in [
            "no-console",
            "no-explicit-any",
            "no-non-null-assertion",
            "no-eval",
            "no-floating-promises",
            "jsx-no-literals",
            "forbid-dom-props",
        ] {
            assert!(
                messages.contains(rule),
                "expected a finding for {rule}: {messages}"
            );
        }
    }

    #[test]
    fn good_example() {
        assert!(findings(&["good/good.ts", "good/good.tsx"]).is_empty());
    }
}
