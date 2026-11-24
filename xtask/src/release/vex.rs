//! The VEX statements of a release (COMP-03, ADR-0047 decision 4, as amended): one statement,
//! *not affected*, for each advisory that Bugaboo accepts, in the `vulnerabilities` of the
//! `CycloneDX` parts list itself, so the release carries one document for both
//! ([`with_statements`]).
//!
//! The advisories are the entries of `deny.toml` (`[advisories] ignore`) and of
//! `tools/checks/npm-advisories.toml` (`accepted`). The reason is the one written in the
//! register's *Accepted risks*, so it is written once. An accepted advisory without its row has no
//! reason to give: its statement is empty of detail and DEP-SBOM fails (`problems`).

use std::fs;
use std::path::Path;

use anyhow::Context as _;
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

use super::parts_list::PartsList;
use crate::checks::dep_register::Rows;
use crate::tools::DENY_CONFIG;

/// The accepted npm advisories.
const NPM_ADVISORIES: &str = "tools/checks/npm-advisories.toml";

/// One accepted advisory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Accepted {
    pub(crate) id: String,
}

/// The advisories an entry list accepts: items of `list` in the table `section`, each a table
/// with an `id` (the reason of an entry is its register row's, not read here).
fn entries(text: &str, path: &[&str]) -> anyhow::Result<Vec<Accepted>> {
    let value: toml::Value = toml::from_str(text).context("not valid TOML")?;
    let mut at = Some(&value);
    for key in path {
        at = at.and_then(|table| table.get(key));
    }
    let mut accepted = Vec::new();
    for item in at.and_then(toml::Value::as_array).into_iter().flatten() {
        let id = item
            .as_str()
            .or_else(|| item.get("id").and_then(toml::Value::as_str));
        if let Some(id) = id {
            accepted.push(Accepted { id: id.to_owned() });
        }
    }
    Ok(accepted)
}

/// Every accepted advisory of the repository at `root`, Rust first, in the order of the files.
pub(crate) fn accepted(root: &Path) -> anyhow::Result<Vec<Accepted>> {
    let read = |path: &str| {
        fs::read_to_string(root.join(path)).with_context(|| format!("could not read {path}"))
    };
    let mut all = entries(&read(DENY_CONFIG)?, &["advisories", "ignore"])
        .with_context(|| format!("{DENY_CONFIG} is unreadable"))?;
    all.extend(
        entries(&read(NPM_ADVISORIES)?, &["accepted"])
            .with_context(|| format!("{NPM_ADVISORIES} is unreadable"))?,
    );
    Ok(all)
}

/// The reasons `CycloneDX` knows for *not affected* (`vulnerability.analysis.justification`).
const JUSTIFICATIONS: [&str; 9] = [
    "code_not_present",
    "code_not_reachable",
    "requires_configuration",
    "requires_dependency",
    "requires_environment",
    "protected_by_compiler",
    "protected_at_runtime",
    "protected_at_perimeter",
    "protected_by_mitigating_control",
];

/// What stops an accepted advisory from having a full statement: `(ID, what is wrong)`. Every one
/// needs its row in the register's *Accepted risks*, where the reason and the justification are
/// written once.
pub(crate) fn problems(accepted: &[Accepted], register: &Rows) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for entry in accepted {
        let message = match register.risks.get(&entry.id) {
            None => {
                Some("has no row in the register's Accepted risks, so its statement has no reason.")
            }
            Some(row) if row.purpose.is_empty() => {
                Some("has an empty Why accepted in the register.")
            }
            Some(row) if !JUSTIFICATIONS.contains(&row.justification.as_str()) => Some(
                "has no VEX justification in the register's Accepted risks that CycloneDX knows (for example code_not_present, code_not_reachable or requires_environment).",
            ),
            Some(_) => None,
        };
        if let Some(message) = message {
            found.push((entry.id.clone(), message.to_owned()));
        }
    }
    found
}

/// Where an advisory is published.
fn source(id: &str) -> Value {
    let (name, url) = if id.starts_with("RUSTSEC-") {
        ("RustSec", format!("https://rustsec.org/advisories/{id}"))
    } else if id.starts_with("GHSA-") {
        (
            "GitHub Advisory Database",
            format!("https://github.com/advisories/{id}"),
        )
    } else {
        return json!({"name": "unknown"});
    };
    json!({"name": name, "url": url})
}

/// The statements about the advisories in `accepted` for the program of `list`.
pub(crate) fn statements(list: &PartsList, accepted: &[Accepted], register: &Rows) -> Vec<Value> {
    let program = list.program.reference();
    accepted
        .iter()
        .map(|entry| {
            let row = register.risks.get(&entry.id);
            let detail = row.map(|row| row.purpose.clone()).unwrap_or_default();
            let mut statement = json!({
                "id": entry.id,
                "source": source(&entry.id),
                "analysis": {"state": "not_affected", "detail": detail},
                "affects": [{"ref": program}],
            });
            if let (Some(row), Some(map)) = (row, statement.as_object_mut()) {
                if let Some(package) = row.cells.get(1) {
                    map.insert("description".into(), json!(package));
                }
                if let Some(analysis) = map.get_mut("analysis").and_then(Value::as_object_mut)
                    && !row.justification.is_empty()
                {
                    analysis.insert("justification".into(), json!(row.justification));
                }
            }
            statement
        })
        .collect()
}

/// The `CycloneDX` parts list `document` with `statements` as its `vulnerabilities`. Its
/// `serialNumber` is made anew from the one it had and the IDs of the statements, so the list
/// with statements and the one without never share one. `cargo xtask release` and
/// `cargo xtask release repack` both put the statements in here.
pub(crate) fn with_statements(mut document: Value, statements: Vec<Value>) -> Value {
    let mut hasher = Sha256::new();
    hasher.update(document["serialNumber"].as_str().unwrap_or_default());
    for statement in &statements {
        hasher.update(statement["id"].as_str().unwrap_or_default());
    }
    let digest = bb_core::hex::encode(&hasher.finalize());
    if let Some(map) = document.as_object_mut() {
        map.insert(
            "serialNumber".into(),
            json!(super::formats::serial_number(&digest)),
        );
        map.insert("vulnerabilities".into(), Value::Array(statements));
    }
    document
}

/// What is wrong with the statements of the `CycloneDX` file of a release, one sentence each
/// (the check of a release folder): the file must carry its `vulnerabilities`, and each statement
/// an `analysis` with a `state`.
pub(crate) fn release_problems(document: &Value, file: &str) -> Vec<String> {
    let Some(statements) = document.get("vulnerabilities").and_then(Value::as_array) else {
        return vec![format!(
            "{file} carries no VEX statements (no `vulnerabilities`)."
        )];
    };
    statements
        .iter()
        .filter(|statement| statement.pointer("/analysis/state").is_none())
        .map(|statement| {
            format!(
                "{file}: the statement on {} has no analysis.",
                statement["id"].as_str().unwrap_or("an advisory")
            )
        })
        .collect()
}

#[cfg(test)]
mod tests;
