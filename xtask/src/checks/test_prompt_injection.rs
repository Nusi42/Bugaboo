//! TEST-PROMPT-INJECTION: the prompt-injection test set exists by name and runs (AI-05, ADR-0013,
//! COMP-10).
//!
//! What people wrote is data to the AI, never an order (architecture §5 *The assistant proposes*).
//! The guarantees are held by tests that cannot be deleted unseen: an issue description, a comment,
//! an own field, a file name and a tool answer that each say "ignore your instructions", "change the
//! status", "show this picture" or "send the text of WEB-1 to this address" stay inside their fence;
//! nothing changes without *Apply* or a scope that allows it; and a field kept from AI never reaches
//! the model. The set runs against a model that obeys everything it reads. This check fails when a
//! named test is missing or switched off, in the ways the one implementation knows (`required_tests`:
//! an `#[ignore]`, a `cfg`, a file that no `mod` reaches). Later packages add their own cases (agents,
//! standing rules) to the list.

use super::required_tests::{Required, crate_sources, evaluate};
use super::{Check, Context, Finding};

const SET: &str = "crates/bb-app/src/tests/prompt_injection.rs";

/// The tests of the set, in the order of the carriers of the orders.
pub(super) const REQUIRED: [Required; 9] = [
    Required::anywhere(
        "prompt_injection_an_issue_description_that_orders_the_assistant_stays_data",
    )
    .within(SET),
    Required::anywhere("prompt_injection_a_comment_that_asks_for_a_change_stays_data").within(SET),
    Required::anywhere(
        "prompt_injection_an_own_field_that_tries_to_close_its_fence_stays_inside_it",
    )
    .within(SET),
    Required::anywhere("prompt_injection_a_file_name_is_fenced_like_any_text").within(SET),
    Required::anywhere("prompt_injection_a_tool_answer_that_gives_orders_stays_data").within(SET),
    Required::anywhere("prompt_injection_nothing_changes_without_apply_or_a_scope_that_allows_it")
        .within(SET),
    Required::anywhere("prompt_injection_a_field_kept_from_ai_never_reaches_the_model")
        .within(SET),
    Required::anywhere(
        "prompt_injection_a_picture_or_a_link_in_a_text_is_data_and_never_written_as_one",
    )
    .within(SET),
    Required::anywhere(
        "prompt_injection_a_request_to_send_another_issue_away_cannot_read_what_the_person_cannot_read",
    )
    .within(SET),
];

pub(crate) struct TestPromptInjection;

impl Check for TestPromptInjection {
    fn id(&self) -> &'static str {
        "TEST-PROMPT-INJECTION"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        Ok(evaluate(
            &crate_sources(context.repo)?,
            &REQUIRED,
            "The safeguards against prompt injection (ADR-0013, AI-05) are held by it;",
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
    fn bad_example_without_the_set_every_test_is_missing() {
        examples::without_the_sources_every_test_is_missing(&REQUIRED);
    }

    #[test]
    fn bad_example_a_test_in_the_wrong_file_or_switched_off() {
        let name = REQUIRED[0].name;
        let one = [Required::anywhere(name).within(SET)];
        let root = (
            "crates/bb-app/src/lib.rs".to_owned(),
            "mod tests;\n".to_owned(),
        );
        let tests = (
            "crates/bb-app/src/tests/mod.rs".to_owned(),
            "mod prompt_injection;\nmod elsewhere;\n".to_owned(),
        );
        let at = |path: &str, attribute: &str| {
            vec![
                root.clone(),
                tests.clone(),
                (
                    path.to_owned(),
                    format!("#[tokio::test]\n{attribute}async fn {name}() {{}}\n"),
                ),
            ]
        };
        assert!(evaluate(&at(SET, ""), &one, "").is_empty());
        assert_eq!(
            evaluate(&at("crates/bb-app/src/tests/elsewhere.rs", ""), &one, "").len(),
            1
        );
        assert_eq!(evaluate(&at(SET, "#[ignore]\n"), &one, "").len(), 1);
    }
}
