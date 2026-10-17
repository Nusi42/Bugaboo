//! The part that the checks of "these tests must exist" share (TEST-ISSUE-IDS, TEST-WORKFLOWS): a
//! list of test names, found in the Rust files of the product crates, and for each one whether it
//! has to run on both databases (ARCH-DB-PARITY). The lists and the words of the findings are the
//! checks' own.

use super::{Context, Finding};
use crate::docs::text::has_extension;
use crate::rust_source::strip_comments;
use crate::source::{Source, read_text};

/// A test that has to exist, and whether it has to run on both databases.
pub(super) struct Required {
    pub(super) name: &'static str,
    pub(super) both_databases: bool,
}

/// What runs a test on both databases.
const BOTH_DATABASES: &str = "each_database(";

/// Every Rust file of the crates, the folders of tests too: the tests are there. Whatever changed,
/// all of them are read.
pub(super) fn crate_sources(context: &Context<'_>) -> anyhow::Result<Vec<Source>> {
    let mut sources = Vec::new();
    for file in context.repo.all_files()? {
        if file.starts_with("crates/") && has_extension(&file, "rs") {
            let text = read_text(context.repo, &file)?;
            sources.push((file, text));
        }
    }
    Ok(sources)
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

/// The findings for the tests of `required` that are missing (`why_needed` finishes the sentence
/// "The test … is missing. …") or run on one database only.
pub(super) fn evaluate(
    sources: &[Source],
    required: &[Required],
    why_needed: &str,
) -> Vec<Finding> {
    let code: Vec<(&str, String)> = sources
        .iter()
        .map(|(path, text)| (path.as_str(), strip_comments(text)))
        .collect();
    let mut findings = Vec::new();
    for each in required {
        let found = code
            .iter()
            .find_map(|(path, text)| body_of(text, each.name).map(|body| (*path, body)));
        match found {
            None => findings.push(Finding::general(format!(
                "The test `{}` is missing. {why_needed} write it again or, if the rule changed, \
                 change this list.",
                each.name
            ))),
            Some((path, body)) if each.both_databases && !body.contains(BOTH_DATABASES) => {
                findings.push(Finding::at(
                    path,
                    format!(
                        "The test `{}` has to run on both databases: wrap it in `each_database` \
                         (ARCH-DB-PARITY).",
                        each.name
                    ),
                ));
            }
            Some(_) => {}
        }
    }
    findings
}

/// What the example tests of a check prove, once for every check that has a list of tests.
#[cfg(test)]
pub(super) mod examples {
    use std::fmt::Write as _;

    use super::{Required, evaluate};
    use crate::source::Source;

    /// A file with every required test, each with `wrapper` as its body.
    fn all_tests(required: &[Required], wrapper: &str) -> Vec<Source> {
        let mut text = String::new();
        for each in required {
            let _ = write!(
                text,
                "#[tokio::test]
async fn {}() {{
    {wrapper}
}}

",
                each.name
            );
        }
        vec![("crates/x/src/tests.rs".to_owned(), text)]
    }

    /// Every test exists and runs on both databases: no finding.
    pub(in super::super) fn every_test_is_there(required: &[Required]) {
        let sources = all_tests(required, "each_database(\"x\", body).await");
        assert!(evaluate(&sources, required, "").is_empty());
    }

    /// A missing test, and a test on one database only, are found.
    pub(in super::super) fn a_missing_or_single_test_is_found(required: &[Required]) {
        let missing = evaluate(
            &[("crates/x/src/tests.rs".to_owned(), String::new())],
            required,
            "",
        );
        assert_eq!(missing.len(), required.len());
        let single = evaluate(&all_tests(required, "body().await"), required, "");
        assert_eq!(
            single.len(),
            required.iter().filter(|each| each.both_databases).count()
        );
    }

    /// A name in a comment is not a test.
    pub(in super::super) fn a_name_in_a_comment_is_no_test(required: &[Required], name: &str) {
        let mut sources = all_tests(required, "each_database(\"x\", body).await");
        for (_, text) in &mut sources {
            *text = text.replace(&format!("async fn {name}"), &format!("// async fn {name}"));
        }
        assert_eq!(evaluate(&sources, required, "").len(), 1);
    }
}
