//! DOC-ADR-ENFORCED: every decision record names the checks that enforce it (ADR-0020).

use super::{Check, Context, Finding};
use crate::docs::source::read_adrs;
use crate::docs::text;

pub(crate) struct DocAdrEnforced;

impl Check for DocAdrEnforced {
    fn id(&self) -> &'static str {
        "DOC-ADR-ENFORCED"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let records = read_adrs(context.repo)?;
        Ok(evaluate(&records, &context.catalog.ids()))
    }
}

/// The text of the `## Enforced by` section, if the record has one.
pub(super) fn enforced_by_section(md: &str) -> Option<String> {
    let mut lines = md
        .lines()
        .skip_while(|line| line.trim_end() != "## Enforced by");
    lines.next()?;
    let section: Vec<&str> = lines
        .take_while(|line| !line.starts_with("## ") && !line.starts_with("<!--"))
        .collect();
    Some(section.join("\n"))
}

fn evaluate(records: &[(String, String)], catalog_ids: &[&str]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (path, md) in records {
        let Some(section) = enforced_by_section(md) else {
            findings.push(Finding::at(
                path,
                "There is no `## Enforced by` section. Name the checks that enforce this decision, or write `review` and why.",
            ));
            continue;
        };
        let tokens = text::check_id_tokens(&section);
        let says_review = text::says_review_with_reason(&section);
        if tokens.is_empty() && !says_review {
            findings.push(Finding::at(
                path,
                "The `## Enforced by` section names no check. Name catalogue IDs, or write `review` and the reason.",
            ));
        }
        for token in tokens {
            if !catalog_ids.iter().any(|id| text::token_matches(&token, id)) {
                findings.push(Finding::at(
                    path,
                    format!(
                        "`{token}` is not in the check catalogue. Fix the ID or add the check to tools/checks/catalog.toml."
                    ),
                ));
            }
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDS: [&str; 2] = ["ARCH-DEPS", "FMT-RUST"];

    fn record(section: &str) -> Vec<(String, String)> {
        vec![(
            "docs/adr/0001-x.md".to_owned(),
            format!("# ADR-0001: X\n\n## Decision\nWe must.\n\n{section}"),
        )]
    }

    #[test]
    fn good_example() {
        for section in [
            "## Enforced by\nARCH-DEPS, FMT-* (see the catalogue).\n",
            "## Enforced by\nreview (a design choice).\n\n<!-- note -->\n",
        ] {
            assert!(evaluate(&record(section), &IDS).is_empty(), "{section}");
        }
    }

    #[test]
    fn bad_example() {
        let missing = evaluate(&record(""), &IDS);
        assert!(missing[0].message.contains("no `## Enforced by`"));
        let empty = evaluate(&record("## Enforced by\nSoon.\n"), &IDS);
        assert!(empty[0].message.contains("names no check"));
        let unknown = evaluate(&record("## Enforced by\nARCH-NOPE, LINT-*.\n"), &IDS);
        assert_eq!(unknown.len(), 2);
        assert!(unknown[0].message.contains("ARCH-NOPE"));
    }
}
