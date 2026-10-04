//! LINT-HTML: the hand-written HTML pages are valid and accessible (html-validate).
//!
//! The pages are artifact pages: the host adds the document around them, so they have no
//! doctype, html, head or body. Generated pages are not checked here (DOC-GENERATED).

use std::path::Path;

use super::node_tool::{Selection, run};
use super::{Check, Context, Finding};

const SELECTION: Selection = Selection {
    folders: &["docs/mockups"],
    files: &["docs/02-ux-design.html", "web/app/index.html"],
    endings: &[".html"],
};

/// The strict config for the browser app, and the looser one for the documentation pages.
const APP_CONFIG: &str = "htmlvalidate.config.mjs";
const DOCS_CONFIG: &str = "htmlvalidate.docs.config.mjs";

pub(crate) struct LintHtml;

impl Check for LintHtml {
    fn id(&self) -> &'static str {
        "LINT-HTML"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let (app, docs): (Vec<String>, Vec<String>) = SELECTION
            .pick(context)
            .into_iter()
            .partition(|file| file.starts_with("web/"));
        let root = context.repo.root();
        let mut findings = lint(root, &app, APP_CONFIG)?;
        findings.extend(lint(root, &docs, DOCS_CONFIG)?);
        Ok(findings)
    }
}

fn lint(root: &Path, files: &[String], config: &str) -> anyhow::Result<Vec<Finding>> {
    if files.is_empty() {
        return Ok(Vec::new());
    }
    let mut args = vec!["--config", config, "--formatter", "json"];
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

    fn findings(file: &str, config: &str) -> Vec<Finding> {
        let repo = Repo::discover().unwrap();
        lint(
            repo.root(),
            &[format!("tools/checks/fixtures/LINT-HTML/{file}")],
            config,
        )
        .unwrap()
    }

    #[test]
    fn bad_example() {
        assert!(!findings("bad/bad.html", DOCS_CONFIG).is_empty());
    }

    #[test]
    fn good_example() {
        assert!(findings("good/good.html", DOCS_CONFIG).is_empty());
    }

    #[test]
    fn app_pages_get_no_inline_style_exception() {
        // The same page passes the docs config but fails the strict app config.
        assert!(findings("good/inline-style.html", DOCS_CONFIG).is_empty());
        assert!(!findings("good/inline-style.html", APP_CONFIG).is_empty());
    }

    #[test]
    fn good_app_page() {
        assert!(findings("good/app.html", APP_CONFIG).is_empty());
    }
}
