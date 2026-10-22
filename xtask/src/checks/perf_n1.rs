//! PERF-N1: the tests that count statements exist (PERF-05, no N+1 queries).
//!
//! A list endpoint must run as many statements for forty rows as for five, and the database counts
//! them itself. The tests of the issue page, the list of issues, tag suggestions, the types and
//! groups of a project and the notifications are named below; one that is renamed, deleted or
//! switched off without the list changing fails here (the one implementation is `required_tests`).
//! The suites run on both databases in `cargo xtask ci`.

use super::required_tests::{Required, crate_sources, evaluate};
use super::{Check, Context, Finding};

const QUERIES: &str = "crates/bb-server/src/tests/sign_in/perf_queries.rs";
const NOTIFICATIONS: &str = "crates/bb-server/src/tests/sign_in/perf_notifications.rs";

/// The reads that must not grow with the number of rows.
pub(super) const REQUIRED: [Required; 10] = [
    // The history names a type and a label group from one read, not one read for each row.
    Required::anywhere("sec_03_the_history_names_a_type_and_a_label_only_where_the_reader_may_know_them"),
    Required::anywhere("sec_06_the_history_names_a_tag_only_where_the_reader_may_know_it"),
    Required::anywhere(
        "sec_lm1_what_one_e_mail_of_a_round_may_name_never_reaches_the_next_one",
    ),
    Required::both(
        "perf_05_the_reads_of_an_issue_page_run_as_many_statements_for_a_busy_issue_as_for_a_quiet_one",
    )
    .within(QUERIES),
    Required::both("perf_05_a_page_of_forty_issues_runs_as_many_statements_as_a_page_of_five")
        .within(QUERIES),
    Required::both(
        "perf_05_tag_suggestions_run_as_many_statements_for_twelve_projects_as_for_two",
    )
    .within(QUERIES),
    Required::both(
        "sec_06_a_hidden_comment_or_relation_and_a_missing_one_run_the_same_statements",
    )
    .within(QUERIES),
    Required::both(
        "perf_05_types_and_label_groups_of_a_project_run_as_many_statements_for_forty_as_for_five",
    )
    .within(QUERIES),
    Required::both(
        "perf_05_a_page_of_forty_notifications_runs_as_many_statements_as_a_page_of_five",
    )
    .within(NOTIFICATIONS),
    Required::both(
        "perf_05_the_number_on_the_menu_runs_as_many_statements_for_forty_unread_as_for_five",
    )
    .within(NOTIFICATIONS),
];

pub(crate) struct PerfN1;

impl Check for PerfN1 {
    fn id(&self) -> &'static str {
        "PERF-N1"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        Ok(evaluate(
            &crate_sources(context.repo)?,
            &REQUIRED,
            "A list that runs a statement for every row (PERF-05) is caught by it;",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::super::required_tests::examples;
    use super::*;

    #[test]
    fn good_example_this_repository_has_every_counting_test() {
        examples::this_repository_has_every_test(&REQUIRED);
    }

    #[test]
    fn bad_example_a_missing_counting_test() {
        examples::without_the_sources_every_test_is_missing(&REQUIRED);
    }
}
