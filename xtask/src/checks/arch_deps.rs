//! ARCH-DEPS: crates depend only on what the module map allows (architecture §4, ADR-0001).
//!
//! The allowed edges are read from the Mermaid flowchart under "Module map" in
//! `docs/architecture/04-building-blocks.md`, so the document is the single source:
//! `a --> b` and `a -.-> b` mean "a may depend on b", `a --> b & c` lists several targets.

use std::collections::BTreeSet;
use std::fs;

use anyhow::Context as _;

use super::{Check, Context, Finding};
use crate::docs::building_blocks::{Map, parse_module_map};
use crate::docs::source::BUILDING_BLOCKS as MAP_DOC;
use crate::metadata::{self, Package};

/// Tooling crates that live in the workspace but are not part of the module map.
const OUTSIDE_THE_MAP: &[&str] = &["xtask"];

pub(crate) struct ArchDeps;

impl Check for ArchDeps {
    fn id(&self) -> &'static str {
        "ARCH-DEPS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let root = context.repo.root();
        let text = fs::read_to_string(root.join(MAP_DOC))
            .with_context(|| format!("could not read {MAP_DOC}"))?;
        let map = parse_module_map(&text)?;
        Ok(evaluate(&metadata::load(root)?, &map))
    }
}

fn evaluate(packages: &[Package], map: &Map) -> Vec<Finding> {
    let mut findings = Vec::new();
    let members: BTreeSet<&str> = packages
        .iter()
        .map(|package| package.name.as_str())
        .collect();
    for package in packages {
        let name = package.name.as_str();
        if OUTSIDE_THE_MAP.contains(&name) {
            continue;
        }
        let Some(allowed) = map.get(name) else {
            findings.push(Finding::at(
                name,
                format!("This crate is not in the module map. Add it to {MAP_DOC} first."),
            ));
            continue;
        };
        for dependency in &package.dependencies {
            // Only edges between workspace members count; crates.io names are not ours.
            if members.contains(dependency.name.as_str()) && !allowed.contains(&dependency.name) {
                findings.push(Finding::at(
                    name,
                    format!(
                        "depends on `{}`, which the module map does not allow. Change the design \
                         (ports and adapters), or change the map in {MAP_DOC} together with the \
                         dependency rules.",
                        dependency.name
                    ),
                ));
            }
        }
    }
    for name in map.keys() {
        if !members.contains(name.as_str()) {
            findings.push(Finding::at(
                name,
                "The module map lists this crate but the workspace has no such member.",
            ));
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docs::building_blocks::parse_module_map as parse_map;
    use crate::metadata::fixtures::package;

    const MAP: &str = "## Module map\n\n```mermaid\nflowchart TB\n  app[\"bb-app<br/>use cases\"]\n  \
        core[\"bb-core\"]\n  store[\"bb-store\"]\n  macros[\"bb-macros\"]\n\n  \
        app --> core\n  store -. implements ports .-> app\n  store --> core & macros\n  \
        core -.-> macros\n```\n";

    fn crates(deps: &[(&str, &[&str])]) -> Vec<Package> {
        ["bb-app", "bb-core", "bb-store", "bb-macros"]
            .iter()
            .map(|name| {
                let deps = deps
                    .iter()
                    .find(|(n, _)| n == name)
                    .map_or(&[][..], |(_, d)| *d);
                package(name, deps)
            })
            .collect()
    }

    #[test]
    fn map_is_read_from_the_mermaid_flowchart() {
        let map = parse_map(MAP).unwrap();
        let store: Vec<&str> = map["bb-store"].iter().map(String::as_str).collect();
        assert_eq!(store, ["bb-app", "bb-core", "bb-macros"]);
        assert!(map["bb-core"].contains("bb-macros"));
        assert!(map["bb-macros"].is_empty());
    }

    #[test]
    fn real_workspace_follows_the_real_map() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap();
        let text = fs::read_to_string(root.join(MAP_DOC)).unwrap();
        let findings = evaluate(&metadata::load(root).unwrap(), &parse_map(&text).unwrap());
        let messages: Vec<&String> = findings.iter().map(|f| &f.message).collect();
        assert!(findings.is_empty(), "{messages:?}");
    }

    #[test]
    fn good_example() {
        let packages = crates(&[
            ("bb-app", &["bb-core"]),
            ("bb-store", &["bb-app", "bb-core"]),
        ]);
        assert!(evaluate(&packages, &parse_map(MAP).unwrap()).is_empty());
    }

    #[test]
    fn bad_example_app_must_not_import_an_adapter() {
        let packages = crates(&[("bb-app", &["bb-core", "bb-store"])]);
        let findings = evaluate(&packages, &parse_map(MAP).unwrap());
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("bb-store"));
    }

    #[test]
    fn bad_example_unknown_crate_and_missing_crate() {
        let mut packages = crates(&[]);
        packages.push(package("bb-extra", &[]));
        assert_eq!(evaluate(&packages, &parse_map(MAP).unwrap()).len(), 1);
        packages.retain(|p| p.name != "bb-macros" && p.name != "bb-extra");
        assert_eq!(evaluate(&packages, &parse_map(MAP).unwrap()).len(), 1);
    }

    #[test]
    fn external_crates_named_bb_are_ignored() {
        let packages = crates(&[("bb-app", &["bb-from-crates-io"])]);
        assert!(evaluate(&packages, &parse_map(MAP).unwrap()).is_empty());
    }

    #[test]
    fn undeclared_box_in_the_map_is_an_error() {
        let bad = MAP.replace("app --> core", "app --> nowhere");
        assert!(parse_map(&bad).is_err());
    }
}
