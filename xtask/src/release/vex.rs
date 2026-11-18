//! The VEX statements of a release (COMP-03, M6-F126, ADR-0047 decision 4): one CycloneDX VEX file
//! with one statement, *not affected*, for each advisory that Bugaboo accepts.
//!
//! The advisories are the entries of `deny.toml` (`[advisories] ignore`) and of
//! `tools/checks/npm-advisories.toml` (`accepted`). The reason is the one written in the
//! register's *Accepted risks*, so it is written once; an entry the register does not describe
//! falls back to its own reason line.

use std::fs;
use std::path::Path;

use anyhow::Context as _;
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

use super::hex;
use super::parts_list::PartsList;
use crate::checks::dep_register::Rows;
use crate::tools::DENY_CONFIG;

/// The accepted npm advisories.
const NPM_ADVISORIES: &str = "tools/checks/npm-advisories.toml";

/// One accepted advisory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Accepted {
    pub(crate) id: String,
    /// The reason line of the entry that accepts it.
    pub(crate) reason: String,
}

/// The advisories an entry list accepts: items of `list` in the table `section`, each a table
/// with an `id` and a `reason`.
fn entries(text: &str, path: &[&str]) -> anyhow::Result<Vec<Accepted>> {
    let value: toml::Value = toml::from_str(text).context("not valid TOML")?;
    let mut at = Some(&value);
    for key in path {
        at = at.and_then(|table| table.get(key));
    }
    let mut accepted = Vec::new();
    for item in at.and_then(toml::Value::as_array).into_iter().flatten() {
        let text = |key: &str| item.get(key).and_then(toml::Value::as_str);
        match (item.as_str(), text("id")) {
            (Some(id), _) | (None, Some(id)) => accepted.push(Accepted {
                id: id.to_owned(),
                reason: text("reason").unwrap_or_default().to_owned(),
            }),
            _ => {}
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
    let digest = hex(&hasher.finalize());
    let statements: Vec<Value> = accepted
        .iter()
        .map(|entry| {
            let row = register.risks.get(&entry.id);
            let detail = row
                .map(|row| row.purpose.clone())
                .filter(|text| !text.is_empty())
                .unwrap_or_else(|| entry.reason.clone());
            let mut statement = json!({
                "id": entry.id,
                "source": source(&entry.id),
                "analysis": {"state": "not_affected", "detail": detail},
                "affects": [{"ref": program.reference()}],
            });
            if let (Some(row), Some(map)) = (row, statement.as_object_mut())
                && let Some(package) = row.cells.get(1)
            {
                map.insert("description".into(), json!(package));
            }
            statement
        })
        .collect();
    json!({
        "$schema": "http://cyclonedx.org/schema/bom-1.6.schema.json",
        "bomFormat": "CycloneDX",
        "specVersion": "1.6",
        "serialNumber": format!(
            "urn:uuid:{}-{}-4{}-a{}-{}",
            &digest[0..8],
            &digest[8..12],
            &digest[13..16],
            &digest[17..20],
            &digest[20..32]
        ),
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
