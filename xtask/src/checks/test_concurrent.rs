//! TEST-CONCURRENT: ten people working at once lose no text and never see a half-saved change
//! (architecture §11, ADR-0019, M3 WP-10).
//!
//! The rule has three parts, and this check keeps each from being deleted or switched off unseen:
//!
//! - the browser scenarios in `web/app/e2e/concurrent/` (ten Playwright contexts) exist under the
//!   names the work package gives them, and none is skipped, marked `fixme` or alone (`only`);
//!   the one scenario that needs a route another package adds may skip itself with a condition
//!   and a reason (`test.skip(condition, "reason")`), never without one, and not any more once
//!   the migration of that package is in the code (it would be skipped for ever without a sign);
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
/// The only spec that may skip itself, with a condition and a reason.
const MAY_SKIP: &str = "bulk.spec.ts";
/// Where the migrations are, and the one of the package that adds the route the skipping spec
/// waits for (WP-08, the README of M3): once it exists the scenario must run.
const MIGRATIONS_FOLDER: &str = "crates/bb-store/src/migrations";
const ROUTE_MIGRATION: &str = "m0011";
/// The folders with the browser tests and their helpers.
const E2E_FOLDERS: [&str; 3] = [
    "web/app/e2e",
    "web/app/e2e/support",
    "web/app/e2e/concurrent",
];
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
        findings.extend(route_has_arrived(&migrations_of(repo)?, &specs));
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

/// The names of the migration files.
fn migrations_of(repo: &crate::repo::Repo) -> anyhow::Result<Vec<String>> {
    sorted_files(repo, MIGRATIONS_FOLDER, |name| has_extension(name, "rs"))
}

/// Every TypeScript file of the browser tests and their helpers: path and code (comments blank).
fn e2e_files(repo: &crate::repo::Repo) -> anyhow::Result<Vec<(String, String)>> {
    let mut files = Vec::new();
    for folder in E2E_FOLDERS {
        for name in sorted_files(repo, folder, |name| has_extension(name, "ts"))? {
            let path = format!("{folder}/{name}");
            let code = strip_comments(&read_text(repo, &path)?);
            files.push((path, code));
        }
    }
    Ok(files)
}

/// Once the migration of the package that adds the route is in the code, the scenario that waited
/// for it must run: a guess at the route that does not fit would otherwise skip it for ever.
fn route_has_arrived(migrations: &[String], specs: &[(String, String)]) -> Vec<Finding> {
    let arrived = migrations
        .iter()
        .any(|name| name.starts_with(&format!("{ROUTE_MIGRATION}_")));
    let skips = specs
        .iter()
        .any(|(file, text)| file == MAY_SKIP && text.contains("test.skip("));
    if arrived && skips {
        vec![Finding::at(
            format!("{SPEC_FOLDER}/{MAY_SKIP}"),
            format!(
                "The migration {ROUTE_MIGRATION} (WP-08, the route that changes many issues) is in the code, but this scenario still skips itself. Adapt `bulkBody` and the route search in web/app/e2e/support/bulk.ts to the real route and take the skip out."
            ),
        )]
    } else {
        Vec::new()
    }
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

/// What switches a test of this file off: `.only`, `.fixme`, a skipped declaration, and a skip
/// without a condition; a skip with a condition is allowed in the one spec that waits for a route.
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
    for (index, _) in text.match_indices("test.skip(") {
        let argument = text[index + "test.skip(".len()..].trim_start();
        let declares_a_skipped_test = argument.starts_with(['"', '\'', '`', ')']);
        if file != MAY_SKIP || declares_a_skipped_test {
            findings.push(Finding::at(
                at.clone(),
                "A test is skipped. Only the scenario that waits for a route of another package may skip itself, with `test.skip(condition, reason)`.",
            ));
        }
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
    fn good_example_the_scenario_that_waits_for_a_route_may_skip_with_a_condition() {
        let mut specs = all_scenarios();
        specs.push((
            MAY_SKIP.to_owned(),
            "test.skip(route === undefined, \"not in this build\");".to_owned(),
        ));
        assert!(scenarios(&specs).is_empty());
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

    fn skipping_spec() -> Vec<(String, String)> {
        let mut specs = all_scenarios();
        specs.push((
            MAY_SKIP.to_owned(),
            "test.skip(late, \"later\");".to_owned(),
        ));
        specs
    }

    #[test]
    fn good_example_the_route_is_not_in_the_code_yet_and_the_scenario_skips() {
        let migrations = ["m0010_relations.rs".to_owned()];
        assert!(route_has_arrived(&migrations, &skipping_spec()).is_empty());
    }

    #[test]
    fn good_example_the_route_is_in_the_code_and_the_scenario_runs() {
        let mut specs = all_scenarios();
        specs.push((MAY_SKIP.to_owned(), "test(\"runs\", () => {});".to_owned()));
        let migrations = ["m0011_tags.rs".to_owned()];
        assert!(route_has_arrived(&migrations, &specs).is_empty());
    }

    #[test]
    fn bad_example_the_route_is_in_the_code_and_the_scenario_still_skips() {
        let migrations = ["m0011_tags.rs".to_owned()];
        let found = route_has_arrived(&migrations, &skipping_spec());
        assert_eq!(found.len(), 1);
        assert!(
            found[0].message.contains("bulkBody"),
            "{}",
            found[0].message
        );
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
            .chain(route_has_arrived(&migrations_of(&repo).unwrap(), &specs))
            .chain(tolerated_elsewhere(&e2e_files(&repo).unwrap()))
            .chain(ci_runs_the_load(&read_text(&repo, CI_FILE).unwrap()))
            .map(|finding| finding.message)
            .collect();
        assert!(found.is_empty(), "{found:?}");
    }
}
