//! TEST-EDIT-LOCKS: the tests of edit locks, drafts and confirmed saves exist (ISS-17, ISS-11,
//! ADR-0019).
//!
//! The server part is the folder `crates/bb-server/src/tests/sign_in/editing/`: each test runs on
//! both databases against a kind of record that exists only in tests. This check fails when a test
//! that the work package names is missing or has lost its `#[tokio::test]`, so the guarantees
//! (a lock frees itself, *Edit anyway* keeps the first person's text, a lost lock never drops
//! text, a stale version tells who changed what, a draft is private and goes) cannot be deleted
//! unseen. A named test counts as missing when it is switched off, in the ways of the one
//! implementation (`required_tests`: `#[ignore]` in any form, a `#[cfg(…)]` on the function or on a
//! module around it, a file that no `mod` reaches). The suite itself runs in
//! `cargo xtask ci` (the Rust tests). The browser part is the scenarios in
//! `web/app/e2e/concurrent/` where several people edit one description and where a lock holder
//! goes offline (`BROWSER`); they must be declared there and not switched off (TEST-CONCURRENT
//! looks at the rest of that folder).

use super::required_tests::{Required, crate_sources, evaluate};
use super::test_concurrent;
use super::{Check, Context, Finding};

/// The tests of the server part.
const SUITE: &str = "crates/bb-server/src/tests/sign_in/editing";

/// What the suite must prove, by the name of the test that proves it (the work package names them).
pub(super) const REQUIRED: [Required; 8] = [
    Required::anywhere("iss_17_a_second_person_sees_who_is_editing_or_someone").within(SUITE),
    Required::anywhere("iss_17_a_lock_frees_itself_a_minute_after_the_last_heartbeat")
        .within(SUITE),
    Required::anywhere("iss_17_edit_anyway_keeps_the_first_persons_text_as_their_draft")
        .within(SUITE),
    Required::anywhere("iss_17_a_save_with_a_lost_lock_is_refused_and_the_text_kept").within(SUITE),
    Required::anywhere("iss_17_a_stale_version_answers_who_changed_what").within(SUITE),
    Required::anywhere("iss_11_a_draft_is_readable_only_by_its_author").within(SUITE),
    Required::anywhere("iss_11_a_draft_goes_after_save_and_after_its_keep_period").within(SUITE),
    Required::anywhere("sec_17_letting_go_of_a_stuck_lock_is_in_the_security_log").within(SUITE),
];

/// What the browser must show (M3 WP-10): the first two scenarios of ten people at once.
const BROWSER: [&str; 2] = [test_concurrent::SCENARIOS[0], test_concurrent::SCENARIOS[1]];

pub(crate) struct TestEditLocks;

impl Check for TestEditLocks {
    fn id(&self) -> &'static str {
        "TEST-EDIT-LOCKS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let mut findings = evaluate(
            &crate_sources(context.repo)?,
            &REQUIRED,
            "The edit locks and drafts (ADR-0019) are held by it;",
        );
        findings.extend(browser(&test_concurrent::specs_of(context.repo)?));
        Ok(findings)
    }
}

/// The browser scenarios of the locks are declared.
fn browser(specs: &[(String, String)]) -> Vec<Finding> {
    BROWSER
        .iter()
        .filter(|name| !test_concurrent::declares(specs, name))
        .map(|name| {
            Finding::at(
                test_concurrent::SPEC_FOLDER,
                format!("The browser scenario `{name}` of the edit locks is missing."),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::super::required_tests::examples;
    use super::*;

    #[test]
    fn bad_example_a_missing_browser_scenario() {
        let specs = vec![(
            "locks.spec.ts".to_owned(),
            format!("test(\"{}\", () => {{}});", BROWSER[0]),
        )];
        let found = browser(&specs);
        assert_eq!(found.len(), 1);
        assert!(found[0].message.contains(BROWSER[1]));
    }

    #[test]
    fn good_example_both_browser_scenarios_are_declared() {
        let mut text = String::new();
        for name in BROWSER {
            let _ = write!(text, "test(\"{name}\", () => {{}});");
        }
        assert!(browser(&[("locks.spec.ts".to_owned(), text)]).is_empty());
    }

    #[test]
    fn bad_example_a_suite_that_lost_a_test_or_a_folder() {
        let name = REQUIRED[0].name;
        let sources = vec![
            (
                "crates/bb-server/src/lib.rs".to_owned(),
                "mod tests;
"
                .to_owned(),
            ),
            (
                "crates/bb-server/src/tests.rs".to_owned(),
                format!(
                    "#[tokio::test]
async fn {name}() {{}}
"
                ),
            ),
        ];
        // The test is there, but not in the folder of the suite.
        assert_eq!(evaluate(&sources, &REQUIRED[..1], "").len(), 1);
        assert_eq!(evaluate(&[], &REQUIRED[..1], "").len(), 1);
    }

    /// The suite as a crate: the folder declares each file of `declared`, and holds all of
    /// `files`; every required test is in `editing/all.rs`.
    fn suite(declared: &[&str], files: &[&str]) -> Vec<(String, String)> {
        let folder = format!("{SUITE}/");
        let mut tests = String::new();
        for each in &REQUIRED {
            let _ = writeln!(
                tests,
                "#[tokio::test]
async fn {}() {{}}",
                each.name
            );
        }
        let mut sources = vec![
            (
                "crates/bb-server/src/lib.rs".to_owned(),
                "#[path = \"tests/sign_in/editing/mod.rs\"]
mod editing;
"
                .to_owned(),
            ),
            (format!("{folder}all.rs"), tests),
        ];
        let mut root = "mod all;
"
        .to_owned();
        for name in declared {
            let _ = writeln!(root, "mod {name};");
        }
        sources.push((format!("{folder}mod.rs"), root));
        for name in files {
            sources.push((
                format!("{folder}{name}.rs"),
                "#[tokio::test]
async fn stale_views_are_shown() {}
"
                .to_owned(),
            ));
        }
        sources
    }

    #[test]
    fn bad_example_a_test_file_of_the_suite_that_mod_rs_does_not_declare() {
        let found = evaluate(&suite(&[], &["stale_views"]), &REQUIRED, "");
        assert_eq!(found.len(), 1);
        assert!(
            found[0]
                .location
                .as_deref()
                .is_some_and(|at| at.ends_with("editing/stale_views.rs"))
        );
    }

    #[test]
    fn good_example_every_test_file_of_the_suite_is_declared() {
        assert!(evaluate(&suite(&["stale_views"], &["stale_views"]), &REQUIRED, "").is_empty());
    }

    #[test]
    fn good_example_the_suite_of_this_repository_has_every_named_test() {
        examples::this_repository_has_every_test(&REQUIRED);
    }
}
