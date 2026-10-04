//! LINT-TYPES: the TypeScript 7 compiler finds no type errors.

use std::path::Path;

use super::node_tool::run;
use super::{Check, Context, Finding};

/// The project that covers the whole browser app.
const PROJECT: &str = "web/tsconfig.json";

pub(crate) struct LintTypes;

impl Check for LintTypes {
    fn id(&self) -> &'static str {
        "LINT-TYPES"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        type_check(context.repo.root(), PROJECT)
    }
}

fn type_check(root: &Path, project: &str) -> anyhow::Result<Vec<Finding>> {
    let result = run(
        root,
        "tsc",
        &["--noEmit", "--pretty", "false", "--project", project],
    )?;
    let mut findings: Vec<Finding> = result
        .output
        .lines()
        .filter(|line| line.contains(": error TS"))
        .map(|line| match line.split_once(": error ") {
            Some((place, message)) => Finding::at(place, message),
            None => Finding::general(line),
        })
        .collect();
    if findings.is_empty() && !result.success {
        findings.push(Finding::general(format!(
            "tsc failed without naming a problem:\n{}",
            result.output.trim()
        )));
    }
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::Repo;

    fn findings(example: &str) -> Vec<Finding> {
        let repo = Repo::discover().unwrap();
        type_check(
            repo.root(),
            &format!("tools/checks/fixtures/LINT-TYPES/{example}/tsconfig.json"),
        )
        .unwrap()
    }

    #[test]
    fn bad_example() {
        assert!(!findings("bad").is_empty());
    }

    #[test]
    fn good_example() {
        assert!(findings("good").is_empty());
    }
}
