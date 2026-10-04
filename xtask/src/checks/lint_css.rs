//! LINT-CSS: stylelint keeps CSS Modules to token variables (ADR-0010).

use std::path::Path;

use super::node_tool::{Selection, run, unix_findings};
use super::{Check, Context, Finding};

const SELECTION: Selection = Selection {
    folders: &["web"],
    files: &[],
    endings: &[".css"],
};

pub(crate) struct LintCss;

impl Check for LintCss {
    fn id(&self) -> &'static str {
        "LINT-CSS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        lint(context.repo.root(), &SELECTION.pick(context))
    }
}

fn lint(root: &Path, files: &[String]) -> anyhow::Result<Vec<Finding>> {
    if files.is_empty() {
        return Ok(Vec::new());
    }
    let mut args = vec![
        "--config",
        "stylelint.config.mjs",
        "--formatter",
        "unix",
        "--max-warnings",
        "0",
    ];
    args.extend(files.iter().map(String::as_str));
    Ok(unix_findings(&run(root, "stylelint", &args)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::Repo;

    fn findings(file: &str) -> Vec<Finding> {
        let repo = Repo::discover().unwrap();
        lint(
            repo.root(),
            &[format!("tools/checks/fixtures/LINT-CSS/{file}")],
        )
        .unwrap()
    }

    #[test]
    fn bad_example() {
        let messages: String = findings("bad/bad.module.css")
            .iter()
            .map(|finding| finding.message.clone())
            .collect();
        for rule in [
            "color-no-hex",
            "function-disallowed-list",
            "color-named",
            "declaration-no-important",
            "declaration-property-value-disallowed-list",
        ] {
            assert!(
                messages.contains(rule),
                "expected a finding for {rule}: {messages}"
            );
        }
    }

    #[test]
    fn good_example() {
        assert!(findings("good/good.module.css").is_empty());
    }
}
