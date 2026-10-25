//! TEST-SEARCH-ROUNDTRIP: Simple search and Advanced search never lose information, because both
//! are one tree that prints to one canonical text (QRY-02, ADR-0005).
//!
//! The tests are in `crates/bb-searchlang/tests/`: the property tests (proptest) that print every
//! generated tree and parse it back to itself, that no text makes the parser panic and that the
//! canonical text of an accepted text is stable, and the tests that the example of QRY-01, every
//! helper of QRY-04, the limits and the equal answer for a hidden and an unknown field (QRY-05)
//! hold. This check fails when a test that the work package names is missing or switched off, in
//! the ways of the one implementation (`required_tests`), so the guarantee cannot be deleted
//! unseen. The suite itself runs in `cargo xtask ci` (the Rust tests); the browser runs the same
//! texts through the module (`web/tooling/search-language.test.ts`).

use super::required_tests::{Required, crate_sources, evaluate};
use super::{Check, Context, Finding};

/// Where the tests are.
const SUITE: &str = "crates/bb-searchlang/tests";

/// What the suite must prove, by the name of the test that proves it (M5 WP-02 names them).
pub(super) const REQUIRED: [Required; 9] = [
    Required::anywhere("search_roundtrip_parse_of_print_is_the_tree").within(SUITE),
    Required::anywhere("search_roundtrip_a_parse_never_panics_on_any_text").within(SUITE),
    Required::anywhere("search_roundtrip_the_canonical_text_of_an_accepted_text_is_stable")
        .within(SUITE),
    Required::anywhere("search_roundtrip_the_json_of_a_tree_reads_back_as_the_tree").within(SUITE),
    Required::anywhere("qry_01_the_example_parses_checks_and_prints_back").within(SUITE),
    Required::anywhere("qry_04_every_helper_parses_checks_and_prints_back").within(SUITE),
    Required::anywhere("qry_05_a_hidden_field_and_an_unknown_field_give_the_same_mistake")
        .within(SUITE),
    Required::anywhere("the_limits_fifty_filters_pass_and_fifty_one_fail").within(SUITE),
    Required::anywhere("the_limits_five_steps_pass_and_six_fail").within(SUITE),
];

pub(crate) struct TestSearchRoundtrip;

impl Check for TestSearchRoundtrip {
    fn id(&self) -> &'static str {
        "TEST-SEARCH-ROUNDTRIP"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        Ok(evaluate(
            &crate_sources(context.repo)?,
            &REQUIRED,
            "The search language (ADR-0005) is held by it;",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::super::required_tests::examples;
    use super::*;

    #[test]
    fn good_example_every_test_exists() {
        examples::every_test_is_there(&REQUIRED);
    }

    #[test]
    fn bad_example_a_missing_test_is_found() {
        examples::a_missing_or_single_test_is_found(&REQUIRED);
    }

    #[test]
    fn a_name_in_a_comment_is_not_a_test() {
        examples::a_name_in_a_comment_is_no_test(
            &REQUIRED,
            "search_roundtrip_a_parse_never_panics",
        );
    }
}
