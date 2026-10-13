//! TEST-EDIT-LOCKS: the tests of edit locks, drafts and confirmed saves exist (ISS-17, ISS-11,
//! ADR-0019).
//!
//! The server part is the folder `crates/bb-server/src/tests/sign_in/editing/`: each test runs on
//! both databases against a kind of record that exists only in tests. This check fails when a test
//! that the work package names is missing or has lost its `#[tokio::test]`, so the guarantees
//! (a lock frees itself, *Edit anyway* keeps the first person's text, a lost lock never drops
//! text, a stale version tells who changed what, a draft is private and goes) cannot be deleted
//! unseen. A named test counts as missing when it is switched off: `#[ignore]` in any form, a
//! `#[cfg(…)]` on the function or on a module around it, or a file that the folder's `mod.rs`
//! does not declare (or declares behind a `#[cfg(…)]`). The suite itself runs in
//! `cargo xtask ci` (the Rust tests). The browser part is the scenarios in
//! `web/app/e2e/concurrent/` where several people edit one description and where a lock holder
//! goes offline (`BROWSER`); they must be declared there and not switched off (TEST-CONCURRENT
//! looks at the rest of that folder).

use super::test_concurrent;
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

/// What the browser must show (M3 WP-10): the first two scenarios of ten people at once.
const BROWSER: [&str; 2] = [test_concurrent::SCENARIOS[0], test_concurrent::SCENARIOS[1]];

pub(crate) struct TestEditLocks;

impl Check for TestEditLocks {
    fn id(&self) -> &'static str {
        "TEST-EDIT-LOCKS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let mut findings = match suite_text(context.repo) {
            Some((suite, mut unused)) => {
                let mut findings = evaluate(Some(&suite));
                findings.append(&mut unused);
                findings
            }
            None => evaluate(None),
        };
        findings.extend(browser(&test_concurrent::specs_of(context.repo)?));
        Ok(findings)
    }
}

/// The code of every file of the suite that is part of it, or nothing when the folder is not
/// there, and a finding for each file that is not part of it.
fn suite_text(repo: &crate::repo::Repo) -> Option<(String, Vec<Finding>)> {
    let names = sorted_files(repo, SUITE, |name| has_extension(name, "rs")).ok()?;
    let declared = declared_modules(&strip(&read_text(repo, &format!("{SUITE}/mod.rs")).ok()?));
    let mut suite = String::new();
    let mut findings = Vec::new();
    for name in names {
        let stem = name.trim_end_matches(".rs");
        if name != "mod.rs" && !declared.iter().any(|module| module == stem) {
            findings.push(Finding::at(
                format!("{SUITE}/{name}"),
                "This file is not declared in the `mod.rs` of the folder (or only behind a `#[cfg(…)]`), so its tests never run.",
            ));
            continue;
        }
        suite.push_str(&read_text(repo, &format!("{SUITE}/{name}")).ok()?);
        suite.push('\n');
    }
    (!suite.is_empty()).then_some((suite, findings))
}

/// The modules a file declares with `mod name;` and no `cfg` in their attributes.
fn declared_modules(code: &str) -> Vec<String> {
    let mut modules = Vec::new();
    for (at, _) in code.match_indices("mod ") {
        let rest = &code[at + 4..];
        let Some(end) = rest.find([';', '{']) else {
            continue;
        };
        if !rest[end..].starts_with(';') || rest[..end].trim().contains(char::is_whitespace) {
            continue;
        }
        if switched_off_by_cfg(&code[..at]) {
            continue;
        }
        modules.push(rest[..end].trim().to_owned());
    }
    modules
}

/// The attributes of the item that starts right after `before`: everything since the end of the
/// item before it. `cfg(` in them (`#[cfg(any())]`, `#[cfg(not(test))]`) can switch the item off.
fn switched_off_by_cfg(before: &str) -> bool {
    let start = before.rfind(['}', ';', '{']).map_or(0, |end| end + 1);
    before[start..].contains("cfg(")
}

/// Whether a block that encloses the position `at` has `cfg(` in its attributes (a `mod` that is
/// switched off around the tests).
fn inside_switched_off_block(code: &str, at: usize) -> bool {
    let mut open: Vec<bool> = Vec::new();
    let mut header_start = 0;
    for (index, character) in code[..at].char_indices() {
        match character {
            '{' => {
                open.push(code[header_start..index].contains("cfg("));
                header_start = index + 1;
            }
            '}' => {
                open.pop();
                header_start = index + 1;
            }
            ';' => header_start = index + 1,
            _ => {}
        }
    }
    open.contains(&true)
}

/// Whether the code has an `async fn` with this name below a `#[tokio::test]` that is not
/// switched off. The attributes of the function are everything between the end of the item before
/// it (`}` or `;`) and the function; comments are already blank in `code`, so a doc comment between
/// the attributes does not hide one, and an attribute over several lines is read whole. `ignore`
/// anywhere in them (`#[ignore]`, `#[ignore = "…"]`, `#[cfg_attr(…, ignore)]`) counts as off, and
/// so does `cfg(` in them or in the attributes of a block around the function.
fn is_a_test(code: &str, name: &str) -> bool {
    let needle = format!("async fn {name}(");
    code.match_indices(&needle).any(|(at, _)| {
        let before = &code[..at];
        let start = before.rfind(['}', ';', '{']).map_or(0, |end| end + 1);
        let attributes = &before[start..];
        attributes.contains("#[tokio::test]")
            && !attributes.contains("ignore")
            && !attributes.contains("cfg(")
            && !inside_switched_off_block(code, at)
    })
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
    fn bad_example_a_test_behind_a_cfg_or_in_a_module_behind_one() {
        let name = REQUIRED[0];
        for text in [
            format!(
                "#[cfg(any())]
#[tokio::test]
async fn {name}() {{}}"
            ),
            format!(
                "#[tokio::test]
#[cfg(not(test))]
async fn {name}() {{}}"
            ),
            format!(
                "#[cfg(any())]
mod later {{
#[tokio::test]
async fn {name}() {{}}
}}"
            ),
            format!(
                "#[cfg(any())]
mod later {{
mod inner {{
#[tokio::test]
async fn {name}() {{}}
}}
}}"
            ),
        ] {
            let mut suite = suite_with(&REQUIRED[1..]);
            let _ = writeln!(suite, "{text}");
            assert_eq!(evaluate(Some(&suite)).len(), 1, "{text}");
        }
    }

    #[test]
    fn good_example_a_module_without_a_cfg_around_the_tests() {
        let name = REQUIRED[0];
        let mut suite = suite_with(&REQUIRED[1..]);
        let _ = writeln!(
            suite,
            "mod ordinary {{
#[tokio::test]
async fn {name}() {{ if true {{}} }}
}}"
        );
        assert!(evaluate(Some(&suite)).is_empty());
    }

    #[test]
    fn bad_example_a_file_the_folder_does_not_declare() {
        let declared = declared_modules(&strip(
            "mod drafts;
mod locks;
#[cfg(any())]
mod later;
mod inline {
}
use x::mod_y;",
        ));
        assert_eq!(declared, ["drafts", "locks"]);
    }

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
        let text: String = BROWSER
            .iter()
            .map(|name| format!("test(\"{name}\", () => {{}});"))
            .collect();
        assert!(browser(&[("locks.spec.ts".to_owned(), text)]).is_empty());
    }

    #[test]
    fn bad_example_a_missing_suite() {
        assert_eq!(evaluate(None).len(), 1);
    }

    #[test]
    fn the_suite_of_this_repository_has_every_named_test() {
        let repo = crate::repo::Repo::discover().unwrap();
        let (text, unused) = suite_text(&repo).unwrap();
        let found: Vec<_> = evaluate(Some(&text))
            .into_iter()
            .chain(unused)
            .map(|finding| finding.message)
            .collect();
        assert!(found.is_empty(), "{found:?}");
    }
}
