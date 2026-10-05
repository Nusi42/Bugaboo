//! TEST-A11Y: every address of the browser app is visited by the accessibility suite (UX-04).
//!
//! The suite is the Playwright tests `web/app/e2e/a11y*.spec.ts`: each opens addresses in the
//! four themes, on a computer and on a phone, and runs axe on them. The addresses are the entries of
//! `paths` in `web/app/src/shared/paths.ts`. This check fails when an address has no entry in the
//! suite, so a new page cannot be added without being checked; the suite itself runs in
//! `cargo xtask ci` (the browser end-to-end tests).

use std::fs;

use anyhow::Context as _;

use super::web_source::code_of;
use super::{Check, Context, Finding};
use crate::docs::source::sorted_files;
use crate::docs::text::has_extension;

const PATHS_FILE: &str = "web/app/src/shared/paths.ts";
const SUITE_FOLDER: &str = "web/app/e2e";
const SUITE_PREFIX: &str = "a11y";

pub(crate) struct TestA11y;

impl Check for TestA11y {
    fn id(&self) -> &'static str {
        "TEST-A11Y"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let repo = context.repo;
        let paths = fs::read_to_string(repo.path(PATHS_FILE))
            .with_context(|| format!("could not read {PATHS_FILE}"))?;
        let mut suite = String::new();
        for name in sorted_files(repo, SUITE_FOLDER, |name| has_extension(name, "ts"))? {
            if name.starts_with(SUITE_PREFIX) && name.ends_with(".spec.ts") {
                suite.push_str(&fs::read_to_string(
                    repo.path(&format!("{SUITE_FOLDER}/{name}")),
                )?);
            }
        }
        Ok(evaluate(
            &paths.replace("\r\n", "\n"),
            &suite.replace("\r\n", "\n"),
        ))
    }
}

/// The entries of `paths`: the name and the address of each.
fn entries(paths_text: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut inside = false;
    for line in paths_text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("export const paths") {
            inside = true;
        } else if inside && trimmed.starts_with('}') {
            break;
        } else if inside && let Some((name, rest)) = trimmed.split_once(':') {
            let address = rest.trim().trim_end_matches(',').trim_matches('"');
            if address.starts_with('/') {
                found.push((name.trim().to_owned(), address.to_owned()));
            }
        }
    }
    found
}

/// The code of the suite: a comment that names an address does not visit it.
fn code(suite: &str) -> String {
    suite.lines().map(code_of).collect::<Vec<_>>().join(
        "
",
    )
}

/// Whether the suite mentions `paths.<name>` as a whole word.
fn mentions(suite: &str, name: &str) -> bool {
    let needle = format!("paths.{name}");
    suite.match_indices(&needle).any(|(at, _)| {
        suite[at + needle.len()..]
            .chars()
            .next()
            .is_none_or(|next| !(next.is_alphanumeric() || next == '_'))
    })
}

fn evaluate(paths_text: &str, suite: &str) -> Vec<Finding> {
    let suite = code(suite);
    let suite = suite.as_str();
    let all = entries(paths_text);
    if all.is_empty() {
        return vec![Finding::at(
            PATHS_FILE,
            "No addresses found in `paths`; the check needs `export const paths = { name: \"/address\", … }`.",
        )];
    }
    all.iter()
        .filter(|(name, _)| !mentions(suite, name))
        .map(|(name, address)| {
            Finding::at(
                PATHS_FILE,
                format!(
                    "The address `paths.{name}` ({address}) has no entry in the accessibility suite. Add it to a test in {SUITE_FOLDER}/{SUITE_PREFIX}*.spec.ts, which visits it in the four themes on a computer and a phone."
                ),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATHS: &str = "export const paths = {\n  home: \"/\",\n  signIn: \"/sign-in\",\n  longer: \"/longer\",\n} as const;\n";

    #[test]
    fn bad_example() {
        // `longer` is only a longer name that starts like `paths.long`: the entry for it is missing.
        let suite = "visit(paths.home); visit(paths.signIn); visit(paths.long);";
        let findings = evaluate(PATHS, suite);
        assert_eq!(
            findings.len(),
            1,
            "{:?}",
            findings.iter().map(|f| &f.message).collect::<Vec<_>>()
        );
        assert!(findings[0].message.contains("paths.longer"));
    }

    #[test]
    fn a_mention_in_a_comment_is_no_entry() {
        // The names are only in comments, one of them behind code on the same line.
        let suite = "// visit(paths.home);
/* paths.signIn */
visit(paths.longer); // paths.home and paths.signIn
 * paths.home
";
        let findings = evaluate(PATHS, suite);
        let missing: Vec<&str> = findings.iter().map(|f| f.message.as_str()).collect();
        assert_eq!(missing.len(), 2, "{missing:?}");
        assert!(missing[0].contains("paths.home") && missing[1].contains("paths.signIn"));
    }

    #[test]
    fn good_example() {
        let suite = "visit(paths.home);\nvisit(paths.signIn);\nvisit(paths.longer);";
        assert!(evaluate(PATHS, suite).is_empty());
    }

    #[test]
    fn a_paths_file_without_addresses_is_a_finding() {
        assert_eq!(evaluate("export const other = 1;", "").len(), 1);
    }

    #[test]
    fn the_real_paths_are_read() {
        let repo = crate::repo::Repo::discover().unwrap();
        let text = fs::read_to_string(repo.path(PATHS_FILE)).unwrap();
        let names: Vec<String> = entries(&text).into_iter().map(|(name, _)| name).collect();
        assert!(names.contains(&"home".to_owned()));
        assert!(names.contains(&"appearance".to_owned()));
    }
}
