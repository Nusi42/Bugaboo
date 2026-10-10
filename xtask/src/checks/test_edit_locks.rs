//! TEST-EDIT-LOCKS: the tests of edit locks, drafts and confirmed saves exist (ISS-17, ISS-11,
//! ADR-0019).
//!
//! The server part is the folder `crates/bb-server/src/tests/sign_in/editing/`: each test runs on
//! both databases against a kind of record that exists only in tests. This check fails when a test
//! that the work package names is missing or has lost its `#[tokio::test]`, so the guarantees
//! (a lock frees itself, *Edit anyway* keeps the first person's text, a lost lock never drops
//! text, a stale version tells who changed what, a draft is private and goes) cannot be deleted
//! unseen; a named test that carries `#[ignore]` counts as missing. The suite itself runs in `cargo xtask ci` (the Rust tests); the browser scenarios come
//! with M3 WP-10.

use super::{Check, Context, Finding};
use crate::docs::source::sorted_files;
use crate::docs::text::has_extension;
use crate::rust_source::strip;
use crate::source::read_text;

/// The tests of the server part.
const SUITE: &str = "crates/bb-server/src/tests/sign_in/editing";

/// What the suite must prove, by the name of the test that proves it (the work package names them).
const REQUIRED: [&str; 8] = [
    "iss_17_a_second_person_sees_who_is_editing_or_someone",
    "iss_17_a_lock_frees_itself_a_minute_after_the_last_heartbeat",
    "iss_17_edit_anyway_keeps_the_first_persons_text_as_their_draft",
    "iss_17_a_save_with_a_lost_lock_is_refused_and_the_text_kept",
    "iss_17_a_stale_version_answers_who_changed_what",
    "iss_11_a_draft_is_readable_only_by_its_author",
    "iss_11_a_draft_goes_after_save_and_after_its_keep_period",
    "sec_17_releasing_a_stuck_lock_is_in_the_security_log",
];

pub(crate) struct TestEditLocks;

impl Check for TestEditLocks {
    fn id(&self) -> &'static str {
        "TEST-EDIT-LOCKS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        Ok(evaluate(suite_text(context.repo).as_deref()))
    }
}

/// The code of every file of the suite, or nothing when the folder is not there.
fn suite_text(repo: &crate::repo::Repo) -> Option<String> {
    let mut suite = String::new();
    for name in sorted_files(repo, SUITE, |name| has_extension(name, "rs")).ok()? {
        suite.push_str(&read_text(repo, &format!("{SUITE}/{name}")).ok()?);
        suite.push('\n');
    }
    (!suite.is_empty()).then_some(suite)
}

/// Whether the code has an `async fn` with this name below a `#[tokio::test]` that is not
/// switched off. The attributes of the function are everything between the end of the item before
/// it (`}` or `;`) and the function; comments are already blank in `code`, so a doc comment between
/// the attributes does not hide one, and an attribute over several lines is read whole. `ignore`
/// anywhere in them (`#[ignore]`, `#[ignore = "…"]`, `#[cfg_attr(…, ignore)]`) counts as off.
fn is_a_test(code: &str, name: &str) -> bool {
    let needle = format!("async fn {name}(");
    code.match_indices(&needle).any(|(at, _)| {
        let before = &code[..at];
        let start = before.rfind(['}', ';']).map_or(0, |end| end + 1);
        let attributes = &before[start..];
        attributes.contains("#[tokio::test]") && !attributes.contains("ignore")
    })
}

fn evaluate(suite: Option<&str>) -> Vec<Finding> {
    let Some(suite) = suite else {
        return vec![Finding::at(
            SUITE,
            "The folder with the tests of edit locks is missing (ADR-0019, ISS-17).",
        )];
    };
    let code = strip(suite);
    REQUIRED
        .iter()
        .filter(|name| !is_a_test(&code, name))
        .map(|name| {
            Finding::at(
                SUITE,
                format!("The test `{name}` is missing, or is no `#[tokio::test]` any more."),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::*;

    fn suite_with(names: &[&str]) -> String {
        let mut suite = String::new();
        for name in names {
            let _ = writeln!(
                suite,
                "#[tokio::test]
async fn {name}() -> TestResult {{ Ok(()) }}"
            );
        }
        suite
    }

    #[test]
    fn good_example_a_suite_with_every_named_test() {
        assert!(evaluate(Some(&suite_with(&REQUIRED))).is_empty());
    }

    #[test]
    fn bad_example_a_suite_that_lost_a_test() {
        let found = evaluate(Some(&suite_with(&REQUIRED[1..])));
        assert_eq!(found.len(), 1);
        assert!(
            found[0].message.contains(REQUIRED[0]),
            "{}",
            found[0].message
        );
    }

    #[test]
    fn bad_example_a_test_that_is_no_test_any_more_or_hides_in_a_comment() {
        let mut suite = suite_with(&REQUIRED[1..]);
        let name = REQUIRED[0];
        // The same name without the attribute, and the attribute and name in a comment.
        let _ = writeln!(suite, "async fn {name}() {{}}");
        let _ = writeln!(suite, "// #[tokio::test]\n// async fn {name}() {{}}");
        assert_eq!(evaluate(Some(&suite)).len(), 1);
    }

    #[test]
    fn bad_example_a_test_that_is_switched_off() {
        let name = REQUIRED[0];
        // Each way of switching a test off, one after the other: the name is then missing.
        for attributes in [
            "#[tokio::test]
#[ignore]",
            "#[ignore]
#[tokio::test]",
            "#[tokio::test]
#[ignore = \"later\"]",
            "#[tokio::test]
#[ignore =
    \"later\"]",
            "#[tokio::test]
#[cfg_attr(windows, ignore)]",
            "#[tokio::test]
/// Later.
#[ignore]",
            "#[tokio::test]
#[cfg_attr(
    windows,
    ignore
)]",
        ] {
            let mut suite = suite_with(&REQUIRED[1..]);
            let _ = writeln!(
                suite,
                "{attributes}
async fn {name}() {{}}"
            );
            let found = evaluate(Some(&suite));
            assert_eq!(found.len(), 1, "{attributes}");
        }
    }

    #[test]
    fn good_example_doc_comments_and_other_attributes_between_are_fine() {
        let name = REQUIRED[0];
        let mut suite = suite_with(&REQUIRED[1..]);
        let _ = writeln!(
            suite,
            "/// What it proves.
#[tokio::test]
#[doc(hidden)]
async fn {name}() {{}}"
        );
        assert!(evaluate(Some(&suite)).is_empty());
    }

    #[test]
    fn bad_example_a_missing_suite() {
        assert_eq!(evaluate(None).len(), 1);
    }

    #[test]
    fn the_suite_of_this_repository_has_every_named_test() {
        let repo = crate::repo::Repo::discover().unwrap();
        let text = suite_text(&repo).unwrap();
        let found: Vec<_> = evaluate(Some(&text))
            .into_iter()
            .map(|finding| finding.message)
            .collect();
        assert!(found.is_empty(), "{found:?}");
    }
}
