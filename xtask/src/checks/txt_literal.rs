//! TXT-LITERAL: no hard-coded user-facing text in components (UX-07, coding-standards §4).
//!
//! JSX text, and the string values of `aria-*`, `title`, `alt`, `placeholder` and every other
//! attribute, must come from the message catalog (`t("key")`). The rule is the one of oxlint
//! (`react/jsx-no-literals`, configured once in `.oxlintrc.json`, where LINT-TS runs it too); this
//! check runs it on its own, so a finding names the text rule and not just "lint", and reports
//! only that rule. API messages are covered by ARCH-ERRORS and TXT-CATALOG: the server sends keys.

use std::path::Path;

use super::node_tool::{in_batches, run, unix_findings};
use super::{Check, Context, Finding};

/// What oxlint prints after the message for this rule.
const RULE: &str = "react(jsx-no-literals)";

pub(crate) struct TxtLiteral;

impl Check for TxtLiteral {
    fn id(&self) -> &'static str {
        "TXT-LITERAL"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        literals(context.repo.root(), &context.selected(self.id())?)
    }
}

fn literals(root: &Path, files: &[String]) -> anyhow::Result<Vec<Finding>> {
    let mut findings = Vec::new();
    for batch in in_batches(files) {
        findings.extend(literals_in(root, batch)?);
    }
    Ok(findings)
}

fn literals_in(root: &Path, files: &[String]) -> anyhow::Result<Vec<Finding>> {
    let mut args = vec!["--config", ".oxlintrc.json", "--format", "unix"];
    args.extend(files.iter().map(String::as_str));
    let output = run(root, "oxlint", &args)?;
    // A finding without a place says the tool itself failed: that is never hidden.
    Ok(unix_findings(&output)
        .into_iter()
        .filter(|finding| finding.location.is_none() || finding.message.contains(RULE))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::Repo;

    fn findings(file: &str) -> Vec<Finding> {
        let repo = Repo::discover().unwrap();
        literals(
            repo.root(),
            &[format!("tools/checks/fixtures/TXT-LITERAL/{file}")],
        )
        .unwrap()
    }

    #[test]
    fn bad_example() {
        // Text between tags, and the values of aria-label, title, placeholder and alt.
        assert_eq!(findings("bad/bad.tsx").len(), 5);
    }

    #[test]
    fn good_example() {
        assert!(findings("good/good.tsx").is_empty());
    }
}
