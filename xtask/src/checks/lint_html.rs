//! LINT-HTML: the hand-written HTML pages are valid and accessible (html-validate).
//!
//! The pages are artifact pages: the host adds the document around them, so they have no
//! doctype, html, head or body. Generated pages are not checked here (DOC-GENERATED).

use std::path::Path;

use super::node_tool::{Selection, run};
use super::{Check, Context, Finding};

const SELECTION: Selection = Selection {
    folders: &["docs/mockups"],
    files: &["docs/02-ux-design.html"],
    endings: &[".html"],
};

pub(crate) struct LintHtml;

impl Check for LintHtml {
    fn id(&self) -> &'static str {
        "LINT-HTML"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        lint(context.repo.root(), &SELECTION.pick(context))
    }
}

fn lint(root: &Path, files: &[String]) -> anyhow::Result<Vec<Finding>> {
    if files.is_empty() {
        return Ok(Vec::new());
    }
    let mut args = vec!["--config", "htmlvalidate.config.mjs", "--formatter", "json"];
    args.extend(files.iter().map(String::as_str));
    let result = run(root, "html-validate", &args)?;
    if result.success {
        return Ok(Vec::new());
    }
    let mut findings = Vec::new();
    if let Ok(serde_json::Value::Array(reports)) = serde_json::from_str(&result.output) {
        for report in &reports {
            let file = report["filePath"].as_str().unwrap_or_default();
            for message in report["messages"].as_array().into_iter().flatten() {
                findings.push(Finding::at(
                    format!("{file}:{}", message["line"]),
                    format!(
                        "{} [{}]",
                        message["message"].as_str().unwrap_or_default(),
                        message["ruleId"].as_str().unwrap_or_default()
                    ),
                ));
            }
        }
    }
    if findings.is_empty() {
        findings.push(Finding::general(format!(
            "html-validate failed:
{}",
            result.output.trim()
        )));
    }
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::Repo;

    fn findings(file: &str) -> Vec<Finding> {
        let repo = Repo::discover().unwrap();
        lint(
            repo.root(),
            &[format!("tools/checks/fixtures/LINT-HTML/{file}")],
        )
        .unwrap()
    }

    #[test]
    fn bad_example() {
        assert!(!findings("bad/bad.html").is_empty());
    }

    #[test]
    fn good_example() {
        assert!(findings("good/good.html").is_empty());
    }
}
