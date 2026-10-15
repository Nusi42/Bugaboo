//! TEST-ISSUE-IDS: the tests of counting and random issue IDs exist, and the ones that touch the
//! database run on both databases (ADR-0021, ISS-01a).
//!
//! The tests are named after the feature: `iss_01a_…`. Each one that is listed below has to be a
//! test function in the product crates, and a test that needs a database has to be run through
//! `each_database`, which runs it on SQLite and on PostgreSQL (ARCH-DB-PARITY holds the record of
//! what ran). A test that is renamed or deleted without the list changing fails here, so the
//! rules about issue IDs cannot lose their tests without anybody noticing.

use super::{Check, Context, Finding};
use crate::docs::text::has_extension;
use crate::rust_source::strip_comments;
use crate::source::{Source, read_text};

/// A test that has to exist, and whether it has to run on both databases.
struct Required {
    name: &'static str,
    both_databases: bool,
}

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

/// What runs a test on both databases.
const BOTH_DATABASES: &str = "each_database(";

pub(crate) struct TestIssueIds;

impl Check for TestIssueIds {
    fn id(&self) -> &'static str {
        "TEST-ISSUE-IDS"
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
                "The test `{}` is missing. The rules about issue IDs (ADR-0021) are held by it; \
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
            *text = text.replace("async fn iss_01a_an_old", "// async fn iss_01a_an_old");
        }
        assert_eq!(evaluate(&sources).len(), 1);
    }
}
