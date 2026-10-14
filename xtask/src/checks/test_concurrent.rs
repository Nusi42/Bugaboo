//! TEST-CONCURRENT: ten people working at once lose no text and never see a half-saved change
//! (architecture §11, ADR-0019, M3 WP-10).
//!
//! The rule has three parts, and this check keeps each from being deleted or switched off unseen:
//!
//! - the browser scenarios in `web/app/e2e/concurrent/` (ten Playwright contexts) exist under the
//!   names the work package gives them, and none is skipped, marked `fixme` or alone (`only`);
//! - `tolerate(`, which lets a test expect the console lines of a browser that is cut off the
//!   network, is used in `concurrent/locks.spec.ts` only: anywhere else it would hide defects;
//! - the invariants test of the server (`test_concurrent_invariants_hold_after_a_run`, run on both
//!   databases by `cargo xtask ci`) is a `#[tokio::test]` that is not switched off;
//! - `cargo xtask ci` makes the short run of `cargo xtask load` (the API at speed, both databases).
//!
//! The long run is `cargo xtask load --nightly`, started by the nightly job.

use super::{Check, Context, Finding};
use crate::docs::source::sorted_files;
use crate::docs::text::has_extension;
use crate::rust_source::{strip, strip_comments};
use crate::source::read_text;

/// Where the browser scenarios are.
pub(super) const SPEC_FOLDER: &str = "web/app/e2e/concurrent";
/// The file of the server test.
const SERVER_TEST_FILE: &str = "crates/bb-server/src/tests/sign_in/concurrent.rs";
/// The test of the invariants after a run, against the database.
const SERVER_TEST: &str = "test_concurrent_invariants_hold_after_a_run";
/// The file that lists the steps of `cargo xtask ci`.
const CI_FILE: &str = "xtask/src/ci.rs";
/// What the step of the short run in `cargo xtask ci` must call.
const LOAD_COMMAND: &str = "\"load\"";
/// The folder with the browser tests and their helpers, with every folder below it.
const E2E_FOLDER: &str = "web/app/e2e";
/// The files that may name `tolerate(`: the spec of the locks, which uses it, and the fixtures,
/// which define it.
const MAY_TOLERATE: [&str; 2] = [
    "web/app/e2e/concurrent/locks.spec.ts",
    "web/app/e2e/support/fixtures.ts",
];

/// The browser scenarios the work package names (the work package, architecture §11).
pub(super) const SCENARIOS: [&str; 5] = [
    "iss_17_three_people_edit_one_description_and_the_first_text_is_kept",
    "iss_17_a_lock_holders_network_drops_and_the_text_comes_back_with_compare_and_save",
    "iss_17_two_people_change_one_status_within_a_second_and_one_of_them_is_told",
    "iss_17_ten_people_change_ten_fields_of_one_issue_and_every_change_lands_once",
    "iss_09_changing_many_issues_skips_the_one_somebody_is_editing_and_names_it",
];

pub(crate) struct TestConcurrent;

impl Check for TestConcurrent {
    fn id(&self) -> &'static str {
        "TEST-CONCURRENT"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let repo = context.repo;
        let specs = specs_of(repo)?;
        let server = read_text(repo, SERVER_TEST_FILE).ok();
        let ci = read_text(repo, CI_FILE)?;
        let mut findings = scenarios(&specs);
        findings.extend(tolerated_elsewhere(&e2e_files(repo)?));
        findings.extend(server_test(server.as_deref()));
        findings.extend(ci_runs_the_load(&ci));
        Ok(findings)
    }
}

/// The spec files of the folder with their code (comments blank, strings kept): name and text.
pub(super) fn specs_of(repo: &crate::repo::Repo) -> anyhow::Result<Vec<(String, String)>> {
    let names =
        sorted_files(repo, SPEC_FOLDER, |name| name.ends_with(".spec.ts")).unwrap_or_default();
    let mut specs = Vec::new();
    for name in names {
        let text = read_text(repo, &format!("{SPEC_FOLDER}/{name}"))?;
        specs.push((name, strip_comments(&text)));
    }
    Ok(specs)
}

/// The TypeScript files of the browser tests and their helpers, in any folder below
/// [`E2E_FOLDER`], among all the files of the repository.
fn e2e_paths(all: &[String]) -> Vec<&String> {
    all.iter()
        .filter(|path| {
            path.strip_prefix(E2E_FOLDER)
                .is_some_and(|rest| rest.starts_with('/'))
                && has_extension(path, "ts")
        })
        .collect()
}

/// Every TypeScript file of the browser tests and their helpers: path and code (comments blank).
fn e2e_files(repo: &crate::repo::Repo) -> anyhow::Result<Vec<(String, String)>> {
    let mut files = Vec::new();
    for path in e2e_paths(&repo.all_files()?) {
        let code = strip_comments(&read_text(repo, path)?);
        files.push((path.clone(), code));
    }
    Ok(files)
}

/// `tolerate(` outside the files that may name it.
fn tolerated_elsewhere(files: &[(String, String)]) -> Vec<Finding> {
    files
        .iter()
        .filter(|(path, code)| !MAY_TOLERATE.contains(&path.as_str()) && code.contains("tolerate("))
        .map(|(path, _)| {
            Finding::at(
                path.clone(),
                "`tolerate(` lets a test expect the console lines of a browser that is cut off the network. Only concurrent/locks.spec.ts may use it; anywhere else it would hide defects.",
            )
        })
        .collect()
}

/// Whether some spec declares a test of this name: `test("name"`.
pub(super) fn declares(specs: &[(String, String)], name: &str) -> bool {
    let declaration = format!("test(\"{name}\"");
    specs.iter().any(|(_, text)| text.contains(&declaration))
}

/// The scenarios are there, and nothing in the folder is switched off.
fn scenarios(specs: &[(String, String)]) -> Vec<Finding> {
    let mut findings: Vec<Finding> = SCENARIOS
        .iter()
        .filter(|name| !declares(specs, name))
        .map(|name| {
            Finding::at(
                SPEC_FOLDER,
                format!("The scenario `{name}` is missing from the browser tests of ten people."),
            )
        })
        .collect();
    for (file, text) in specs {
        findings.extend(switched_off(file, text));
    }
    findings
}

/// What switches a test of this file off: `.only`, `.fixme`, and a skip with or without a
/// condition.
fn switched_off(file: &str, text: &str) -> Vec<Finding> {
    let at = format!("{SPEC_FOLDER}/{file}");
    let mut findings = Vec::new();
    for banned in [
        ".only(",
        "test.fixme(",
        ".describe.skip(",
        ".describe.fixme(",
    ] {
        if text.contains(banned) {
            findings.push(Finding::at(
                at.clone(),
                format!(
                    "`{}` switches tests off or runs only some of them.",
                    banned.trim_end_matches('(')
                ),
            ));
        }
    }
    for _ in text.match_indices("test.skip(") {
        findings.push(Finding::at(
            at.clone(),
            "A test is skipped. A scenario of ten people runs in every build; a route that is not there yet is waited for in the work package, not skipped.",
        ));
    }
    findings
}

/// The server test is a `#[tokio::test…]` that is not switched off.
fn server_test(file: Option<&str>) -> Vec<Finding> {
    let missing = || {
        vec![Finding::at(
            SERVER_TEST_FILE,
            format!("The test `{SERVER_TEST}` is missing, or is no `#[tokio::test]` any more."),
        )]
    };
    let Some(file) = file else {
        return missing();
    };
    let code = strip(file);
    let needle = format!("async fn {SERVER_TEST}(");
    let on = code.match_indices(&needle).any(|(at, _)| {
        let before = &code[..at];
        let start = before.rfind(['}', ';', '{']).map_or(0, |end| end + 1);
        let attributes = &before[start..];
        attributes.contains("#[tokio::test")
            && !attributes.contains("ignore")
            && !attributes.contains("cfg(")
    });
    if on { Vec::new() } else { missing() }
}

/// `cargo xtask ci` runs `cargo xtask load` as a step.
fn ci_runs_the_load(ci: &str) -> Vec<Finding> {
    if strip_comments(ci).contains(LOAD_COMMAND) {
        Vec::new()
    } else {
        vec![Finding::at(
            CI_FILE,
            "`cargo xtask ci` does not run the short run of `cargo xtask load` (both databases).",
        )]
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::*;

    fn all_scenarios() -> Vec<(String, String)> {
        let mut text = String::new();
        for name in SCENARIOS {
            let _ = writeln!(text, "test(\"{name}\", async () => {{}});");
        }
        vec![("scenarios.spec.ts".to_owned(), text)]
    }

    #[test]
    fn good_example_every_scenario_is_declared_and_nothing_is_switched_off() {
        assert!(scenarios(&all_scenarios()).is_empty());
    }

    #[test]
    fn bad_example_a_scenario_is_missing() {
        let mut specs = all_scenarios();
        specs[0].1 = specs[0].1.replace(SCENARIOS[0], "another_name");
        let found = scenarios(&specs);
        assert_eq!(found.len(), 1);
        assert!(
            found[0].message.contains(SCENARIOS[0]),
            "{}",
            found[0].message
        );
    }

    #[test]
    fn bad_example_a_scenario_that_is_skipped_or_alone() {
        for banned in [
            "test.only(\"x\", () => {});",
            "test.fixme(\"x\", () => {});",
            "test.skip(\"x\", () => {});",
            "test.skip();",
            "test.describe.skip(\"x\", () => {});",
        ] {
            let mut specs = all_scenarios();
            specs[0].1.push_str(banned);
            assert!(!scenarios(&specs).is_empty(), "{banned}");
        }
    }

    #[test]
    fn bad_example_another_scenario_skips_itself_with_a_condition() {
        let mut specs = all_scenarios();
        specs[0].1.push_str("test.skip(slow, \"too slow\");");
        assert_eq!(scenarios(&specs).len(), 1);
    }

    #[test]
    fn bad_example_a_scenario_in_a_comment_does_not_count() {
        let specs = vec![(
            "a.spec.ts".to_owned(),
            strip_comments(&format!("// test(\"{}\", () => {{}});", SCENARIOS[0])),
        )];
        assert!(!declares(&specs, SCENARIOS[0]));
    }

    #[test]
    fn good_example_tolerate_in_the_spec_of_the_locks_and_in_the_fixtures() {
        let files = [
            (
                MAY_TOLERATE[0].to_owned(),
                "const net = tolerate(holder.context(), lines);".to_owned(),
            ),
            (
                MAY_TOLERATE[1].to_owned(),
                "tolerate(context, lines) {}".to_owned(),
            ),
            (
                "web/app/e2e/issues.spec.ts".to_owned(),
                "nothing like it here".to_owned(),
            ),
        ];
        assert!(tolerated_elsewhere(&files).is_empty());
    }

    #[test]
    fn bad_example_tolerate_in_a_folder_that_did_not_exist_before() {
        let all = [
            "web/app/e2e/deeper/new/crowd.spec.ts".to_owned(),
            "web/app/e2e/support/more/helper.ts".to_owned(),
            "web/app/e2e/notes.md".to_owned(),
            "web/app/src/e2e/other.ts".to_owned(),
            "web/app/e2e-other/x.ts".to_owned(),
        ];
        let found = e2e_paths(&all);
        assert_eq!(
            found,
            [
                "web/app/e2e/deeper/new/crowd.spec.ts",
                "web/app/e2e/support/more/helper.ts"
            ]
        );
        let files: Vec<(String, String)> = found
            .into_iter()
            .map(|path| (path.clone(), "tolerate(page.context(), /x/u);".to_owned()))
            .collect();
        assert_eq!(tolerated_elsewhere(&files).len(), 2);
    }

    #[test]
    fn bad_example_tolerate_in_another_spec_or_a_helper() {
        for path in [
            "web/app/e2e/issues.spec.ts",
            "web/app/e2e/concurrent/changes.spec.ts",
            "web/app/e2e/support/crowd-pages.ts",
        ] {
            let files = [(
                path.to_owned(),
                "tolerate(page.context(), /x/u);".to_owned(),
            )];
            assert_eq!(tolerated_elsewhere(&files).len(), 1, "{path}");
        }
    }

    #[test]
    fn good_example_the_server_test_is_a_tokio_test_with_options() {
        let code = format!(
            "#[tokio::test(flavor = \"multi_thread\", worker_threads = 4)]\nasync fn {SERVER_TEST}() -> TestResult {{ Ok(()) }}"
        );
        assert!(server_test(Some(&code)).is_empty());
    }

    #[test]
    fn bad_example_the_server_test_is_missing_ignored_or_no_test() {
        assert_eq!(server_test(None).len(), 1);
        for attributes in [
            "#[tokio::test]\n#[ignore]",
            "#[ignore]\n#[tokio::test]",
            "#[tokio::test]\n#[cfg(any())]",
            "",
        ] {
            let code = format!("{attributes}\nasync fn {SERVER_TEST}() {{}}");
            assert_eq!(server_test(Some(&code)).len(), 1, "{attributes}");
        }
    }

    #[test]
    fn bad_example_ci_that_does_not_run_the_load() {
        assert_eq!(
            ci_runs_the_load("Step::new(\"x\", p, &[\"check\"])").len(),
            1
        );
        assert!(ci_runs_the_load("Step::new(\"x\", p, &[\"load\"])").is_empty());
    }

    #[test]
    fn the_repository_has_every_part_of_the_rule() {
        let repo = crate::repo::Repo::discover().unwrap();
        let specs = specs_of(&repo).unwrap();
        let found: Vec<_> = scenarios(&specs)
            .into_iter()
            .chain(server_test(
                read_text(&repo, SERVER_TEST_FILE).ok().as_deref(),
            ))
            .chain(tolerated_elsewhere(&e2e_files(&repo).unwrap()))
            .chain(ci_runs_the_load(&read_text(&repo, CI_FILE).unwrap()))
            .map(|finding| finding.message)
            .collect();
        assert!(found.is_empty(), "{found:?}");
    }
}
