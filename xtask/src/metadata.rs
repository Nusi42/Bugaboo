//! The Cargo workspace as `cargo metadata` describes it, for the architecture and dependency checks.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, bail};
use serde::Deserialize;

/// A workspace member.
#[derive(Deserialize, Clone, Debug)]
pub(crate) struct Package {
    pub(crate) name: String,
    pub(crate) manifest_path: PathBuf,
    pub(crate) dependencies: Vec<Dependency>,
    pub(crate) targets: Vec<Target>,
}

/// A dependency written in a member's `Cargo.toml`.
#[derive(Deserialize, Clone, Debug)]
pub(crate) struct Dependency {
    pub(crate) name: String,
    /// Set when the dependency is a folder in this repository.
    pub(crate) path: Option<PathBuf>,
}

/// A library, binary or other target of a package.
#[derive(Deserialize, Clone, Debug)]
pub(crate) struct Target {
    pub(crate) kind: Vec<String>,
    pub(crate) src_path: PathBuf,
}

impl Package {
    /// The crate roots that ship code: libraries, binaries and procedural macros.
    pub(crate) fn roots(&self) -> impl Iterator<Item = &Path> {
        self.targets
            .iter()
            .filter(|target| {
                target
                    .kind
                    .iter()
                    .any(|kind| matches!(kind.as_str(), "lib" | "bin" | "proc-macro"))
            })
            .map(|target| target.src_path.as_path())
    }
}

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
}

/// Runs `cargo metadata --no-deps` in `root` and returns the workspace members.
pub(crate) fn load(root: &Path) -> anyhow::Result<Vec<Package>> {
    let out = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(root)
        .output()
        .context("could not run cargo; is Rust installed?")?;
    if !out.status.success() {
        bail!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    parse(&String::from_utf8_lossy(&out.stdout))
}

pub(crate) fn parse(json: &str) -> anyhow::Result<Vec<Package>> {
    let metadata: Metadata = serde_json::from_str(json).context("unreadable cargo metadata")?;
    Ok(metadata.packages)
}

#[cfg(test)]
pub(crate) mod fixtures {
    use super::{Dependency, Package, Target};
    use std::path::PathBuf;

    /// A package named `name` that depends (normally) on the workspace crates in `deps`.
    pub(crate) fn package(name: &str, deps: &[&str]) -> Package {
        Package {
            name: name.to_owned(),
            manifest_path: PathBuf::from(format!("crates/{name}/Cargo.toml")),
            dependencies: deps
                .iter()
                .map(|dep| Dependency {
                    name: (*dep).to_owned(),
                    path: Some(PathBuf::from(format!("crates/{dep}"))),
                })
                .collect(),
            targets: vec![Target {
                kind: vec!["lib".to_owned()],
                src_path: PathBuf::from(format!("crates/{name}/src/lib.rs")),
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn this_workspace_is_readable() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let packages = load(root).unwrap();
        assert!(packages.iter().any(|package| package.name == "xtask"));
        assert!(packages.iter().any(|package| package.name == "bb-core"));
    }
}
