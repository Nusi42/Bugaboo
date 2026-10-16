//! TEST-CONNECTIONS: a run of the browser tests never uses up the ports of the computer
//! (architecture §11, M4 WP-03).
//!
//! The guard itself is a Playwright reporter (`web/tooling/connection-guard.ts`) that counts the
//! connections in `TIME_WAIT` and `CLOSE_WAIT` of the servers of the tests during the run and fails the
//! run when the most it saw is over the limit `MOST_LEFTOVER_CONNECTIONS`. This check keeps each
//! part from being deleted or switched off unseen:
//!
//! - the Playwright config names the reporter;
//! - the reporter exists and fails the run (it returns the status `failed`);
//! - the limit is one named whole number, defined once, in `web/tooling/connections.ts`;
//! - the tests of the guard (`connections.test.ts`) exist and none is skipped or alone;
//! - no test of the browser is run again to make a failure go away: `retries` is set once, in the
//!   speed project, which competes with nothing (WP-03 task 7), and nowhere else: not in another
//!   place of the config, in the browser tests and their helpers, in `package.json` or in
//!   `cargo xtask ci`.

use super::{Check, Context, Finding};
use crate::rust_source::strip_comments;
use crate::source::read_text;

const CONFIG_FILE: &str = "web/tooling/playwright.config.ts";
const REPORTER_FILE: &str = "web/tooling/connection-guard.ts";
const LIMIT_FILE: &str = "web/tooling/connections.ts";
const TESTS_FILE: &str = "web/tooling/connections.test.ts";
/// The folder of the config, which names the reporter relative to itself.
const TOOLING_FOLDER: &str = "web/tooling/";
/// What makes the reporter fail the run.
const FAILS_THE_RUN: &str = "status: \"failed\"";
/// The name of the limit.
const LIMIT_NAME: &str = "MOST_LEFTOVER_CONNECTIONS";
/// What the speed project, the only one that may run tests again, is called in the config.
const SPEED_PROJECT: &str = "name: \"speed\"";
/// Where tests could be run again besides the config: the browser tests and their helpers, the
/// scripts of `package.json` and the steps of `cargo xtask ci`.
const E2E_FOLDER: &str = "web/app/e2e/";
const OTHER_FILES: [&str; 2] = ["package.json", "xtask/src/ci.rs"];
/// The words that run tests again.
const RETRY_WORDS: [&str; 2] = ["retries", "--retries"];

/// How the config names the reporter: its path from the folder of the config.
fn reporter_in_config() -> String {
    let file = REPORTER_FILE
        .strip_prefix(TOOLING_FOLDER)
        .unwrap_or(REPORTER_FILE);
    format!("\"./{file}\"")
}

pub(crate) struct TestConnections;

impl Check for TestConnections {
    fn id(&self) -> &'static str {
        "TEST-CONNECTIONS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let repo = context.repo;
        let read = |path: &str| read_text(repo, path).ok().map(|text| strip_comments(&text));
        let mut findings = config(read(CONFIG_FILE).as_deref());
        for path in repo.all_files()? {
            let searched = path.starts_with(E2E_FOLDER) || OTHER_FILES.contains(&path.as_str());
            if searched && let Some(text) = read(&path) {
                findings.extend(retries_elsewhere(&path, &text));
            }
        }
        findings.extend(reporter(read(REPORTER_FILE).as_deref()));
        findings.extend(limit(read(LIMIT_FILE).as_deref()));
        findings.extend(tests(read(TESTS_FILE).as_deref()));
        Ok(findings)
    }
}

/// The config names the reporter, and runs tests again only in the speed project.
fn config(text: Option<&str>) -> Vec<Finding> {
    let Some(text) = text else {
        return vec![Finding::at(
            CONFIG_FILE,
            "The Playwright config is missing.",
        )];
    };
    let mut findings = Vec::new();
    if !text.contains(&reporter_in_config()) {
        findings.push(Finding::at(
            CONFIG_FILE,
            format!(
                "The config does not use the reporter {}, so nothing counts the connections of the run.",
                reporter_in_config()
            ),
        ));
    }
    // Every `retries` must come after the name of the speed project, and there is at most one.
    let speed = text.find(SPEED_PROJECT);
    let retries: Vec<usize> = text.match_indices("retries").map(|(at, _)| at).collect();
    let all_in_speed = speed.is_some_and(|start| retries.iter().all(|at| *at > start));
    if retries.len() > 1 || (!retries.is_empty() && !all_in_speed) {
        findings.push(Finding::at(
            CONFIG_FILE,
            "Tests are run again outside the speed project, or more than once there. A failure is found and fixed, not run again until it passes.",
        ));
    }
    findings
}

/// A file besides the config that runs tests again.
fn retries_elsewhere(path: &str, text: &str) -> Vec<Finding> {
    if RETRY_WORDS.iter().any(|word| text.contains(word)) {
        vec![Finding::at(
            path,
            "Tests are run again here. Only the speed project of the Playwright config may do that; a failure is found and fixed, not run again until it passes.",
        )]
    } else {
        Vec::new()
    }
}

/// The reporter is there and fails the run.
fn reporter(text: Option<&str>) -> Vec<Finding> {
    match text {
        None => vec![Finding::at(
            REPORTER_FILE,
            "The reporter that counts the connections of the run is missing.",
        )],
        Some(text) if !text.contains(FAILS_THE_RUN) => vec![Finding::at(
            REPORTER_FILE,
            "The reporter does not fail the run (`status: \"failed\"`), so a run over the limit would pass.",
        )],
        Some(_) => Vec::new(),
    }
}

/// The limit is a named whole number above zero, defined once.
fn limit(text: Option<&str>) -> Vec<Finding> {
    let declaration = format!("export const {LIMIT_NAME} =");
    let Some(text) = text else {
        return vec![Finding::at(LIMIT_FILE, "The file of the limit is missing.")];
    };
    let values: Vec<&str> = text
        .match_indices(&declaration)
        .map(|(at, _)| {
            text[at + declaration.len()..]
                .split(';')
                .next()
                .unwrap_or_default()
                .trim()
        })
        .collect();
    let number = |value: &str| value.replace('_', "").parse::<u64>().is_ok_and(|n| n > 0);
    if values.len() == 1 && values.iter().all(|value| number(value)) {
        Vec::new()
    } else {
        vec![Finding::at(
            LIMIT_FILE,
            format!("`{LIMIT_NAME}` must be defined once, as a whole number above zero."),
        )]
    }
}

/// The tests of the guard are there, and none is switched off.
fn tests(text: Option<&str>) -> Vec<Finding> {
    let Some(text) = text else {
        return vec![Finding::at(
            TESTS_FILE,
            "The tests of the connection guard are missing.",
        )];
    };
    let off = [".only(", ".skip(", ".fixme(", ".todo("]
        .iter()
        .any(|banned| text.contains(banned));
    let over = text.contains("fails a run over the limit");
    let under = text.contains("passes a run that stays under the limit");
    if off || !over || !under {
        vec![Finding::at(
            TESTS_FILE,
            "The tests of the guard must include a run over the limit that fails and a run under it that passes, and none may be skipped or alone.",
        )]
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD_CONFIG: &str = "reporter: [[\"list\"], [\"./connection-guard.ts\"]], projects: [{ name: \"speed\", retries: 1 }]";

    #[test]
    fn good_example_the_config_uses_the_reporter_and_retries_only_the_speed_project() {
        assert!(config(Some(GOOD_CONFIG)).is_empty());
    }

    #[test]
    fn bad_example_the_config_without_the_reporter() {
        assert_eq!(config(Some("reporter: \"list\"")).len(), 1);
        assert_eq!(config(None).len(), 1);
    }

    #[test]
    fn bad_example_the_config_that_runs_tests_again() {
        let text = format!("{GOOD_CONFIG}, retries: 2");
        assert_eq!(config(Some(&text)).len(), 1);
    }

    #[test]
    fn bad_example_the_retries_sit_in_another_project_than_the_speed_project() {
        let text = "[\"./connection-guard.ts\"] projects: [{ name: \"desktop\", retries: 1 }, { name: \"speed\" }]";
        assert_eq!(config(Some(text)).len(), 1);
        let nowhere = "[\"./connection-guard.ts\"] projects: [{ retries: 1 }]";
        assert_eq!(config(Some(nowhere)).len(), 1);
    }

    #[test]
    fn good_example_a_config_without_any_retries() {
        assert!(config(Some("[\"./connection-guard.ts\"] projects: []")).is_empty());
    }

    #[test]
    fn bad_example_retries_in_the_browser_tests_the_scripts_or_the_ci_steps() {
        for text in [
            "test.describe.configure({ retries: 2 });",
            "\"test:e2e\": \"playwright test --retries=2\"",
            "&[\"run\", \"test:e2e\", \"--retries\", \"2\"]",
        ] {
            assert_eq!(
                retries_elsewhere("web/app/e2e/a.spec.ts", text).len(),
                1,
                "{text}"
            );
        }
    }

    #[test]
    fn good_example_a_file_that_runs_every_test_once() {
        assert!(retries_elsewhere("package.json", "\"test:e2e\": \"playwright test\"").is_empty());
    }

    #[test]
    fn the_reporter_is_named_from_the_folder_of_the_config() {
        assert_eq!(reporter_in_config(), "\"./connection-guard.ts\"");
    }

    #[test]
    fn good_example_the_reporter_fails_the_run() {
        assert!(reporter(Some("return { status: \"failed\" };")).is_empty());
    }

    #[test]
    fn bad_example_the_reporter_that_never_fails_or_is_gone() {
        assert_eq!(reporter(Some("return undefined;")).len(), 1);
        assert_eq!(reporter(None).len(), 1);
    }

    #[test]
    fn good_example_the_limit_is_one_whole_number() {
        assert!(limit(Some("export const MOST_LEFTOVER_CONNECTIONS = 2_500;")).is_empty());
    }

    #[test]
    fn bad_example_the_limit_is_missing_twice_defined_or_no_number() {
        for text in [
            "const x = 1;",
            "export const MOST_LEFTOVER_CONNECTIONS = 2500;\nexport const MOST_LEFTOVER_CONNECTIONS = 2600;",
            "export const MOST_LEFTOVER_CONNECTIONS = Infinity;",
            "export const MOST_LEFTOVER_CONNECTIONS = 0;",
        ] {
            assert_eq!(limit(Some(text)).len(), 1, "{text}");
        }
        assert_eq!(limit(None).len(), 1);
    }

    #[test]
    fn good_example_the_tests_of_the_guard_are_all_on() {
        let text =
            "it(\"passes a run that stays under the limit\"); it(\"fails a run over the limit\");";
        assert!(tests(Some(text)).is_empty());
    }

    #[test]
    fn bad_example_the_tests_of_the_guard_are_switched_off_or_missing() {
        for text in [
            "it.skip(\"passes a run that stays under the limit\"); it(\"fails a run over the limit\");",
            "it(\"passes a run that stays under the limit\");",
            "it(\"fails a run over the limit\");",
        ] {
            assert_eq!(tests(Some(text)).len(), 1, "{text}");
        }
        assert_eq!(tests(None).len(), 1);
    }

    #[test]
    fn the_repository_has_every_part_of_the_guard() {
        let repo = crate::repo::Repo::discover().unwrap();
        let read = |path: &str| {
            read_text(&repo, path)
                .ok()
                .map(|text| strip_comments(&text))
        };
        let elsewhere: Vec<_> = repo
            .all_files()
            .unwrap()
            .into_iter()
            .filter(|path| path.starts_with(E2E_FOLDER) || OTHER_FILES.contains(&path.as_str()))
            .flat_map(|path| {
                read(&path)
                    .map(|text| retries_elsewhere(&path, &text))
                    .unwrap_or_default()
            })
            .collect();
        let found: Vec<_> = config(read(CONFIG_FILE).as_deref())
            .into_iter()
            .chain(elsewhere)
            .chain(reporter(read(REPORTER_FILE).as_deref()))
            .chain(limit(read(LIMIT_FILE).as_deref()))
            .chain(tests(read(TESTS_FILE).as_deref()))
            .map(|finding| finding.message)
            .collect();
        assert!(found.is_empty(), "{found:?}");
    }
}
