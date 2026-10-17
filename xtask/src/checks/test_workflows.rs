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
const REQUIRED: [Required; 23] = [
    Required {
        name: "wf_01_an_m3_database_keeps_every_status_after_the_upgrade",
        both_databases: true,
    },
    Required {
        name: "wf_01_a_project_finds_its_workflow_like_a_layout",
        both_databases: true,
    },
    Required {
        name: "wf_01_a_workflow_comes_back_as_stored_and_a_save_needs_the_version_it_started_from",
        both_databases: true,
    },
    Required {
        name: "wf_02_publishing_without_a_status_moves_its_issues_with_history",
        both_databases: false,
    },
    Required {
        name: "wf_02_publishing_without_a_status_moves_its_issues_over_http",
        both_databases: true,
    },
    Required {
        name: "wf_03_who_can_do_it_allows_only_the_named_roles_and_people",
        both_databases: false,
    },
    Required {
        name: "wf_03_only_if_refuses_with_a_sentence",
        both_databases: false,
    },
    Required {
        name: "wf_03_a_rule_never_gives_a_right_change_issues_does_not",
        both_databases: false,
    },
    Required {
        name: "wf_03_whatever_the_rules_are_they_give_no_right_and_name_no_hidden_field",
        both_databases: false,
    },
    Required {
        name: "wf_04_must_be_filled_in_names_the_missing_field",
        both_databases: false,
    },
    Required {
        name: "wf_04_ask_for_never_asks_for_a_hidden_field",
        both_databases: false,
    },
    Required {
        name: "wf_06_a_new_organization_has_the_three_starter_workflows",
        both_databases: false,
    },
    Required {
        name: "wf_06_a_new_organization_offers_the_three_starter_workflows_over_http",
        both_databases: true,
    },
    Required {
        name: "iss_09_many_changes_skip_an_issue_whose_workflow_does_not_allow_it",
        both_databases: false,
    },
    Required {
        name: "sec_06_a_refusal_of_a_status_change_names_no_field_the_person_cannot_see",
        both_databases: false,
    },
    Required {
        name: "m3_f53_who_may_change_names_the_roles_held_in_the_project_over_http",
        both_databases: true,
    },
    // Where an issue type or a group of labels is offered is found by the same walk as a workflow
    // (docs/milestones/M4/WP-11-types-and-labels-per-project.md).
    Required {
        name: "iss_02_a_project_offers_only_its_issue_types",
        both_databases: true,
    },
    Required {
        name: "iss_02_an_upgraded_database_offers_every_type_everywhere",
        both_databases: true,
    },
    Required {
        name: "iss_02_a_retired_type_is_not_missing_on_a_move",
        both_databases: true,
    },
    Required {
        name: "sec_03_a_project_lead_manages_the_types_of_their_project_only",
        both_databases: true,
    },
    Required {
        name: "tag_02_a_label_group_of_a_project_is_offered_there_and_below",
        both_databases: true,
    },
    Required {
        name: "sec_03_check_access_names_the_level_per_issue_type",
        both_databases: true,
    },
    Required {
        name: "iss_02_a_sub_project_follows_its_parent_until_it_has_its_own_row",
        both_databases: false,
    },
];

pub(crate) struct TestWorkflows;

impl Check for TestWorkflows {
    fn id(&self) -> &'static str {
        "TEST-WORKFLOWS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        Ok(evaluate(
            &crate_sources(context)?,
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
