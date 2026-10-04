//! META-CATALOG: the catalogue and the implementations match (ADR-0020).

use std::collections::HashSet;

use super::{Check, Context, Finding};
use crate::catalog::{Catalog, STAGES, STATUSES};

pub(crate) struct MetaCatalog;

impl Check for MetaCatalog {
    fn id(&self) -> &'static str {
        "META-CATALOG"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let implemented: Vec<&str> = super::registry().iter().map(|check| check.id()).collect();
        Ok(evaluate(context.catalog, &implemented))
    }
}

fn evaluate(catalog: &Catalog, implemented: &[&str]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut seen = HashSet::new();
    for entry in catalog.entries() {
        if !seen.insert(entry.id.as_str()) {
            findings.push(Finding::at(
                &entry.id,
                "This ID appears more than once in the catalogue.",
            ));
        }
        if !STATUSES.contains(&entry.status.as_str()) {
            findings.push(Finding::at(
                &entry.id,
                format!(
                    "Unknown status `{}`; use one of: {}.",
                    entry.status,
                    STATUSES.join(", ")
                ),
            ));
        }
        if entry.when.is_empty() {
            findings.push(Finding::at(
                &entry.id,
                "`when` is empty, so this check never runs.",
            ));
        }
        for stage in &entry.when {
            if !STAGES.contains(&stage.as_str()) {
                findings.push(Finding::at(
                    &entry.id,
                    format!(
                        "Unknown stage `{stage}`; use one of: {}.",
                        STAGES.join(", ")
                    ),
                ));
            }
        }
        if entry.files.is_empty() {
            findings.push(Finding::at(
                &entry.id,
                "`files` is empty; use [\"*\"] for every file.",
            ));
        }
        if [&entry.title, &entry.rule, &entry.tool]
            .iter()
            .any(|text| text.trim().is_empty())
        {
            findings.push(Finding::at(
                &entry.id,
                "Title, rule and tool must all be filled in.",
            ));
        }
        let is_implemented = implemented.contains(&entry.id.as_str());
        if entry.is_active() && !is_implemented {
            findings.push(Finding::at(
                &entry.id,
                "Marked active, but xtask has no implementation. Implement it or set status = \"planned\".",
            ));
        }
        if !entry.is_active() && is_implemented {
            findings.push(Finding::at(
                &entry.id,
                "Implemented in xtask, but still planned in the catalogue. Set status = \"active\".",
            ));
        }
    }
    for id in implemented {
        if catalog.get(id).is_none() {
            findings.push(Finding::at(
                *id,
                "Implemented in xtask but missing from the catalogue. Add an entry (ADR-0020).",
            ));
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"
[[check]]
id = "A-1"
title = "A"
rule = "r"
tool = "t"
when = ["commit"]
files = ["*"]
status = "active"

[[check]]
id = "B-1"
title = "B"
rule = "r"
tool = "t"
when = ["ci"]
files = ["*.rs"]
status = "planned"
"#;

    #[test]
    fn good_example() {
        let catalog = Catalog::parse(GOOD).unwrap();
        assert!(evaluate(&catalog, &["A-1"]).is_empty());
    }

    #[test]
    fn bad_example() {
        let bad = GOOD
            .replace("\"ci\"", "\"sometimes\"")
            .replace("id = \"B-1\"", "id = \"A-1\"");
        let catalog = Catalog::parse(&bad).unwrap();
        let findings = evaluate(&catalog, &["X-9"]);
        let text: Vec<&str> = findings.iter().map(|f| f.message.as_str()).collect();
        assert!(text.iter().any(|m| m.contains("more than once")));
        assert!(text.iter().any(|m| m.contains("Unknown stage")));
        assert!(text.iter().any(|m| m.contains("no implementation")));
        assert!(
            text.iter()
                .any(|m| m.contains("missing from the catalogue"))
        );
    }
}
