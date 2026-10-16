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
//! - no test of the browser is run again to make a failure go away: `retries` is set only for the
//!   speed project, which competes with nothing (WP-03 task 7).

use super::{Check, Context, Finding};
use crate::rust_source::strip_comments;
use crate::source::read_text;

const CONFIG_FILE: &str = "web/tooling/playwright.config.ts";
const REPORTER_FILE: &str = "web/tooling/connection-guard.ts";
const LIMIT_FILE: &str = "web/tooling/connections.ts";
const TESTS_FILE: &str = "web/tooling/connections.test.ts";
/// How the config names the reporter.
const REPORTER_IN_CONFIG: &str = "\"./connection-guard.ts\"";
/// What makes the reporter fail the run.
const FAILS_THE_RUN: &str = "status: \"failed\"";
/// The name of the limit.
const LIMIT_NAME: &str = "MOST_LEFTOVER_CONNECTIONS";
/// The most places in the config that may say `retries`: the speed project.
const MOST_RETRIES: usize = 1;

pub(crate) struct TestConnections;

impl Check for TestConnections {
    fn id(&self) -> &'static str {
        "TEST-CONNECTIONS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let repo = context.repo;
        let read = |path: &str| read_text(repo, path).ok().map(|text| strip_comments(&text));
        let mut findings = config(read(CONFIG_FILE).as_deref());
        findings.extend(reporter(read(REPORTER_FILE).as_deref()));
        findings.extend(limit(read(LIMIT_FILE).as_deref()));
        findings.extend(tests(read(TESTS_FILE).as_deref()));
        Ok(findings)
    }
}

/// The config names the reporter, and does not run tests again to hide a failure.
fn config(text: Option<&str>) -> Vec<Finding> {
    let Some(text) = text else {
        return vec![Finding::at(
            CONFIG_FILE,
            "The Playwright config is missing.",
        )];
    };
    let mut findings = Vec::new();
    if !text.contains(REPORTER_IN_CONFIG) {
        findings.push(Finding::at(
            CONFIG_FILE,
            "The config does not use the reporter `./connection-guard.ts`, so nothing counts the connections of the run.",
        ));
    }
    let retries = text.matches("retries").count();
    if retries > MOST_RETRIES {
        findings.push(Finding::at(
            CONFIG_FILE,
            "Tests are run again in more than the speed project. A failure is found and fixed, not run again until it passes.",
        ));
    }
    findings
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

    const GOOD_CONFIG: &str =
        "reporter: [[\"list\"], [\"./connection-guard.ts\"]], projects: [{ retries: 1 }]";

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
        let found: Vec<_> = config(read(CONFIG_FILE).as_deref())
            .into_iter()
            .chain(reporter(read(REPORTER_FILE).as_deref()))
            .chain(limit(read(LIMIT_FILE).as_deref()))
            .chain(tests(read(TESTS_FILE).as_deref()))
            .map(|finding| finding.message)
            .collect();
        assert!(found.is_empty(), "{found:?}");
    }
}
