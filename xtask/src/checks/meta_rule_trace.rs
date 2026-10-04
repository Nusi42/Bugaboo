//! META-RULE-TRACE: every rule in the rules documents names the check that enforces it, or says
//! `review` and why (architecture §10).
//!
//! A rule is a sentence with "must", "shall", "has to", "may only" and the like
//! ([`text::states_rule`]). It is traced when its paragraph, list item or table row names a
//! catalogue ID or says `review` with a reason ([`text::traces`]).
//!
//! Scope: the coding standards and the architecture chapters in full; decision records in their
//! `## Decision` and `## Consequences` sections (the context explains why, it does not decide).

use super::{Check, Context, Finding};
use crate::docs::source::{ADR_DIR, CODING_STANDARDS, read_adrs, read_chapters, read_text};
use crate::docs::text;

/// The decision-record sections that hold rules.
const ADR_RULE_SECTIONS: [&str; 2] = ["Decision", "Consequences"];

pub(crate) struct MetaRuleTrace;

impl Check for MetaRuleTrace {
    fn id(&self) -> &'static str {
        "META-RULE-TRACE"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let repo = context.repo;
        let ids = context.catalog.ids();
        let mut documents = vec![(
            CODING_STANDARDS.to_owned(),
            read_text(repo, CODING_STANDARDS)?,
        )];
        documents.extend(read_chapters(repo)?);
        documents.extend(read_adrs(repo)?);
        let mut findings = Vec::new();
        for (path, md) in &documents {
            findings.extend(evaluate(path, md, &ids));
        }
        Ok(findings)
    }
}

/// One paragraph, list item or table row.
struct Unit {
    line: usize,
    /// The `## ` section it is in.
    section: String,
    text: String,
}

/// Splits Markdown into units, leaving out code blocks, table headers and headings.
fn units(md: &str) -> Vec<Unit> {
    let mut units: Vec<Unit> = Vec::new();
    let mut section = String::new();
    let mut in_code = false;
    let mut previous_blank = true;
    let mut table_row = 0_usize;
    for (index, line) in md.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_code = !in_code;
            previous_blank = true;
            continue;
        }
        if in_code {
            continue;
        }
        if trimmed.is_empty() {
            previous_blank = true;
            table_row = 0;
            continue;
        }
        if trimmed.starts_with('#') {
            if let Some(title) = trimmed.strip_prefix("## ") {
                title.trim().clone_into(&mut section);
            }
            previous_blank = true;
            continue;
        }
        if trimmed.starts_with('|') {
            table_row += 1;
            // The first two rows are the header and its separator: no rules in there.
            if table_row > 2 {
                units.push(Unit {
                    line: index + 1,
                    section: section.clone(),
                    text: line.to_owned(),
                });
            }
            previous_blank = false;
            continue;
        }
        table_row = 0;
        let starts_item = trimmed.starts_with("- ")
            || trimmed.starts_with("* ")
            || trimmed.split_once(". ").is_some_and(|(number, _)| {
                !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit())
            });
        match units.last_mut() {
            Some(unit) if !previous_blank && !starts_item && !unit.text.starts_with('|') => {
                unit.text.push('\n');
                unit.text.push_str(line);
            }
            _ => units.push(Unit {
                line: index + 1,
                section: section.clone(),
                text: line.to_owned(),
            }),
        }
        previous_blank = false;
    }
    units
}

fn evaluate(path: &str, md: &str, catalog_ids: &[&str]) -> Vec<Finding> {
    let is_record = path.starts_with(&format!("{ADR_DIR}/"));
    units(md)
        .into_iter()
        .filter(|unit| !is_record || ADR_RULE_SECTIONS.contains(&unit.section.as_str()))
        .filter(|unit| text::states_rule(&unit.text) && !text::traces(&unit.text, catalog_ids))
        .map(|unit| {
            Finding::at(
                format!("{path}:{}", unit.line),
                "This rule says what must, shall or may only happen, but names no check. Add `Enforced by: <check ID>` from the catalogue, or write `review` and why a person decides.",
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDS: [&str; 2] = ["ARCH-DEPS", "LINT-RUST"];

    #[test]
    fn good_example() {
        let md = "\
# Rules

- Crates must not depend on each other (ARCH-DEPS).
- Names have to be kind; review: a person reads every new name.

A paragraph that has no rule word.

| Scenario | What must hold |
|---|---|
| Lint | LINT-RUST stays green |
| Spelling | Words may only be plain; review: the glossary decides |

```text
A line in code that must be ignored.
```
";
        assert!(evaluate(CODING_STANDARDS, md, &IDS).is_empty());
    }

    #[test]
    fn bad_example() {
        let md = "\
# Rules

- Crates must not depend on each other (ARCH-NOPE).
- Another item that is fine (ARCH-DEPS).
- Tests needs to pass.
- Names shall be kind, review.

A paragraph, second line starts here and
runs on: it may only be unique.

| Scenario | What must hold |
|---|---|
| Lint | The linter is required to run |
| Spelling | Fine (LINT-RUST) |
";
        let findings = evaluate(CODING_STANDARDS, md, &IDS);
        let places: Vec<&str> = findings
            .iter()
            .filter_map(|f| f.location.as_deref())
            .collect();
        let at = |line: usize| format!("{CODING_STANDARDS}:{line}");
        assert_eq!(places, [at(3), at(5), at(6), at(8), at(13)]);
    }

    #[test]
    fn a_decision_record_is_checked_in_decision_and_consequences_only() {
        let md = "\
# ADR

## Context
We must be fast.

## Decision
We must be small.

## Consequences
- Builds have to stay green (ARCH-DEPS).
- Reviews shall be kind.

## Enforced by
ARCH-DEPS (see the catalogue).
";
        let path = format!("{ADR_DIR}/0001-x.md");
        let findings = evaluate(&path, md, &IDS);
        let places: Vec<&str> = findings
            .iter()
            .filter_map(|f| f.location.as_deref())
            .collect();
        assert_eq!(places, [format!("{path}:7"), format!("{path}:11")]);
        assert_eq!(evaluate(CODING_STANDARDS, md, &IDS).len(), 3);
    }
}
