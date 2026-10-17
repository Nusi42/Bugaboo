//! TEST-WORKFLOWS: the tests of the rules of a status change exist, and the ones that touch the
//! database run on both databases (ADR-0031, WF-01 to WF-06).
//!
//! The tests are named after the feature: `wf_01_…` to `wf_06_…`. Each one that is listed below has
//! to be a test function in the product crates, and a test that needs a database has to be run
//! through `each_database`, which runs it on SQLite and on PostgreSQL (ARCH-DB-PARITY holds the
//! record of what ran). A test that is renamed or deleted without the list changing fails here, so
//! the judge of a status change, the choice of the workflow of a place and the publishing of a
//! workflow cannot lose their tests without anybody noticing.

use super::{Check, Context, Finding};
use crate::docs::text::has_extension;
use crate::rust_source::strip_comments;
use crate::source::{Source, read_text};

/// A test that has to exist, and whether it has to run on both databases.
struct Required {
    name: &'static str,
    both_databases: bool,
}

/// The tests of the rules of a status change (docs/milestones/M4/WP-04-workflows-on-the-server.md).
const REQUIRED: [Required; 16] = [
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
];

/// What runs a test on both databases.
const BOTH_DATABASES: &str = "each_database(";

pub(crate) struct TestWorkflows;

impl Check for TestWorkflows {
    fn id(&self) -> &'static str {
        "TEST-WORKFLOWS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        // Whatever changed, every Rust file of the crates is read, the folders of tests too: the
        // tests are there.
        let mut sources = Vec::new();
        for file in context.repo.all_files()? {
            if file.starts_with("crates/") && has_extension(&file, "rs") {
                let text = read_text(context.repo, &file)?;
                sources.push((file, text));
            }
        }
        Ok(evaluate(&sources))
    }
}

/// The body of the function `name`: from its `fn` up to the next function at the same level.
fn body_of<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let start = text.find(&format!("fn {name}("))?;
    let rest = &text[start..];
    let end = rest[1..]
        .find("\n#[")
        .or_else(|| rest[1..].find("\nfn "))
        .map_or(rest.len(), |at| at + 1);
    Some(&rest[..end])
}

fn evaluate(sources: &[Source]) -> Vec<Finding> {
    let code: Vec<(&str, String)> = sources
        .iter()
        .map(|(path, text)| (path.as_str(), strip_comments(text)))
        .collect();
    let mut findings = Vec::new();
    for required in REQUIRED {
        let found = code
            .iter()
            .find_map(|(path, text)| body_of(text, required.name).map(|body| (*path, body)));
        match found {
            None => findings.push(Finding::general(format!(
                "The test `{}` is missing. The rules of a status change (ADR-0031) are held by it; \
                 write it again or, if the rule changed, change this list.",
                required.name
            ))),
            Some((path, body)) if required.both_databases && !body.contains(BOTH_DATABASES) => {
                findings.push(Finding::at(
                    path,
                    format!(
                        "The test `{}` has to run on both databases: wrap it in `each_database` \
                         (ARCH-DB-PARITY).",
                        required.name
                    ),
                ));
            }
            Some(_) => {}
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::*;

    fn all_tests(wrapper: &str) -> Vec<Source> {
        let mut text = String::new();
        for required in REQUIRED {
            let _ = write!(
                text,
                "#[tokio::test]\nasync fn {}() {{\n    {wrapper}\n}}\n\n",
                required.name
            );
        }
        vec![("crates/x/src/tests.rs".to_owned(), text)]
    }

    #[test]
    fn good_example_every_test_exists_and_runs_on_both_databases() {
        assert!(evaluate(&all_tests("each_database(\"x\", body).await")).is_empty());
    }

    #[test]
    fn bad_example_a_missing_test_or_one_that_runs_on_one_database() {
        let missing = evaluate(&[("crates/x/src/tests.rs".to_owned(), String::new())]);
        assert_eq!(missing.len(), REQUIRED.len());
        let single = evaluate(&all_tests("body().await"));
        assert_eq!(
            single.len(),
            REQUIRED.iter().filter(|r| r.both_databases).count()
        );
    }

    #[test]
    fn a_name_in_a_comment_is_not_a_test() {
        let mut sources = all_tests("each_database(\"x\", body).await");
        for (_, text) in &mut sources {
            *text = text.replace(
                "async fn wf_01_an_m3_database_keeps",
                "// async fn wf_01_an_m3_database_keeps",
            );
        }
        assert_eq!(evaluate(&sources).len(), 1);
    }
}
