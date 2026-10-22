//! TEST-WORKFLOWS: the tests of the rules of a status change exist, and the ones that touch the
//! database run on both databases (ADR-0031, WF-01 to WF-06).
//!
//! The tests are named after the feature: `wf_01_…` to `wf_06_…`. Each one that is listed below has
//! to be a test function in the product crates, and a test that needs a database has to be run
//! through `each_database`, which runs it on SQLite and on PostgreSQL (ARCH-DB-PARITY holds the
//! record of what ran). A test that is renamed or deleted without the list changing fails here, so
//! the judge of a status change, the choice of the workflow of a place and the publishing of a
//! workflow cannot lose their tests without anybody noticing. The tests of where issue types and
//! groups of labels are offered are held here too: they are found by the same walk (ADR-0027
//! amendment, ISS-02, TAG-02, SEC-03).

use super::required_tests::{Required, crate_sources, evaluate};
use super::{Check, Context, Finding};

/// The tests of the rules of a status change (docs/milestones/M4/WP-04-workflows-on-the-server.md).
pub(super) const REQUIRED: [Required; 59] = [
    Required::both("wf_01_an_m3_database_keeps_every_status_after_the_upgrade"),
    Required::both("wf_01_a_project_finds_its_workflow_like_a_layout"),
    Required::both(
        "wf_01_a_workflow_comes_back_as_stored_and_a_save_needs_the_version_it_started_from",
    ),
    Required::anywhere("wf_02_publishing_without_a_status_moves_its_issues_with_history"),
    Required::both("wf_02_publishing_without_a_status_moves_its_issues_over_http"),
    Required::anywhere("wf_03_who_can_do_it_allows_only_the_named_roles_and_people"),
    Required::anywhere("wf_03_only_if_refuses_with_a_sentence"),
    Required::anywhere("wf_03_a_rule_never_gives_a_right_change_issues_does_not"),
    Required::anywhere("wf_03_whatever_the_rules_are_they_give_no_right_and_name_no_hidden_field"),
    Required::anywhere("wf_04_must_be_filled_in_names_the_missing_field"),
    Required::anywhere("wf_04_ask_for_never_asks_for_a_hidden_field"),
    Required::anywhere("wf_06_a_new_organization_has_the_three_starter_workflows"),
    Required::both("wf_06_a_new_organization_offers_the_three_starter_workflows_over_http"),
    Required::anywhere("iss_09_many_changes_skip_an_issue_whose_workflow_does_not_allow_it"),
    Required::anywhere("sec_06_a_refusal_of_a_status_change_names_no_field_the_person_cannot_see"),
    Required::both("m3_f53_who_may_change_names_the_roles_held_in_the_project_over_http"),
    // Where an issue type or a group of labels is offered is found by the same walk as a workflow
    // (docs/milestones/M4/WP-11-types-and-labels-per-project.md).
    Required::both("iss_02_a_project_offers_only_its_issue_types"),
    Required::both("iss_02_an_upgraded_database_offers_every_type_everywhere"),
    Required::both("iss_02_a_retired_type_is_not_missing_on_a_move"),
    Required::both("sec_03_a_project_lead_manages_the_types_of_their_project_only"),
    Required::both("tag_02_a_label_group_of_a_project_is_offered_there_and_below"),
    Required::both("sec_03_check_access_names_the_level_per_issue_type"),
    Required::anywhere("iss_02_a_sub_project_follows_its_parent_until_it_has_its_own_row"),
    // Who may know a subject, the names, the limits and the upgrade (review of WP-11).
    Required::anywhere("sec_03_a_subject_created_in_a_project_is_unknown_outside_it"),
    Required::anywhere("sec_03_check_access_does_not_name_a_type_the_checker_may_not_know"),
    Required::anywhere("sec_03_a_subject_in_use_on_an_issue_the_reader_reads_is_known"),
    Required::anywhere("sec_03_a_name_is_unique_among_the_subjects_that_can_meet_in_a_project"),
    Required::anywhere("sec_03_the_rows_of_a_subject_leave_out_a_place_the_reader_may_not_know"),
    Required::anywhere("iss_02_each_place_has_its_own_limit_of_issue_types"),
    Required::anywhere("tag_02_each_place_has_its_own_limit_of_groups_and_of_labels"),
    Required::anywhere(
        "sec_03_a_label_of_a_project_is_not_rewritten_in_projects_its_lead_does_not_manage",
    ),
    Required::anywhere("tag_02_a_list_about_a_project_holds_only_the_subjects_that_can_reach_it"),
    Required::both(
        "sec_03_a_subject_created_in_a_project_is_not_there_for_a_guest_a_sibling_lead_or_a_key",
    ),
    Required::both(
        "perf_05_types_and_label_groups_of_a_project_run_as_many_statements_for_forty_as_for_five",
    ),
    Required::both("iss_09_a_stored_run_that_skipped_for_a_missing_type_is_still_read"),
    Required::both("iss_02_a_subject_refers_to_its_project_and_a_group_name_may_repeat"),
    Required::both("sec_06_the_subjects_of_the_issues_a_person_reads_are_found_for_the_offers"),
    Required::both("iss_02_the_rows_of_a_subject_are_found_by_the_subject"),
    Required::anywhere("sec_03_a_visible_sub_project_under_a_hidden_parent_knows_what_reaches_it"),
    Required::anywhere("sec_03_an_issue_of_a_type_the_writer_does_not_know_is_not_found"),
    Required::both("sec_03_no_screen_names_a_type_of_a_project_the_reader_does_not_know"),
    Required::anywhere("sec_06_a_group_in_use_is_unknown_to_a_reader_whose_role_hides_the_labels"),
    Required::anywhere(
        "sec_03_a_lead_who_knows_a_project_they_cannot_see_is_told_the_same_by_the_list_and_the_put",
    ),
    // What a Publish moves, judged with the rights of the publisher (ADR-0031 amendments 11 and 12).
    Required::anywhere(
        "wf_02_a_publish_with_an_issue_the_publisher_does_not_see_publishes_nothing",
    ),
    Required::anywhere(
        "wf_02_a_publish_with_an_issue_whose_status_the_publisher_may_not_change_publishes_nothing",
    ),
    Required::anywhere(
        "wf_02_a_move_ignores_the_rules_of_the_old_change_and_closes_by_the_group_of_the_target",
    ),
    Required::anywhere("wf_02_the_preview_names_what_the_publisher_sees_and_only_counts_the_rest"),
    Required::anywhere("sec_03_a_project_lead_manages_the_label_groups_of_their_project_only"),
    Required::anywhere("sec_l6_who_may_change_does_not_name_a_type_the_reader_may_not_know"),
    Required::anywhere("sec_l7_try_it_needs_check_access_as_well_as_manage_workflows"),
    // The same moves and the same preview on both databases (the tests above run on the fakes).
    Required::both(
        "wf_02_a_publish_with_an_issue_the_publisher_does_not_see_publishes_nothing_over_http",
    ),
    Required::both(
        "wf_02_a_publish_with_an_issue_whose_status_the_publisher_may_not_change_publishes_nothing_over_http",
    ),
    Required::both(
        "wf_02_a_move_ignores_the_rules_of_the_old_change_and_closes_by_the_group_of_the_target_over_http",
    ),
    Required::both(
        "wf_02_the_preview_names_what_the_publisher_sees_and_only_counts_the_rest_over_http",
    ),
    // Names stored before one rule told them apart stay as they are (bb_core::names).
    Required::anywhere(
        "wf_02_two_stored_names_that_differ_only_in_spaces_can_be_saved_again_but_not_made",
    ),
    Required::anywhere(
        "iss_02_two_stored_issue_types_that_differ_only_in_spaces_can_be_retired_and_not_renamed",
    ),
    Required::anywhere(
        "iss_04_two_stored_kinds_of_relation_that_differ_only_in_spaces_can_be_retired_and_not_renamed",
    ),
    Required::anywhere(
        "cf_02_two_stored_fields_and_choices_that_differ_only_in_spaces_can_be_retired_and_not_renamed",
    ),
    // One rule for "the same name" (bb_core::names).
    Required::anywhere("iss_02_a_type_and_a_group_of_labels_are_the_same_name_by_one_rule"),
];

pub(crate) struct TestWorkflows;

impl Check for TestWorkflows {
    fn id(&self) -> &'static str {
        "TEST-WORKFLOWS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        Ok(evaluate(
            &crate_sources(context.repo)?,
            &REQUIRED,
            "The rules of a status change (ADR-0031) are held by it;",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::super::required_tests::examples;
    use super::*;

    #[test]
    fn good_example_every_test_exists_and_runs_on_both_databases() {
        examples::every_test_is_there(&REQUIRED);
    }

    #[test]
    fn bad_example_a_missing_test_or_one_that_runs_on_one_database() {
        examples::a_missing_or_single_test_is_found(&REQUIRED);
    }

    #[test]
    fn a_name_in_a_comment_is_not_a_test() {
        examples::a_name_in_a_comment_is_no_test(&REQUIRED, "wf_01_an_m3_database_keeps");
    }
}
