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
    /// `None` for a normal dependency, else `dev` or `build`.
    #[serde(default)]
    pub(crate) kind: Option<String>,
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

/// One package of the full `cargo metadata` output.
#[derive(Deserialize)]
struct Resolved {
    id: String,
    name: String,
    version: String,
    /// `None` for packages that are folders in this repository.
    source: Option<String>,
}

#[derive(Deserialize)]
struct Link {
    pkg: String,
}

#[derive(Deserialize)]
struct Node {
    id: String,
    deps: Vec<Link>,
}

#[derive(Deserialize)]
struct Graph {
    nodes: Vec<Node>,
}

#[derive(Deserialize)]
struct FullMetadata {
    packages: Vec<Resolved>,
    workspace_members: Vec<String>,
    resolve: Graph,
}

/// Name -> the versions that workspace members really depend on directly (the locked packages).
pub(crate) type DirectVersions =
    std::collections::BTreeMap<String, std::collections::BTreeSet<String>>;

/// Runs `cargo metadata` with the resolved graph and returns the locked version of every
/// external package a workspace member lists as a dependency.
pub(crate) fn load_direct_versions(root: &Path) -> anyhow::Result<DirectVersions> {
    let out = Command::new("cargo")
        .args(["metadata", "--locked", "--format-version", "1"])
        .current_dir(root)
        .output()
        .context("could not run cargo; is Rust installed?")?;
    if !out.status.success() {
        bail!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    direct_versions(&String::from_utf8_lossy(&out.stdout))
}

pub(crate) fn direct_versions(json: &str) -> anyhow::Result<DirectVersions> {
    let metadata: FullMetadata = serde_json::from_str(json).context("unreadable cargo metadata")?;
    let mut versions = DirectVersions::new();
    for node in metadata
        .resolve
        .nodes
        .iter()
        .filter(|node| metadata.workspace_members.contains(&node.id))
    {
        for link in &node.deps {
            let package = metadata
                .packages
                .iter()
                .find(|package| package.id == link.pkg && package.source.is_some());
            if let Some(package) = package {
                versions
                    .entry(package.name.clone())
                    .or_default()
                    .insert(package.version.clone());
            }
        }
    }
    Ok(versions)
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
                    kind: None,
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
    fn direct_versions_ignore_transitive_duplicates() {
        let json = r#"{
            "packages": [
              {"id":"app 0.0.0 (path+file:///a)","name":"app","version":"0.0.0","source":null},
              {"id":"x 1.0.0 (registry+r)","name":"x","version":"1.0.0","source":"registry+r"},
              {"id":"x 2.0.0 (registry+r)","name":"x","version":"2.0.0","source":"registry+r"}
            ],
            "workspace_members": ["app 0.0.0 (path+file:///a)"],
            "resolve": {"nodes": [
              {"id":"app 0.0.0 (path+file:///a)","deps":[{"pkg":"x 2.0.0 (registry+r)"}]},
              {"id":"x 2.0.0 (registry+r)","deps":[{"pkg":"x 1.0.0 (registry+r)"}]}
            ]}
        }"#;
        let versions = direct_versions(json).unwrap();
        assert_eq!(versions["x"].iter().collect::<Vec<_>>(), ["2.0.0"]);
        assert!(!versions.contains_key("app"));
    }

    #[test]
    fn this_workspace_is_readable() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let packages = load(root).unwrap();
        assert!(packages.iter().any(|package| package.name == "xtask"));
        assert!(packages.iter().any(|package| package.name == "bb-core"));
    }
}
