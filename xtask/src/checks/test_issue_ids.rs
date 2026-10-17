//! TEST-ISSUE-IDS: the tests of counting and random issue IDs exist, and the ones that touch the
//! database run on both databases (ADR-0021, ISS-01a).
//!
//! The tests are named after the feature: `iss_01a_…`. Each one that is listed below has to be a
//! test function in the product crates, and a test that needs a database has to be run through
//! `each_database`, which runs it on SQLite and on PostgreSQL (ARCH-DB-PARITY holds the record of
//! what ran). A test that is renamed or deleted without the list changing fails here, so the
//! rules about issue IDs cannot lose their tests without anybody noticing.

use super::required_tests::{Required, crate_sources, evaluate};
use super::{Check, Context, Finding};

/// The tests of the rules about issue IDs (docs/milestones/M3/WP-03-issues-core.md).
const REQUIRED: [Required; 5] = [
    Required {
        name: "iss_01a_random_ids_use_the_readable_alphabet_and_never_repeat",
        both_databases: false,
    },
    Required {
        name: "iss_01a_random_ids_made_in_a_project_never_repeat",
        both_databases: true,
    },
    Required {
        name: "iss_01a_counting_ids_count_per_project_without_gaps_under_ten_writers",
        both_databases: true,
    },
    Required {
        name: "iss_01a_an_old_id_redirects_after_the_mode_changes",
        both_databases: true,
    },
    Required {
        name: "iss_01a_an_old_id_keeps_working_after_the_issue_moves",
        both_databases: true,
    },
];

pub(crate) struct TestIssueIds;

impl Check for TestIssueIds {
    fn id(&self) -> &'static str {
        "TEST-ISSUE-IDS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        Ok(evaluate(
            &crate_sources(context)?,
            &REQUIRED,
            "The rules about issue IDs (ADR-0021) are held by it;",
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
        examples::a_name_in_a_comment_is_no_test(&REQUIRED, "iss_01a_an_old_id_redirects");
    }
}
