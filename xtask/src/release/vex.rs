//! The VEX statements of a release (COMP-03, M6-F126, ADR-0047 decision 4): one `CycloneDX` VEX file
//! with one statement, *not affected*, for each advisory that Bugaboo accepts.
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

/// The VEX document of `list`: a statement for each advisory in `accepted`.
pub(crate) fn vex(list: &PartsList, accepted: &[Accepted], register: &Rows) -> Value {
    let program = &list.program;
    let mut hasher = Sha256::new();
    hasher.update(program.reference());
    for entry in accepted {
        hasher.update(&entry.id);
    }
    let digest = bb_core::hex::encode(&hasher.finalize());
    let statements: Vec<Value> = accepted
        .iter()
        .map(|entry| {
            let row = register.risks.get(&entry.id);
            let detail = row.map(|row| row.purpose.clone()).unwrap_or_default();
            let mut statement = json!({
                "id": entry.id,
                "source": source(&entry.id),
                "analysis": {"state": "not_affected", "detail": detail},
                "affects": [{"ref": program.reference()}],
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
        .collect();
    json!({
        "$schema": "http://cyclonedx.org/schema/bom-1.6.schema.json",
        "bomFormat": "CycloneDX",
        "specVersion": "1.6",
        "serialNumber": super::formats::serial_number(&digest),
        "version": 1,
        "metadata": {
            "timestamp": list.created,
            "component": {
                "type": "application",
                "bom-ref": program.reference(),
                "name": program.name,
                "version": program.version,
            },
        },
        "vulnerabilities": statements,
    })
}

#[cfg(test)]
mod tests;
