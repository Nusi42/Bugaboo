//! ARCH-DEPS: crates depend only on what the module map allows (architecture §4, ADR-0001).
//!
//! The allowed edges are read from the Mermaid flowchart under "Module map" in
//! `docs/architecture/04-building-blocks.md`, so the document is the single source:
//! `a --> b` and `a -.-> b` mean "a may depend on b", `a --> b & c` lists several targets.

use std::collections::BTreeSet;

use super::{Check, Context, Finding};
use crate::docs::building_blocks::{Map, parse_module_map};
use crate::docs::source::BUILDING_BLOCKS as MAP_DOC;
use crate::metadata::{self, Package};
use crate::source::read_text;

/// Tooling crates that live in the workspace but are not part of the module map.
const OUTSIDE_THE_MAP: &[&str] = &["xtask"];

/// The database test harness: allowed as a dev-dependency only.
const TEST_SUPPORT: &str = "bb-testing";

/// The feature of a crate that opens test hooks (`bb-store`, `bb-app`, `bb-api`).
const TESTING_FEATURE: &str = "testing";

/// The developer tools that may switch `testing` on in a normal dependency: they run without an
/// attacker, on a database of their own. Every other crate switches it on in `[dev-dependencies]`
/// only, so the program that ships never has the hooks.
const TOOLS_WITH_TESTING: [&str; 2] = ["bb-sample", "bb-load-check"];

pub(crate) struct ArchDeps;

impl Check for ArchDeps {
    fn id(&self) -> &'static str {
        "ARCH-DEPS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let root = context.repo.root();
        let text = read_text(context.repo, MAP_DOC)?;
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
            if dependency.name == TEST_SUPPORT {
                // Test support may be used by any crate's tests, and by nothing else.
                if dependency.kind.as_deref() != Some("dev") {
                    findings.push(Finding::at(
                        name,
                        format!(
                            "depends on `{TEST_SUPPORT}` outside `[dev-dependencies]`. The test \
                             harness must never become part of a shipped crate."
                        ),
                    ));
                }
                continue;
            }
            if dependency.kind.is_none()
                && dependency
                    .features
                    .iter()
                    .any(|each| each == TESTING_FEATURE)
                && !TOOLS_WITH_TESTING.contains(&name)
            {
                findings.push(Finding::at(
                    name,
                    format!(
                        "switches on `{TESTING_FEATURE}` of `{}` outside `[dev-dependencies]`. The test hooks must not be in the program that ships; only the developer tools {TOOLS_WITH_TESTING:?} may.",
                        dependency.name
                    ),
                ));
            }
            // A crate may name itself as a dev-dependency to switch on its own `testing` feature
            // for its integration tests. That is no edge between two crates.
            if dependency.name == name && dependency.kind.as_deref() == Some("dev") {
                continue;
            }
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
    fn bad_example_the_test_harness_as_a_normal_dependency() {
        let map = parse_map(MAP).unwrap();
        let mut harness = package("bb-testing", &[]);
        harness.name = "bb-testing".to_owned();
        let mut store = package("bb-store", &["bb-testing"]);
        let mut packages = crates(&[]);
        packages.retain(|p| p.name != "bb-store");
        store.dependencies[0].kind = None;
        packages.extend([store, harness]);
        let findings = evaluate(&packages, &map);
        assert!(
            findings
                .iter()
                .any(|f| f.message.contains("outside `[dev-dependencies]`")),
            "{:?}",
            findings.iter().map(|f| &f.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn good_example_the_test_harness_as_a_dev_dependency() {
        let map = parse_map(MAP).unwrap();
        let mut store = package("bb-store", &["bb-testing"]);
        store.dependencies[0].kind = Some("dev".to_owned());
        let mut packages = crates(&[]);
        packages.retain(|p| p.name != "bb-store");
        packages.extend([store, package("bb-testing", &[])]);
        let findings = evaluate(&packages, &map);
        assert!(
            findings
                .iter()
                .all(|f| !f.message.contains("dev-dependencies")),
            "{:?}",
            findings.iter().map(|f| &f.message).collect::<Vec<_>>()
        );
    }

    fn with_testing(name: &str, kind: Option<&str>) -> Vec<Package> {
        let mut app = package(name, &["bb-core"]);
        app.dependencies[0].features = vec![TESTING_FEATURE.to_owned()];
        app.dependencies[0].kind = kind.map(str::to_owned);
        let mut packages = crates(&[]);
        packages.retain(|p| p.name != name);
        packages.push(app);
        packages
    }

    #[test]
    fn bad_example_a_shipped_crate_switches_the_testing_hooks_on() {
        let findings = evaluate(&with_testing("bb-app", None), &parse_map(MAP).unwrap());
        assert!(
            findings
                .iter()
                .any(|f| f.message.contains("switches on `testing`")),
            "{:?}",
            findings.iter().map(|f| &f.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn good_example_the_testing_hooks_in_dev_dependencies_and_in_tools() {
        let map = parse_map(MAP).unwrap();
        let in_dev = evaluate(&with_testing("bb-app", Some("dev")), &map);
        assert!(in_dev.iter().all(|f| !f.message.contains("switches on")));
        // A tool is not in this small map, so only the one rule is looked at.
        let tool = evaluate(&with_testing("bb-sample", None), &map);
        assert!(tool.iter().all(|f| !f.message.contains("switches on")));
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
        let text = std::fs::read_to_string(root.join(MAP_DOC)).unwrap();
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
    fn good_example_a_crate_switches_on_its_own_test_feature_as_a_dev_dependency() {
        let mut own = package("bb-app", &["bb-app"]);
        own.dependencies[0].kind = Some("dev".to_owned());
        let mut packages = crates(&[]);
        packages.retain(|p| p.name != "bb-app");
        packages.push(own);
        assert!(evaluate(&packages, &parse_map(MAP).unwrap()).is_empty());
    }

    #[test]
    fn bad_example_a_crate_depending_on_itself_outside_dev_dependencies() {
        let packages = crates(&[("bb-app", &["bb-app"])]);
        let findings = evaluate(&packages, &parse_map(MAP).unwrap());
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("bb-app"));
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
