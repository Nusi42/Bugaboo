//! LINT-TYPES: the TypeScript 7 compiler finds no type errors.

use std::path::Path;

use super::node_tool::run;
use super::{Check, Context, Finding};

/// The file that makes a folder a TypeScript project.
const PROJECT_FILE: &str = "tsconfig.json";

pub(crate) struct LintTypes;

impl Check for LintTypes {
    fn id(&self) -> &'static str {
        "LINT-TYPES"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        // A change in one project can break another, so every project is checked. Browser code and
        // tooling are separate projects, so browser code cannot use Node's types by accident.
        let projects = context.everything().selected(self.id())?;
        // The browser code imports the generated WebAssembly glue, which has to be there to be typed.
        crate::wasm::ensure(context.repo.real_root())?;
        let mut findings = Vec::new();
        for project in projects.iter().filter(|file| file.ends_with(PROJECT_FILE)) {
            findings.extend(type_check(context.repo.root(), project)?);
        }
        Ok(findings)
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
