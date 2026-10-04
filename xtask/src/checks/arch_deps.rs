//! ARCH-DEPS: crates depend only on what the module map allows (architecture §4, ADR-0001).

use std::collections::{BTreeSet, HashMap};

use super::{Check, Context, Finding};
use crate::metadata::{self, Package};

/// The allowed dependency edges between workspace crates, written once.
///
/// Source: the module map and the dependency rules in `docs/architecture/04-building-blocks.md`.
/// Every crate may use `bb-core` (the shared vocabulary) except the three crates that must
/// compile to WebAssembly without it (`bb-searchlang`, `bb-markdown`) and `bb-core` itself.
/// `bb-core` may use `bb-macros` for its derives (personal-data marks, ADR-0015).
/// Dashed arrows in the map ("implements ports") become `bb-app` edges for the adapters.
const ALLOWED: &[(&str, &[&str])] = &[
    ("bb-core", &["bb-macros"]),
    ("bb-macros", &[]),
    ("bb-searchlang", &[]),
    ("bb-markdown", &[]),
    ("bb-policy", &["bb-core"]),
    (
        "bb-app",
        &["bb-core", "bb-policy", "bb-searchlang", "bb-markdown"],
    ),
    (
        "bb-store",
        &["bb-core", "bb-app", "bb-policy", "bb-searchlang"],
    ),
    ("bb-files", &["bb-core", "bb-app"]),
    ("bb-auth", &["bb-core", "bb-app"]),
    ("bb-ai", &["bb-core", "bb-app"]),
    ("bb-notify", &["bb-core", "bb-app"]),
    ("bb-jobs", &["bb-core", "bb-app"]),
    ("bb-mcp", &["bb-core", "bb-app", "bb-ai"]),
    (
        "bb-api",
        &["bb-core", "bb-app", "bb-policy", "bb-auth", "bb-mcp"],
    ),
    ("bb-wasm", &["bb-searchlang", "bb-markdown"]),
    (
        "bb-server",
        &[
            "bb-core",
            "bb-app",
            "bb-policy",
            "bb-api",
            "bb-mcp",
            "bb-jobs",
            "bb-store",
            "bb-files",
            "bb-ai",
            "bb-notify",
            "bb-auth",
        ],
    ),
];

/// Tooling crates that live in the workspace but are not part of the module map.
const OUTSIDE_THE_MAP: &[&str] = &["xtask"];

pub(crate) struct ArchDeps;

impl Check for ArchDeps {
    fn id(&self) -> &'static str {
        "ARCH-DEPS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let packages = metadata::load(context.repo.root())?;
        Ok(evaluate(&packages))
    }
}

fn evaluate(packages: &[Package]) -> Vec<Finding> {
    let allowed: HashMap<&str, &[&str]> = ALLOWED.iter().copied().collect();
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
        let Some(allowed_here) = allowed.get(name) else {
            findings.push(Finding::at(
                name,
                "This crate is not in the module map. Add it to architecture §4 and to the allowed \
                 edges in xtask/src/checks/arch_deps.rs first.",
            ));
            continue;
        };
        for dependency in &package.dependencies {
            let is_ours =
                members.contains(dependency.name.as_str()) || dependency.name.starts_with("bb-");
            if is_ours && !allowed_here.contains(&dependency.name.as_str()) {
                findings.push(Finding::at(
                    name,
                    format!(
                        "depends on `{}`, which the module map does not allow. Change the design \
                         (ports and adapters) or update architecture §4 and the allowed edges \
                         together.",
                        dependency.name
                    ),
                ));
            }
        }
    }
    for (name, _) in ALLOWED {
        if !members.contains(name) {
            findings.push(Finding::at(
                *name,
                "The module map lists this crate but the workspace has no such member.",
            ));
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::fixtures::package;

    #[test]
    fn real_workspace_follows_the_map() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap();
        let packages = metadata::load(root).unwrap();
        let findings = evaluate(&packages);
        assert!(
            findings.is_empty(),
            "{:?}",
            findings.iter().map(|f| &f.message).collect::<Vec<_>>()
        );
    }

    fn all_crates(overrides: &[(&str, &[&str])]) -> Vec<Package> {
        ALLOWED
            .iter()
            .map(|(name, deps)| {
                let deps = overrides
                    .iter()
                    .find(|(n, _)| n == name)
                    .map_or(*deps, |(_, d)| *d);
                package(name, deps)
            })
            .collect()
    }

    #[test]
    fn good_example() {
        assert!(evaluate(&all_crates(&[])).is_empty());
    }

    #[test]
    fn bad_example_app_must_not_import_an_adapter() {
        let packages = all_crates(&[("bb-app", &["bb-core", "bb-store"])]);
        let findings = evaluate(&packages);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("bb-store"));
    }

    #[test]
    fn bad_example_pure_crates_must_stay_pure() {
        let packages = all_crates(&[("bb-searchlang", &["bb-core"])]);
        assert_eq!(evaluate(&packages).len(), 1);
    }

    #[test]
    fn bad_example_unknown_crate() {
        let mut packages = all_crates(&[]);
        packages.push(package("bb-extra", &[]));
        assert_eq!(evaluate(&packages).len(), 1);
    }
}
