//! LINT-CSS: stylelint keeps every style to token variables (ADR-0010, ADR-0026).
//!
//! The rules of `design/theme.kdl` are compiled to CSS (the same text `cargo xtask gen theme`
//! writes after the variables) and stylelint reads that: no color, size, duration or weight written
//! out, no `!important`, fonts from tokens, system colors only under `forced`, and the rules of
//! correctness (no unknown property, no repeated selector). The token values themselves are
//! checked when the file is read. The CSS Modules that have not been moved onto the theme file yet
//! (THEME-ONLY) are read by stylelint as before, with the older rules of `stylelint.config.mjs`.

use std::path::Path;

use super::node_tool::{in_batches, run, unix_findings};
use super::{Check, Context, Finding};
use crate::docs::text::has_extension;
use crate::generate::all_paths;
use crate::generate::theme::parts_css_with_lines;
use crate::generate::theme_file::{THEME_PATH, ThemeFile};

/// The rules for the CSS Modules that still stand.
const MODULES_CONFIG: &str = "stylelint.config.mjs";
/// The rules for the compiled rules of the theme file: the same, and stricter.
const THEME_CONFIG: &str = "stylelint.theme.config.mjs";
/// Where the compiled rules are written for stylelint to read, below the repository.
const WORK_FOLDER: &str = "target/lint-css";
const COMPILED_FILE: &str = "theme-rules.css";

pub(crate) struct LintCss;

impl Check for LintCss {
    fn id(&self) -> &'static str {
        "LINT-CSS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let selected = context.selected(self.id())?;
        let generated = all_paths();
        let modules: Vec<String> = selected
            .iter()
            .filter(|file| has_extension(file, "css") && !generated.contains(&file.as_str()))
            .cloned()
            .collect();
        let root = context.repo.root();
        let mut findings = lint(root, &modules, MODULES_CONFIG)?;
        let theme_changed = selected
            .iter()
            .any(|file| file == THEME_PATH || file.starts_with("stylelint."));
        if theme_changed {
            let file = ThemeFile::load(context.repo)?;
            findings.extend(lint_theme(root, &file, COMPILED_FILE)?);
        }
        Ok(findings)
    }
}

fn lint(root: &Path, files: &[String], config: &str) -> anyhow::Result<Vec<Finding>> {
    let mut findings = Vec::new();
    for batch in in_batches(files) {
        findings.extend(lint_batch(root, batch, config)?);
    }
    Ok(findings)
}

fn lint_batch(root: &Path, files: &[String], config: &str) -> anyhow::Result<Vec<Finding>> {
    let mut args = vec![
        "--config",
        config,
        "--formatter",
        "unix",
        "--max-warnings",
        "0",
    ];
    args.extend(files.iter().map(String::as_str));
    Ok(unix_findings(&run(root, "stylelint", &args)?))
}

/// The rules of the theme file, compiled, read by stylelint. A problem is named by the line of the
/// theme file that the rule comes from (not by the temporary file, whatever way stylelint writes
/// its path), and the message says which line of the compiled text it is.
fn lint_theme(root: &Path, file: &ThemeFile, name: &str) -> anyhow::Result<Vec<Finding>> {
    let (css, from) = parts_css_with_lines(file)?;
    let folder = root.join(WORK_FOLDER);
    std::fs::create_dir_all(&folder)?;
    std::fs::write(folder.join(name), css)?;
    let compiled = format!("{WORK_FOLDER}/{name}");
    let findings = lint(root, std::slice::from_ref(&compiled), THEME_CONFIG)?;
    Ok(findings
        .into_iter()
        .map(|finding| point_at_theme_file(finding, &from))
        .collect())
}

/// A finding about the compiled text, as one about the rule of the theme file. The place is
/// `path:line`; the path may be absolute and hold a drive letter, so only what follows the last
/// colon is the line.
fn point_at_theme_file(finding: Finding, from: &[usize]) -> Finding {
    let compiled_line = finding
        .location
        .as_deref()
        .and_then(|place| place.rsplit(':').next())
        .and_then(|row| row.parse::<usize>().ok());
    let Some(row) = compiled_line else {
        return finding;
    };
    let rule_line = from.get(row.saturating_sub(1)).copied().unwrap_or(1);
    Finding::at(
        format!("{THEME_PATH}:{rule_line}"),
        format!("{} (line {row} of the compiled rules)", finding.message),
    )
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
            MODULES_CONFIG,
        )
        .unwrap()
    }

    /// The theme file of the repository with `rules` added, read by stylelint.
    fn theme_findings(name: &str, rules: &str) -> Vec<Finding> {
        let repo = Repo::discover().unwrap();
        let text = crate::source::read_text(&repo, THEME_PATH).unwrap();
        let file = ThemeFile::parse(&format!("{text}\n{rules}\n")).unwrap();
        // Each test has its own file: the tests run side by side.
        lint_theme(repo.root(), &file, &format!("test-{name}.css")).unwrap()
    }

    fn messages(found: &[Finding]) -> String {
        found
            .iter()
            .map(|finding| finding.message.clone())
            .collect()
    }

    #[test]
    fn bad_example() {
        let messages = messages(&findings("bad/bad.module.css"));
        for rule in [
            "color-no-hex",
            "function-disallowed-list",
            "color-named",
            "declaration-no-important",
            "declaration-property-value-disallowed-list",
            "\"-4px\"",
            "\"-0.5em\"",
            "declaration-property-value-allowed-list",
            "system-colors-only-in-forced-colors",
            "\"1PX\"",
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

    #[test]
    fn a_finding_points_at_the_rule_of_the_theme_file() {
        let repo = Repo::discover().unwrap();
        let before = crate::source::read_text(&repo, THEME_PATH)
            .unwrap()
            .lines()
            .count();
        let found = theme_findings("place", "part \"zz\" {\n  margin \"12px\"\n}");
        let place = found[0].location.clone().unwrap_or_default();
        let line: usize = place
            .strip_prefix("design/theme.kdl:")
            .and_then(|row| row.parse().ok())
            .unwrap_or(0);
        assert!(line > before, "{place}");
    }

    #[test]
    fn a_path_with_a_drive_letter_is_no_problem() {
        let finding = Finding::at(
            "C:/Users/x/target/lint-css/theme-rules.css:3",
            "Disallowed value",
        );
        let moved = point_at_theme_file(finding, &[9, 9, 14]);
        assert_eq!(moved.location.as_deref(), Some("design/theme.kdl:14"));
        assert!(moved.message.contains("line 3 of the compiled rules"));
    }

    #[test]
    fn bad_example_in_the_theme_file() {
        let bad = "part \"zz\" {\n  margin \"12px\"\n  border \"1px solid red\"\n  padding \"1rem\"\n  font-weight 700\n  line-height 1.4\n  z-index 30\n  opacity 0.6\n  transition \"opacity 200ms ease\"\n  font-family \"Arial\"\n  color CanvasText\n  width \"100px !important\"\n}";
        let messages = messages(&theme_findings("bad", bad));
        for expected in [
            "\"12px\"",
            "\"1px solid red\"",
            "color-named",
            "\"1rem\"",
            "\"700\"",
            "\"1.4\"",
            "\"30\"",
            "\"0.6\"",
            "\"opacity 200ms ease\"",
            "declaration-property-value-allowed-list",
            "system-colors-only-in-forced-colors",
            "declaration-no-important",
        ] {
            assert!(
                messages.contains(expected),
                "expected a finding for {expected}: {messages}"
            );
        }
    }

    #[test]
    fn good_example_in_the_theme_file() {
        let good = "part \"zz\" {\n  margin 0\n  padding \"$space-2 $space-3\"\n  border \"$border-width solid $border\"\n  font-weight \"$weight-bold\"\n  line-height \"$lh-body\"\n  z-index \"$z-overlay\"\n  opacity 0\n  animation \"fade $motion-base $ease-out\"\n  font-family \"$font\"\n  forced {\n    color CanvasText\n  }\n}";
        let found = theme_findings("good", good);
        assert!(found.is_empty(), "{}", messages(&found));
    }

    #[test]
    fn the_theme_file_of_the_repository_passes() {
        let found = theme_findings("repository", "");
        assert!(found.is_empty(), "{}", messages(&found));
    }
}
