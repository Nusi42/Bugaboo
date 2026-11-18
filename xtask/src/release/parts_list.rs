//! The parts list of the program (COMP-01, ADR-0047, decision 3): every crate compiled into each
//! program of a release, every npm package bundled into the browser app, and the files of
//! third-party data and notices that ship, each with name, version, license and checksum.
//!
//! The sources are `cargo metadata` for the release targets (normal dependencies only: no build
//! and no test crates), `Cargo.lock` for the checksums of the crates, `package-lock.json` for the
//! npm packages and their checksums, and the register for the purpose of what is listed there.
//! The lockfiles are the one source; nothing here is kept by hand.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, bail};
use serde::Deserialize;
use sha2::{Digest as _, Sha256};

use super::hex;
use crate::checks::dep_licenses::with_or_for_slash;
use crate::checks::dep_register::{Rows, read_register};
use crate::wasm;

/// The systems of the first release: Windows x64 and Linux x64, static (ADR-0047, decision 7).
pub(crate) const RELEASE_TARGETS: [&str; 2] =
    ["x86_64-pc-windows-msvc", "x86_64-unknown-linux-musl"];
/// The crate that is the program.
const PROGRAM_CRATE: &str = "bb-server";
/// The notices of code and data inside crates (COMP-02).
pub(crate) const NOTICES_FOLDER: &str = "docs/third-party/notices";
/// The line of a notice file that names its license.
const NOTICE_LICENSE_LINE: &str = "SPDX-License-Identifier:";

/// What a part is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Kind {
    Crate,
    Npm,
    /// A file of third-party data or a notice.
    File,
}

/// How a checksum was made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Algorithm {
    Sha256,
    Sha512,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Checksum {
    pub(crate) algorithm: Algorithm,
    /// Lower-case hexadecimal.
    pub(crate) hex: String,
}

/// One part of the program.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Part {
    pub(crate) kind: Kind,
    /// The name of the crate or package, or the path of the file.
    pub(crate) name: String,
    /// Empty for a file.
    pub(crate) version: String,
    /// An SPDX license expression; `None` when the package declares none.
    pub(crate) license: Option<String>,
    pub(crate) checksum: Option<Checksum>,
    /// What the register says it is for (direct dependencies only).
    pub(crate) purpose: Option<String>,
    /// Who wrote it, as the package says (crates only): the copyright line of a standard license
    /// text.
    pub(crate) authors: Vec<String>,
    /// The references of the parts it depends on.
    pub(crate) depends_on: BTreeSet<String>,
    /// Where its license files are, relative to the repository root or absolute.
    pub(crate) folder: Option<PathBuf>,
}

impl Part {
    /// The reference that names the part inside the files: its package URL, or `file:<path>`.
    pub(crate) fn reference(&self) -> String {
        match self.kind {
            Kind::Crate => format!("pkg:cargo/{}@{}", self.name, self.version),
            Kind::Npm => format!("pkg:npm/{}@{}", self.name.replace('@', "%40"), self.version),
            Kind::File => format!("file:{}", self.name),
        }
    }

    /// The package URL, for the parts that have one.
    pub(crate) fn purl(&self) -> Option<String> {
        (self.kind != Kind::File).then(|| self.reference())
    }
}

/// The program itself: the top component of the files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Program {
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) license: String,
}

impl Program {
    pub(crate) fn reference(&self) -> String {
        format!("pkg:generic/{}@{}", self.name, self.version)
    }
}

/// The parts list of one commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PartsList {
    pub(crate) program: Program,
    /// When the files were made: the time of the commit, so two runs write the same bytes.
    pub(crate) created: String,
    /// Sorted by name, version and kind.
    pub(crate) parts: Vec<Part>,
    /// The references of the parts the program depends on directly.
    pub(crate) direct: BTreeSet<String>,
}

// ---- Rust crates ------------------------------------------------------------------------------

#[derive(Deserialize)]
struct MetaPackage {
    id: String,
    name: String,
    version: String,
    license: Option<String>,
    #[serde(default)]
    authors: Vec<String>,
    source: Option<String>,
    manifest_path: PathBuf,
}

#[derive(Deserialize)]
struct DepKind {
    kind: Option<String>,
}

#[derive(Deserialize)]
struct MetaLink {
    pkg: String,
    #[serde(default)]
    dep_kinds: Vec<DepKind>,
}

#[derive(Deserialize)]
struct MetaNode {
    id: String,
    #[serde(default)]
    deps: Vec<MetaLink>,
}

#[derive(Deserialize)]
struct MetaResolve {
    nodes: Vec<MetaNode>,
}

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<MetaPackage>,
    resolve: MetaResolve,
}

/// A crate of the closure, before the checksum and the purpose are added.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Crate {
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) license: Option<String>,
    pub(crate) authors: Vec<String>,
    pub(crate) folder: PathBuf,
    /// `(name, version)` of the crates it depends on.
    pub(crate) depends_on: BTreeSet<(String, String)>,
}

/// What a walk over the normal dependencies found.
#[derive(Default, Debug)]
pub(crate) struct Closure {
    /// `(name, version, license)` of the first root that is a program, when asked for.
    pub(crate) program: Option<Program>,
    pub(crate) crates: BTreeMap<(String, String), Crate>,
    /// The crates our own code depends on directly.
    pub(crate) direct: BTreeSet<(String, String)>,
}

/// The crates that the workspace members named in `roots` compile in, from the text of
/// `cargo metadata --format-version 1 --filter-platform <target>`: the normal dependencies only,
/// so crates for build scripts and for tests are not in it. With `only`, a crate that is not in it
/// is left out with everything below it: `cargo metadata` unifies the features of the whole
/// workspace, so it names crates that the program does not compile in (see [`tree_crates`]).
pub(crate) fn rust_closure(
    metadata: &str,
    roots: &[&str],
    only: Option<&BTreeSet<(String, String)>>,
) -> anyhow::Result<Closure> {
    let metadata: Metadata = serde_json::from_str(metadata).context("unreadable cargo metadata")?;
    let packages: BTreeMap<&str, &MetaPackage> = metadata
        .packages
        .iter()
        .map(|package| (package.id.as_str(), package))
        .collect();
    let nodes: BTreeMap<&str, &MetaNode> = metadata
        .resolve
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();
    let mut closure = Closure::default();
    let mut seen = BTreeSet::new();
    let mut waiting: Vec<&str> = Vec::new();
    for root in roots {
        let package = metadata
            .packages
            .iter()
            .find(|package| package.name == *root && package.source.is_none())
            .with_context(|| format!("the workspace has no crate {root}"))?;
        if closure.program.is_none() {
            closure.program = Some(Program {
                name: package.name.clone(),
                version: package.version.clone(),
                license: package.license.clone().unwrap_or_default(),
            });
        }
        waiting.push(package.id.as_str());
    }
    while let Some(id) = waiting.pop() {
        if !seen.insert(id) {
            continue;
        }
        let (Some(package), Some(node)) = (packages.get(id), nodes.get(id)) else {
            continue;
        };
        let ours = package.source.is_none();
        for link in &node.deps {
            if !link.dep_kinds.iter().any(|kind| kind.kind.is_none()) {
                continue; // a build or test dependency only
            }
            let Some(child) = packages.get(link.pkg.as_str()) else {
                continue;
            };
            if child.source.is_none() {
                waiting.push(child.id.as_str());
                continue;
            }
            let key = (child.name.clone(), child.version.clone());
            if only.is_some_and(|only| !only.contains(&key)) {
                continue;
            }
            if ours {
                closure.direct.insert(key.clone());
            }
            closure.crates.entry(key).or_insert_with(|| Crate {
                name: child.name.clone(),
                version: child.version.clone(),
                license: child.license.clone().map(|text| with_or_for_slash(&text)),
                authors: child.authors.clone(),
                folder: child
                    .manifest_path
                    .parent()
                    .map_or_else(PathBuf::new, Path::to_path_buf),
                depends_on: BTreeSet::new(),
            });
            if !ours {
                let parent = (package.name.clone(), package.version.clone());
                if let Some(parent) = closure.crates.get_mut(&parent) {
                    parent
                        .depends_on
                        .insert((child.name.clone(), child.version.clone()));
                }
            }
            waiting.push(child.id.as_str());
        }
    }
    Ok(closure)
}

/// `(name, version)` -> the SHA-256 that `Cargo.lock` records for a crate of a registry.
pub(crate) fn lock_checksums(lock: &str) -> anyhow::Result<BTreeMap<(String, String), String>> {
    let value: toml::Value = toml::from_str(lock).context("Cargo.lock is not valid TOML")?;
    let mut sums = BTreeMap::new();
    for package in value
        .get("package")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
    {
        let text = |key: &str| package.get(key).and_then(toml::Value::as_str);
        if let (Some(name), Some(version), Some(sum)) =
            (text("name"), text("version"), text("checksum"))
        {
            sums.insert((name.to_owned(), version.to_owned()), sum.to_owned());
        }
    }
    Ok(sums)
}

/// `cargo metadata` for `target`, filtered to what that system compiles.
fn cargo_metadata(root: &Path, target: &str) -> anyhow::Result<String> {
    let out = crate::tools::command("cargo")
        .args([
            "metadata",
            "--locked",
            "--format-version",
            "1",
            "--filter-platform",
            target,
        ])
        .current_dir(root)
        .output()
        .context("could not run cargo; is Rust installed?")?;
    if !out.status.success() {
        bail!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Each program of the release as `(system, crate)`, and each WebAssembly module of the browser
/// app: what the parts list covers.
pub(crate) fn programs() -> Vec<(&'static str, &'static str)> {
    RELEASE_TARGETS
        .iter()
        .map(|target| (*target, PROGRAM_CRATE))
        .chain(
            wasm::MODULES
                .iter()
                .map(|module| (wasm::TARGET, module.crate_name)),
        )
        .collect()
}

/// The crates that `cargo tree -e normal` says `package` compiles in for `target`, as
/// `(name, version)`: cargo's own answer for one program with its own features, the one the parts
/// list is held to. Our own crates are not in it.
pub(crate) fn tree_crates(
    root: &Path,
    target: &str,
    package: &str,
) -> anyhow::Result<BTreeSet<(String, String)>> {
    let out = crate::tools::command("cargo")
        .args([
            "tree", "--locked", "-p", package, "--target", target, "-e", "normal", "--prefix",
            "none", "-f", "{p}",
        ])
        .current_dir(root)
        .output()
        .context("could not run cargo; is Rust installed?")?;
    if !out.status.success() {
        bail!(
            "cargo tree failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(parse_tree(&String::from_utf8_lossy(&out.stdout)))
}

/// The crates in the text of `cargo tree --prefix none -f "{p}"`, as `(name, version)`.
pub(crate) fn parse_tree(text: &str) -> BTreeSet<(String, String)> {
    let mut crates = BTreeSet::new();
    for line in text.lines() {
        let words: Vec<&str> = line.split_whitespace().collect();
        let [name, version, rest @ ..] = words.as_slice() else {
            continue;
        };
        // A crate of ours has its folder in brackets; a procedural macro says so in brackets too.
        if rest
            .iter()
            .any(|word| word.starts_with('(') && *word != "(proc-macro)" && *word != "(*)")
        {
            continue;
        }
        crates.insert((
            (*name).to_owned(),
            version.trim_start_matches('v').to_owned(),
        ));
    }
    crates
}

/// Every crate of every program of [`programs`], by `cargo tree`.
pub(crate) fn tree_union(root: &Path) -> anyhow::Result<BTreeSet<(String, String)>> {
    let mut all = BTreeSet::new();
    for (target, package) in programs() {
        all.extend(tree_crates(root, target, package)?);
    }
    Ok(all)
}

/// The crates in each program of the release and in the WebAssembly modules of the browser app.
fn compiled_crates(root: &Path) -> anyhow::Result<Closure> {
    let mut all = Closure::default();
    let mut metadata: BTreeMap<&str, String> = BTreeMap::new();
    for (target, package) in programs() {
        if !metadata.contains_key(target) {
            metadata.insert(target, cargo_metadata(root, target)?);
        }
        let only = tree_crates(root, target, package)?;
        let part = rust_closure(&metadata[target], &[package], Some(&only))?;
        if all.program.is_none() {
            all.program = part.program;
        }
        all.direct.extend(part.direct);
        for (key, found) in part.crates {
            all.crates
                .entry(key)
                .and_modify(|have| have.depends_on.extend(found.depends_on.clone()))
                .or_insert(found);
        }
    }
    Ok(all)
}

// ---- Files that ship --------------------------------------------------------------------------

fn sha256_of(bytes: &[u8]) -> Checksum {
    Checksum {
        algorithm: Algorithm::Sha256,
        hex: hex(&Sha256::digest(bytes)),
    }
}

/// The files of third-party data the register lists as shipping, and the notice files.
fn file_parts(root: &Path, register: &Rows) -> anyhow::Result<Vec<Part>> {
    let mut parts = Vec::new();
    for (path, row) in &register.data {
        let ships = row.ships.starts_with("yes");
        if !ships {
            continue;
        }
        let bytes = fs::read(root.join(path)).with_context(|| {
            format!("the register lists the data file {path}, which is not there")
        })?;
        parts.push(Part {
            kind: Kind::File,
            name: path.clone(),
            version: String::new(),
            license: Some(leading_license(&row.license)),
            checksum: Some(sha256_of(&bytes)),
            purpose: Some(row.purpose.clone()).filter(|text| !text.is_empty()),
            authors: Vec::new(),
            depends_on: BTreeSet::new(),
            folder: None,
        });
    }
    for path in notice_files(root)? {
        let text = fs::read_to_string(root.join(&path))
            .with_context(|| format!("could not read {path}"))?;
        parts.push(Part {
            kind: Kind::File,
            name: path.clone(),
            version: String::new(),
            license: notice_license(&text),
            checksum: Some(sha256_of(text.as_bytes())),
            purpose: text.lines().next().map(str::to_owned),
            authors: Vec::new(),
            depends_on: BTreeSet::new(),
            folder: None,
        });
    }
    Ok(parts)
}

/// The license of a register row that says more than the expression (`MIT (SecLists, …)`).
fn leading_license(cell: &str) -> String {
    cell.split(" (").next().unwrap_or(cell).trim().to_owned()
}

/// The license a notice file names in its header.
pub(crate) fn notice_license(text: &str) -> Option<String> {
    text.lines()
        .take(8)
        .find_map(|line| line.strip_prefix(NOTICE_LICENSE_LINE))
        .map(|license| license.trim().to_owned())
}

/// The notice files, as paths relative to the repository root, in order.
pub(crate) fn notice_files(root: &Path) -> anyhow::Result<Vec<String>> {
    let mut names = Vec::new();
    let Ok(entries) = fs::read_dir(root.join(NOTICES_FOLDER)) else {
        return Ok(names);
    };
    for entry in entries.flatten() {
        if entry.path().is_file() {
            names.push(format!(
                "{NOTICES_FOLDER}/{}",
                entry.file_name().to_string_lossy()
            ));
        }
    }
    names.sort();
    Ok(names)
}

// ---- The list ---------------------------------------------------------------------------------

/// Makes the parts list of the repository at `root`. `created` is the time written into the
/// files (see [`PartsList::created`]).
///
/// # Errors
/// When a lockfile, `cargo metadata` or the register cannot be read.
pub(crate) fn collect(root: &Path, created: &str) -> anyhow::Result<PartsList> {
    let register = read_register(root)?;
    let compiled = compiled_crates(root)?;
    // The product is named after its program, `bugaboo`; the crate that builds it is bb-server.
    let program = Program {
        name: crate::steps::SERVER_BINARY.to_owned(),
        ..compiled
            .program
            .clone()
            .context("the program crate was not found")?
    };
    let sums = lock_checksums(
        &fs::read_to_string(root.join("Cargo.lock")).context("could not read Cargo.lock")?,
    )?;
    let mut parts: Vec<Part> = Vec::new();
    let reference = |name: &str, version: &str| format!("pkg:cargo/{name}@{version}");
    for found in compiled.crates.values() {
        let key = (found.name.clone(), found.version.clone());
        parts.push(Part {
            kind: Kind::Crate,
            name: found.name.clone(),
            version: found.version.clone(),
            license: found.license.clone(),
            checksum: sums.get(&key).map(|sum| Checksum {
                algorithm: Algorithm::Sha256,
                hex: sum.clone(),
            }),
            purpose: register
                .rust
                .get(&found.name)
                .map(|row| row.purpose.clone())
                .filter(|text| !text.is_empty()),
            authors: found.authors.clone(),
            depends_on: found
                .depends_on
                .iter()
                .map(|(name, version)| reference(name, version))
                .collect(),
            folder: Some(found.folder.clone()),
        });
    }
    let mut direct: BTreeSet<String> = compiled
        .direct
        .iter()
        .map(|(name, version)| reference(name, version))
        .collect();
    let lock = read_lock(root)?;
    let (mut npm, npm_direct) = npm_closure(&lock, &|key| {
        fs::read_to_string(root.join(key).join("package.json"))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
    })?;
    for part in &mut npm {
        part.purpose = register
            .npm
            .get(&part.name)
            .map(|row| row.purpose.clone())
            .filter(|text| !text.is_empty());
        part.folder = part.folder.take().map(|key| root.join(key));
    }
    direct.extend(npm_direct);
    parts.extend(npm);
    let files = file_parts(root, &register)?;
    direct.extend(files.iter().map(Part::reference));
    parts.extend(files);
    parts.sort_by(|a, b| (&a.name, &a.version, a.kind).cmp(&(&b.name, &b.version, b.kind)));
    Ok(PartsList {
        program,
        created: created.to_owned(),
        parts,
        direct,
    })
}

/// The time of the last commit, as the files write it: UTC, to the second. Without Git history
/// (a throw-away folder) it is the start of 1970.
pub(crate) fn commit_time(root: &Path) -> String {
    let seconds = crate::tools::command("git")
        .args(["show", "-s", "--format=%ct", "HEAD"])
        .current_dir(root)
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| {
            String::from_utf8_lossy(&out.stdout)
                .trim()
                .parse::<i64>()
                .ok()
        })
        .unwrap_or(0);
    jiff::Timestamp::from_second(seconds).map_or_else(
        |_| "1970-01-01T00:00:00Z".to_owned(),
        |time| time.to_string(),
    )
}

mod npm;
#[cfg(test)]
use npm::{base64_decode, integrity_checksum};
pub(crate) use npm::{locked_not_dev, npm_closure, read_lock, workspace_dependencies};
#[cfg(test)]
use serde_json::Value;

#[cfg(test)]
mod tests;
