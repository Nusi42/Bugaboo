//! TEST-UPLOADS: the tests of uploads, downloads and previews exist (ATT-01 … ATT-04, ATT-07,
//! ADR-0032).
//!
//! A file is the most dangerous thing a person can give the server, so the guarantees are held
//! by tests that cannot be deleted unseen: a file is stored by its content and deleted when
//! nothing points to it, an interrupted upload goes on, the type comes from the content, a file over
//! the limit or of a type the organization never accepts is refused before it is stored, SVG and
//! HTML are only downloaded (`attachment`, `nosniff`), a decompression bomb, a broken PDF and a huge
//! picture get no preview and the server goes on, and a link stops working when access ends and is
//! for one person and five minutes. The server part runs on both databases
//! (`crates/bb-server/src/tests/sign_in/files_http.rs`), the child process of the previews is tried
//! as the real program (`crates/bb-server/tests/preview.rs`), and the limits of the previews are
//! tried where they are written (`crates/bb-files`). This check fails when a named test is missing
//! or switched off, in the ways `TEST-EDIT-LOCKS` knows (an `#[ignore]`, a `cfg`, a file that no
//! `mod.rs` declares). The suites themselves run in `cargo xtask ci`.

use super::test_edit_locks::{declared_modules, is_a_test_as};
use super::{Check, Context, Finding};
use crate::rust_source::strip;
use crate::source::read_text;

/// A file of tests and what it must prove.
struct Suite {
    /// The file.
    path: &'static str,
    /// The `mod.rs` that must declare the file, if it is a module of a bigger suite.
    declared_in: Option<(&'static str, &'static str)>,
    /// How a test in the file starts: its attribute and the start of its signature.
    kind: (&'static str, &'static str),
    /// The tests, by name.
    required: &'static [&'static str],
}

const ASYNC_TEST: (&str, &str) = ("#[tokio::test]", "async fn");
const PLAIN_TEST: (&str, &str) = ("#[test]", "fn");

/// Where the tests are and what each file must prove (the work package names them).
const SUITES: [Suite; 3] = [
    Suite {
        path: "crates/bb-server/src/tests/sign_in/files_http.rs",
        declared_in: Some(("crates/bb-server/src/tests/sign_in/mod.rs", "files_http")),
        kind: ASYNC_TEST,
        required: &[
            "att_02_a_file_is_stored_once_by_its_content_and_deleted_when_nothing_points_to_it",
            "att_01_an_interrupted_upload_goes_on_from_the_last_part",
            "att_03_the_type_comes_from_the_content_not_the_name",
            "att_03_a_file_over_the_limit_is_refused_before_it_is_stored",
            "att_03_a_blocked_type_is_refused",
            "att_02_svg_and_html_are_only_downloaded_with_attachment_and_nosniff",
            "att_04_a_decompression_bomb_gets_no_preview_and_the_server_goes_on",
            "att_04_a_broken_pdf_gets_no_preview",
            "att_07_a_link_stops_working_after_access_ends",
            "att_07_a_link_is_for_one_person_and_five_minutes",
            "sec_06_a_guest_adds_files_only_on_issues_they_reported_and_a_viewer_adds_none",
        ],
    },
    Suite {
        path: "crates/bb-server/tests/preview.rs",
        declared_in: None,
        kind: ASYNC_TEST,
        required: &[
            "att_04_a_decompression_bomb_gets_no_preview_and_the_server_goes_on",
            "att_04_a_broken_pdf_gets_no_preview",
            "att_04_a_child_that_ends_badly_or_takes_too_long_costs_the_preview_only",
        ],
    },
    Suite {
        path: "crates/bb-files/src/preview/tests.rs",
        declared_in: Some(("crates/bb-files/src/preview/mod.rs", "tests")),
        kind: PLAIN_TEST,
        required: &["att_04_a_huge_image_is_refused_by_the_pixel_limit"],
    },
];

pub(crate) struct TestUploads;

impl Check for TestUploads {
    fn id(&self) -> &'static str {
        "TEST-UPLOADS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let mut findings = Vec::new();
        for suite in &SUITES {
            let file = read_text(context.repo, suite.path).ok();
            let module = suite.declared_in.map(|(parent, name)| {
                (
                    name,
                    read_text(context.repo, parent)
                        .ok()
                        .map(|text| declared_modules(&strip(&text))),
                )
            });
            findings.extend(evaluate(suite, file.as_deref(), module));
        }
        Ok(findings)
    }
}

/// What is wrong with one suite: the file is missing, nothing declares it, or a test is gone.
fn evaluate(
    suite: &Suite,
    file: Option<&str>,
    module: Option<(&str, Option<Vec<String>>)>,
) -> Vec<Finding> {
    let Some(file) = file else {
        return vec![Finding::at(
            suite.path,
            "The file with the tests of uploads is missing (ADR-0032).",
        )];
    };
    let mut findings = Vec::new();
    if let Some((name, declared)) = module
        && !declared.is_some_and(|modules| modules.iter().any(|known| known == name))
    {
        findings.push(Finding::at(
            suite.path,
            format!("No module declares `{name}` (or only behind a `#[cfg(…)]`), so its tests never run."),
        ));
    }
    let code = strip(file);
    let (attribute, signature) = suite.kind;
    findings.extend(
        suite
            .required
            .iter()
            .filter(|name| !is_a_test_as(&code, name, signature, attribute))
            .map(|name| {
                Finding::at(
                    suite.path,
                    format!("The test `{name}` is missing, or is no `{attribute}` any more."),
                )
            }),
    );
    findings
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::*;

    fn text_with(suite: &Suite, names: &[&str]) -> String {
        let (attribute, signature) = suite.kind;
        let mut text = String::new();
        for name in names {
            let _ = writeln!(text, "{attribute}\n{signature} {name}() {{}}");
        }
        text
    }

    fn declared(suite: &Suite) -> Option<(&'static str, Option<Vec<String>>)> {
        suite
            .declared_in
            .map(|(_, name)| (name, Some(vec![name.to_owned()])))
    }

    #[test]
    fn good_example_every_suite_with_every_named_test() {
        for suite in &SUITES {
            let text = text_with(suite, suite.required);
            assert!(evaluate(suite, Some(&text), declared(suite)).is_empty());
        }
    }

    #[test]
    fn bad_example_a_suite_that_lost_a_test_or_a_file() {
        for suite in &SUITES {
            let text = text_with(suite, &suite.required[1..]);
            let found = evaluate(suite, Some(&text), declared(suite));
            assert_eq!(found.len(), 1, "{}", suite.path);
            assert!(found[0].message.contains(suite.required[0]));
            assert_eq!(evaluate(suite, None, declared(suite)).len(), 1);
        }
    }

    #[test]
    fn bad_example_a_test_that_is_switched_off_or_a_module_nobody_declares() {
        let suite = &SUITES[0];
        let name = suite.required[0];
        let mut text = text_with(suite, &suite.required[1..]);
        let _ = writeln!(text, "#[tokio::test]\n#[ignore]\nasync fn {name}() {{}}");
        assert_eq!(evaluate(suite, Some(&text), declared(suite)).len(), 1);
        let whole = text_with(suite, suite.required);
        let nobody = suite
            .declared_in
            .map(|(_, module)| (module, Some(Vec::new())));
        assert_eq!(evaluate(suite, Some(&whole), nobody).len(), 1);
        let behind_cfg = suite.declared_in.map(|(_, module)| (module, None));
        assert_eq!(evaluate(suite, Some(&whole), behind_cfg).len(), 1);
    }
}
