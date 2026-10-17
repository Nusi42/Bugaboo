//! UX-07: the message that a word makes (`words!` with `messages`) has to be in the catalog, or
//! the program does not compile. Nothing lists the enums that make messages: a new one is held to
//! the catalog the moment it is written.

// Dependencies of the library that this test does not use itself (`unused_crate_dependencies`).
use bb_core as _;
use bb_macros as _;
use jiff as _;
use proptest as _;
use serde as _;
use serde_json as _;
use unicode_properties as _;
#[cfg(feature = "openapi")]
use utoipa as _;
use uuid as _;

#[test]
fn ux_07_a_message_made_from_a_word_that_is_not_in_the_catalog_does_not_compile() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/words_message_present.rs");
    cases.compile_fail("tests/ui/words_message_missing.rs");
}
