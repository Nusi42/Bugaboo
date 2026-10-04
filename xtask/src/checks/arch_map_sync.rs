//! ARCH-MAP-SYNC: the repository layout in the architecture equals the workspace (COMP-06).
//!
//! The parser for `docs/architecture/04-building-blocks.md` and the readers for the Cargo and npm
//! workspaces live here once; META-NEW-MODULE uses them too.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use anyhow::Context as _;

use super::{Check, Context, Finding};
use crate::docs::building_blocks::Layout;
use crate::docs::source::{BUILDING_BLOCKS, read_text};

pub(crate) struct ArchMapSync;

impl Check for ArchMapSync {
    fn id(&self) -> &'static str {
        "ARCH-MAP-SYNC"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let md = read_text(context.repo, BUILDING_BLOCKS)?;
        let rust = rust_members(context.repo.root())?;
        let npm = npm_members(context.repo.root())?;
        Ok(evaluate(&md, &rust, npm.as_ref()))
    }
}

/// The Cargo workspace members (`crates/bb-core`, `xtask`), from the root `Cargo.toml`.
pub(super) fn rust_members(root: &Path) -> anyhow::Result<BTreeSet<String>> {
    let text = fs::read_to_string(root.join("Cargo.toml")).context("could not read Cargo.toml")?;
    let manifest: toml::Value = toml::from_str(&text).context("Cargo.toml is not valid")?;
    let patterns: Vec<String> = manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("members"))
        .and_then(toml::Value::as_array)
        .map(|members| {
            members
                .iter()
                .filter_map(|member| member.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    Ok(expand(root, "", &patterns, "Cargo.toml"))
}

/// The npm workspace packages (`web/app`, `web/packages/ui`), or `None` while there is no
/// `web/package.json` yet.
pub(super) fn npm_members(root: &Path) -> anyhow::Result<Option<BTreeSet<String>>> {
    let path = root.join("web").join("package.json");
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).context("could not read web/package.json")?;
    let manifest: serde_json::Value =
        serde_json::from_str(&text).context("web/package.json is not valid")?;
    let workspaces = manifest.get("workspaces");
    let list = workspaces
        .and_then(|value| value.get("packages").or(Some(value)))
        .and_then(serde_json::Value::as_array);
    let patterns: Vec<String> = list
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    Ok(Some(expand(root, "web", &patterns, "package.json")))
}

/// Turns workspace patterns (`crates/*` or a plain folder) into folders that hold `manifest`.
fn expand(root: &Path, base: &str, patterns: &[String], manifest: &str) -> BTreeSet<String> {
    let join = |relative: &str| {
        if base.is_empty() {
            relative.to_owned()
        } else {
            format!("{base}/{relative}")
        }
    };
    let mut found = BTreeSet::new();
    for pattern in patterns {
        let pattern = pattern.trim_start_matches("./");
        if let Some(folder) = pattern.strip_suffix("/*") {
            let Ok(entries) = fs::read_dir(root.join(join(folder))) else {
                continue;
            };
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if entry.path().join(manifest).is_file() {
                    found.insert(join(&format!("{folder}/{name}")));
                }
            }
        } else if root.join(join(pattern)).join(manifest).is_file() {
            found.insert(join(pattern));
        }
    }
    found
}

/// The members that are missing from the layout, as findings.
pub(super) fn missing_from_layout(
    layout: &Layout,
    rust: &BTreeSet<String>,
    npm: Option<&BTreeSet<String>>,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for member in rust.difference(&layout.rust_members()) {
        findings.push(Finding::at(
            BUILDING_BLOCKS,
            format!("The Rust crate `{member}` is in the workspace but not in the repository layout. Add it (with a one-line purpose) and to the module map."),
        ));
    }
    if let Some(npm) = npm {
        for member in npm.difference(&layout.npm_members()) {
            findings.push(Finding::at(
                BUILDING_BLOCKS,
                format!("The browser package `{member}` is in the npm workspaces but not in the repository layout. Add it with a one-line purpose."),
            ));
        }
    }
    findings
}

fn evaluate(md: &str, rust: &BTreeSet<String>, npm: Option<&BTreeSet<String>>) -> Vec<Finding> {
    let Some(layout) = Layout::parse(md) else {
        return vec![Finding::at(
            BUILDING_BLOCKS,
            "There is no repository layout block under `## Repository layout`.",
        )];
    };
    let mut findings = missing_from_layout(&layout, rust, npm);
    for member in layout.rust_members().difference(rust) {
        findings.push(Finding::at(
            BUILDING_BLOCKS,
            format!("The repository layout lists `{member}`, but it is not a workspace member. Remove it or add the crate."),
        ));
    }
    if let Some(npm) = npm {
        for member in layout.npm_members().difference(npm) {
            findings.push(Finding::at(
                BUILDING_BLOCKS,
                format!("The repository layout lists `{member}`, but it is not an npm workspace. Remove it or add the package."),
            ));
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docs::building_blocks::fixtures::{LAYOUT, set};
    use crate::docs::building_blocks::parse_module_map;

    #[test]
    fn layout_is_parsed_into_paths() {
        let layout = Layout::parse(LAYOUT).unwrap();
        assert_eq!(
            layout.rust_members(),
            set(&["crates/bb-api", "crates/bb-core", "xtask"])
        );
        assert_eq!(layout.npm_members(), set(&["web/app", "web/packages/ui"]));
        assert!(layout.top_level().contains("design"));
        assert!(parse_module_map(LAYOUT).unwrap().contains_key("bb-api"));
    }

    #[test]
    fn good_example() {
        let rust = set(&["crates/bb-api", "crates/bb-core", "xtask"]);
        let npm = set(&["web/app", "web/packages/ui"]);
        assert!(evaluate(LAYOUT, &rust, Some(&npm)).is_empty());
        assert!(evaluate(LAYOUT, &rust, None).is_empty());
    }

    #[test]
    fn bad_example() {
        let rust = set(&["crates/bb-core", "crates/bb-new", "xtask"]);
        let npm = set(&["web/app", "web/packages/extra"]);
        let findings = evaluate(LAYOUT, &rust, Some(&npm));
        let text: Vec<&str> = findings.iter().map(|f| f.message.as_str()).collect();
        let has = |needle: &str| text.iter().any(|message| message.contains(needle));
        assert!(has("`crates/bb-new` is in the workspace"));
        assert!(has("`crates/bb-api`, but it is not a workspace member"));
        assert!(has("`web/packages/extra` is in the npm workspaces"));
        assert!(has("`web/packages/ui`, but it is not an npm workspace"));
        assert_eq!(findings.len(), 4);
    }
}
