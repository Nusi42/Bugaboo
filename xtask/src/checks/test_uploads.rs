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
//! or switched off, in the ways the one implementation knows (`required_tests`: an `#[ignore]`, a
//! `cfg`, a file that no `mod` reaches). The security chapter cites these tests as proof, and
//! DOC-PROVEN-BY holds that every one it cites is here. The suites themselves run in
//! `cargo xtask ci`.

use super::required_tests::{Required, crate_sources, evaluate};
use super::{Check, Context, Finding};

const SERVER: &str = "crates/bb-server/src/tests/sign_in/files_http.rs";
const CHILD: &str = "crates/bb-server/tests/preview.rs";
const APP: &str = "crates/bb-app/src/tests/files.rs";

/// What the work packages and the security chapter name as proof, and where each test lives. The
/// server part runs on both databases.
pub(super) const REQUIRED: [Required; 26] = [
    Required::both(
        "att_02_a_file_is_stored_once_by_its_content_and_deleted_when_nothing_points_to_it",
    )
    .within(SERVER),
    Required::both("att_01_an_interrupted_upload_goes_on_from_the_last_part").within(SERVER),
    Required::both("att_03_the_type_comes_from_the_content_not_the_name").within(SERVER),
    Required::both("att_03_a_file_over_the_limit_is_refused_before_it_is_stored").within(SERVER),
    Required::both("att_03_a_blocked_type_is_refused").within(SERVER),
    Required::both("att_02_svg_and_html_are_only_downloaded_with_attachment_and_nosniff")
        .within(SERVER),
    Required::both("att_04_a_decompression_bomb_gets_no_preview_and_the_server_goes_on")
        .within(SERVER),
    Required::both("att_04_a_broken_pdf_gets_no_preview").within(SERVER),
    Required::both("att_07_a_link_stops_working_after_access_ends").within(SERVER),
    Required::both("att_07_a_link_is_for_one_person_and_five_minutes").within(SERVER),
    Required::both("sec_06_a_guest_adds_files_only_on_issues_they_reported_and_a_viewer_adds_none")
        .within(SERVER),
    Required::both(
        "att_01_an_unfinished_upload_is_deleted_after_a_day_and_only_a_part_has_a_big_body_limit",
    )
    .within(SERVER),
    Required::both("sec_06_the_upload_of_somebody_else_cannot_be_reached").within(SERVER),
    Required::both("sec_06_an_upload_ends_when_the_person_loses_access_in_the_middle_of_it")
        .within(SERVER),
    Required::both("att_04_finishing_a_file_that_somebody_else_stored_answers_like_a_new_one")
        .within(SERVER),
    // The child process, tried as the real program.
    Required::anywhere("att_04_a_decompression_bomb_gets_no_preview_and_the_server_goes_on")
        .within(CHILD),
    Required::anywhere("att_04_a_broken_pdf_gets_no_preview").within(CHILD),
    Required::anywhere("att_04_a_child_that_ends_badly_or_takes_too_long_costs_the_preview_only")
        .within(CHILD),
    // The accepted risk of ADR-0032 (Windows limits the heap only) rests on this test.
    Required::anywhere("att_04_a_pdf_bomb_costs_the_child_its_memory_limit_and_the_preview_only")
        .within(CHILD),
    Required::anywhere("att_04_a_huge_image_is_refused_by_the_pixel_limit")
        .within("crates/bb-files/src/preview/tests.rs"),
    // The limits and the clean-up, where the rules are written.
    Required::anywhere("att_03_open_uploads_are_limited_in_number_and_in_bytes").within(APP),
    Required::anywhere("att_03_files_in_the_trash_count_toward_the_limit_of_an_issue").within(APP),
    Required::anywhere(
        "att_01_a_part_is_refused_when_the_disk_has_no_room_and_the_upload_goes_on_later",
    )
    .within(APP),
    Required::anywhere("att_02_the_clean_up_does_not_take_the_bytes_of_a_file_that_is_being_added")
        .within(APP),
    Required::anywhere("att_04_finishing_a_file_that_is_stored_already_answers_like_a_new_one")
        .within(APP),
    // Adding a file tells the followers of the issue.
    Required::anywhere("ntf_01_adding_a_file_tells_the_followers_and_not_the_one_who_added_it")
        .within("crates/bb-app/src/tests/notifications_files.rs"),
];

pub(crate) struct TestUploads;

impl Check for TestUploads {
    fn id(&self) -> &'static str {
        "TEST-UPLOADS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        Ok(evaluate(
            &crate_sources(context.repo)?,
            &REQUIRED,
            "The guarantees of uploads (ADR-0032) are held by it;",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::super::required_tests::examples;
    use super::*;

    #[test]
    fn good_example_the_tests_of_this_repository_are_all_there() {
        examples::this_repository_has_every_test(&REQUIRED);
    }

    #[test]
    fn bad_example_a_test_that_is_gone_switched_off_or_in_the_wrong_file() {
        let name = REQUIRED[0].name;
        let live = format!(
            "#[tokio::test]
async fn {name}() {{ each_database(\"x\"); }}
"
        );
        let one = [Required::both(name).within(SERVER)];
        let root = (
            "crates/bb-server/src/lib.rs".to_owned(),
            "mod tests;
"
            .to_owned(),
        );
        let at = |path: &str, text: &str| {
            vec![
                root.clone(),
                (
                    "crates/bb-server/src/tests/mod.rs".to_owned(),
                    "mod sign_in;
"
                    .to_owned(),
                ),
                (
                    "crates/bb-server/src/tests/sign_in/mod.rs".to_owned(),
                    "mod files_http;
"
                    .to_owned(),
                ),
                (path.to_owned(), text.to_owned()),
            ]
        };
        assert!(evaluate(&at(SERVER, &live), &one, "").is_empty());
        assert_eq!(
            evaluate(
                &at(
                    SERVER,
                    &live.replace(
                        "#[tokio::test]",
                        "#[tokio::test]
#[ignore]"
                    )
                ),
                &one,
                ""
            )
            .len(),
            1
        );
        assert_eq!(evaluate(&at(SERVER, ""), &one, "").len(), 1);
        assert_eq!(
            evaluate(
                &at("crates/bb-server/src/tests/sign_in/other.rs", &live),
                &one,
                ""
            )
            .len(),
            1
        );
        // A module that nothing declares.
        let mut undeclared = at(SERVER, &live);
        undeclared[2].1 = String::new();
        assert_eq!(evaluate(&undeclared, &one, "").len(), 1);
    }
}
