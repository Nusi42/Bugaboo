//! LINT-CSS: stylelint keeps every style to token variables (ADR-0010, ADR-0026).
//!
//! The rules of `design/theme.kdl` are compiled to CSS (the same text `cargo xtask gen theme`
//! writes after the variables) and stylelint reads that: no color, size, duration or weight written
//! out, no `!important`, fonts from tokens, system colors only under `forced`, and the rules of
//! correctness (no unknown property, no repeated selector). The token values themselves are
//! checked when the file is read. There is no other style file to read: THEME-ONLY keeps CSS out of
//! `web/`.

use std::path::Path;

use super::node_tool::{in_batches, run, unix_findings};
use super::{Check, Context, Finding};
use crate::generate::theme::{doc_css_with_lines, parts_css_with_lines};
use crate::generate::theme_file::{THEME_PATH, ThemeFile, is_theme_source};

/// The rules for the compiled rules of the theme file.
const CONFIG: &str = "stylelint.config.mjs";
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
        let theme_changed = selected
            .iter()
            .any(|file| is_theme_source(file) || file.starts_with("stylelint."));
        if !theme_changed {
            return Ok(Vec::new());
        }
        let file = ThemeFile::load(context.repo)?;
        lint_theme(context.repo.root(), &file, COMPILED_FILE)
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
    // The rules of the parts, and each sheet of the design pages on its own (its selectors are
    // plain CSS, and the same element selector may stand in two sheets). One run of stylelint reads
    // them all: starting it is most of the time.
    let folder = root.join(WORK_FOLDER);
    std::fs::create_dir_all(&folder)?;
    let mut compiled = vec![(name.to_owned(), parts_css_with_lines(file)?)];
    for sheet in &file.docs {
        compiled.push((
            format!("doc-{}-{name}", sheet.name),
            doc_css_with_lines(file, sheet)?,
        ));
    }
    let mut paths = Vec::new();
    for (written, (css, _)) in &compiled {
        std::fs::write(folder.join(written), css)?;
        paths.push(format!("{WORK_FOLDER}/{written}"));
    }
    Ok(lint(root, &paths, CONFIG)?
        .into_iter()
        .map(|finding| {
            // The longest name that the place holds is the sheet it is about.
            let place = finding.location.clone().unwrap_or_default();
            let from = compiled
                .iter()
                .filter(|(written, _)| place.contains(written.as_str()))
                .max_by_key(|(written, _)| written.len())
                .map(|(_, (_, from))| from.as_slice())
                .unwrap_or_default();
            point_at_theme_file(finding, from)
        })
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
            CONFIG,
        )
        .unwrap()
    }

    /// The theme file of the repository with `rules` added, read by stylelint.
    fn theme_findings(name: &str, rules: &str) -> Vec<Finding> {
        let repo = Repo::discover().unwrap();
        let text = crate::source::read_text(&repo, THEME_PATH).unwrap();
        let file = ThemeFile::parse_base(&format!("{text}\n{rules}\n")).unwrap();
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
        let messages = messages(&findings("bad/bad.css"));
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
        assert!(findings("good/good.css").is_empty());
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
        let good = "part \"zz\" {\n  margin 0\n  padding \"$space-2 $space-3\"\n  border \"$border-width solid $border\"\n  font-weight \"$weight-bold\"\n  line-height \"$lh-body\"\n  z-index \"$z-overlay\"\n  opacity 0\n  animation \"fade $motion-base $ease-out\"\n  font-family \"$font-ui\"\n  forced {\n    color CanvasText\n  }\n}";
        let found = theme_findings("good", good);
        assert!(found.is_empty(), "{}", messages(&found));
    }

    /// The theme file of the repository with a rule added to its `components` sheet.
    fn doc_findings(name: &str, rule: &str) -> Vec<Finding> {
        let repo = Repo::discover().unwrap();
        let text = crate::source::read_text(&repo, THEME_PATH).unwrap();
        let wanted = "    rule \"html\" {";
        assert!(text.contains(wanted));
        let text = text.replacen(wanted, &format!("{rule}\n\n{wanted}"), 1);
        let file = ThemeFile::parse_base(&text).unwrap();
        lint_theme(repo.root(), &file, &format!("test-{name}.css")).unwrap()
    }

    #[test]
    fn bad_example_in_a_sheet_of_the_design_pages() {
        let bad = "    rule \".zz\" {\n        margin \"12px\"\n        color \"red\"\n    }";
        let messages = messages(&doc_findings("doc-bad", bad));
        assert!(messages.contains("\"12px\""), "{messages}");
        assert!(messages.contains("color-named"), "{messages}");
    }

    #[test]
    fn good_example_in_a_sheet_of_the_design_pages() {
        let good = "    rule \".zz\" \"a.zz:hover\" {\n        margin \"$space-2 0\"\n        color \"$fg\"\n    }";
        let found = doc_findings("doc-good", good);
        assert!(found.is_empty(), "{}", messages(&found));
    }

    #[test]
    fn the_theme_file_of_the_repository_passes() {
        let found = theme_findings("repository", "");
        assert!(found.is_empty(), "{}", messages(&found));
    }
}
