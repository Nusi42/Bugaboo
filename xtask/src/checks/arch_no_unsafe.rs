//! ARCH-NO-UNSAFE: every crate forbids unsafe code unless an ADR allows it (coding-standards §3).

use std::fs;
use std::path::Path;

use anyhow::Context as _;

use super::{Check, Context, Finding};
use crate::metadata::{self, Package};

/// The attribute every crate root must carry.
const FORBID: &str = "#![forbid(unsafe_code)]";

pub(crate) struct ArchNoUnsafe;

impl Check for ArchNoUnsafe {
    fn id(&self) -> &'static str {
        "ARCH-NO-UNSAFE"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let root = context.repo.root();
        let packages = metadata::load(root)?;
        evaluate(root, &packages)
    }
}

fn evaluate(root: &Path, packages: &[Package]) -> anyhow::Result<Vec<Finding>> {
    let mut findings = Vec::new();
    let workspace = read_toml(&root.join("Cargo.toml"))?;
    let workspace_level = workspace
        .get("workspace")
        .and_then(|w| w.get("lints"))
        .and_then(|l| l.get("rust"))
        .and_then(|r| r.get("unsafe_code"))
        .and_then(toml::Value::as_str);
    if workspace_level.is_some_and(|level| level != "forbid") {
        findings.push(Finding::at(
            "Cargo.toml",
            "[workspace.lints.rust] sets `unsafe_code` to something other than \"forbid\".",
        ));
    }
    for package in packages {
        let manifest = read_toml(&package.manifest_path)?;
        let lints = manifest.get("lints");
        let overrides = lints
            .and_then(|l| l.get("rust"))
            .and_then(|r| r.get("unsafe_code"))
            .is_some();
        if overrides {
            findings.push(Finding::at(
                &package.name,
                "overrides `unsafe_code` in its own [lints.rust]. Unsafe code needs an ADR first.",
            ));
        }
        let inherits = lints
            .and_then(|l| l.get("workspace"))
            .and_then(toml::Value::as_bool)
            == Some(true);
        if !inherits && !overrides {
            findings.push(Finding::at(
                &package.name,
                "does not inherit the workspace lints. Add `[lints]` with `workspace = true`.",
            ));
        }
        for source in package.roots() {
            let text = fs::read_to_string(source)
                .with_context(|| format!("could not read {}", source.display()))?;
            if !text.lines().any(|line| line.trim() == FORBID) {
                findings.push(Finding::at(
                    source
                        .strip_prefix(root)
                        .unwrap_or(source)
                        .to_string_lossy()
                        .replace('\\', "/"),
                    format!("The crate root must contain `{FORBID}` (coding-standards §3)."),
                ));
            }
        }
    }
    Ok(findings)
}

fn read_toml(path: &Path) -> anyhow::Result<toml::Value> {
    let text =
        fs::read_to_string(path).with_context(|| format!("could not read {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("{} is not valid TOML", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::fixtures::package;
    use crate::testutil::TempDir;

    fn workspace(temp: &TempDir, manifest_lints: &str, root_source: &str) -> Vec<Package> {
        temp.write(
            "Cargo.toml",
            "[workspace]\nmembers = [\"crates/*\"]\n\n[workspace.lints.rust]\nunsafe_code = \"forbid\"\n",
        );
        temp.write(
            "crates/bb-demo/Cargo.toml",
            format!("[package]\nname = \"bb-demo\"\n{manifest_lints}"),
        );
        temp.write("crates/bb-demo/src/lib.rs", root_source);
        let mut demo = package("bb-demo", &[]);
        demo.manifest_path = temp.path().join("crates/bb-demo/Cargo.toml");
        demo.targets[0].src_path = temp.path().join("crates/bb-demo/src/lib.rs");
        vec![demo]
    }

    #[test]
    fn good_example() {
        let temp = TempDir::new("no-unsafe-good");
        let packages = workspace(
            &temp,
            "\n[lints]\nworkspace = true\n",
            "//! Docs.\n\n#![forbid(unsafe_code)]\n",
        );
        assert!(evaluate(temp.path(), &packages).unwrap().is_empty());
    }

    #[test]
    fn bad_example_missing_attribute() {
        let temp = TempDir::new("no-unsafe-missing");
        let packages = workspace(&temp, "\n[lints]\nworkspace = true\n", "//! Docs.\n");
        assert_eq!(evaluate(temp.path(), &packages).unwrap().len(), 1);
    }

    #[test]
    fn bad_example_lint_override() {
        let temp = TempDir::new("no-unsafe-override");
        let packages = workspace(
            &temp,
            "\n[lints.rust]\nunsafe_code = \"allow\"\n",
            "#![forbid(unsafe_code)]\n",
        );
        assert_eq!(evaluate(temp.path(), &packages).unwrap().len(), 1);
    }

    #[test]
    fn bad_example_not_inheriting_workspace_lints() {
        let temp = TempDir::new("no-unsafe-inherit");
        let packages = workspace(&temp, "", "#![forbid(unsafe_code)]\n");
        assert_eq!(evaluate(temp.path(), &packages).unwrap().len(), 1);
    }
}
