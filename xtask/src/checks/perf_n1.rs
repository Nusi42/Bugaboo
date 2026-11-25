//! PERF-N1: the tests that count statements exist (PERF-05, no N+1 queries).
//!
//! A list endpoint must run as many statements for forty rows as for five, and the database counts
//! them itself. The tests of the issue page, the list of issues, tag suggestions, the types and
//! groups of a project, the notifications and the saved searches are named below; one that is renamed, deleted or
//! switched off without the list changing fails here (the one implementation is `required_tests`).
//! The suites run on both databases in `cargo xtask ci`.

use super::required_tests::{Required, crate_sources, evaluate};
use super::{Check, Context, Finding};

const QUERIES: &str = "crates/bb-server/src/tests/sign_in/perf_queries.rs";
const NOTIFICATIONS: &str = "crates/bb-server/src/tests/sign_in/perf_notifications.rs";
const LIST_LABELS: &str = "crates/bb-server/src/tests/sign_in/list_labels_http.rs";
const SAVED_SEARCHES: &str = "crates/bb-server/src/tests/sign_in/saved_searches_http.rs";
const TIME_ENTRIES: &str = "crates/bb-server/src/tests/sign_in/time_perf_http.rs";
const TOTALS: &str = "crates/bb-server/src/tests/sign_in/search_totals_more_http.rs";
const WORK_TIMES: &str = "crates/bb-server/src/tests/sign_in/work_times_perf.rs";
const DASHBOARDS: &str = "crates/bb-server/src/tests/sign_in/dashboards_boxes_http.rs";

/// The reads that must not grow with the number of rows.
pub(super) const REQUIRED: [Required; 20] = [
    // A stretch of work days reads its days, periods, breaks and changed days once each, however long.
    Required::both("perf_05_a_stretch_of_forty_work_days_runs_as_many_statements_as_one_of_five")
        .within("crates/bb-server/src/tests/sign_in/working_hours_perf.rs"),
    // The totals of a search by people, named or merged by how they are shown, read the people,
    // their roles and their names once, however many people the groups hold; the groups of a list
    // by people are one statement of the search, like its count.
    Required::both("perf_05_totals_grouped_by_people_run_as_many_statements_for_forty_as_for_five")
        .within(TOTALS),
    Required::both("perf_05_the_groups_of_a_list_by_assigned_to_are_one_statement_like_the_count")
        .within(TOTALS),
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
    Required::both(
        "perf_05_a_page_of_forty_issues_with_labels_runs_as_many_statements_as_a_page_of_five",
    )
    .within(LIST_LABELS),
    Required::both(
        "perf_05_a_page_of_forty_saved_searches_runs_as_many_statements_as_a_page_of_five",
    )
    .within(SAVED_SEARCHES),
    Required::both(
        "perf_05_a_page_of_forty_time_entries_runs_as_many_statements_as_a_page_of_five",
    )
    .within(TIME_ENTRIES),
    // The people of a project with their shares and the other projects of each read together.
    Required::both("perf_05_forty_people_with_their_shares_run_as_many_statements_as_five")
        .within(WORK_TIMES),
    // The projects two people share are found from one tree, however many there are.
    Required::both(
        "perf_05_a_read_of_one_persons_work_times_runs_as_many_statements_with_twenty_projects_as_with_two",
    )
    .within(WORK_TIMES),
    // A dashboard judges each box with a fixed number of reads; its list reads makers and shares
    // once for the page.
    Required::both("perf_05_a_dashboard_runs_a_fixed_number_of_statements_for_each_box")
        .within(DASHBOARDS),
    Required::both(
        "perf_05_a_page_of_forty_dashboards_runs_as_many_statements_as_a_page_of_five",
    )
    .within(DASHBOARDS),
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
