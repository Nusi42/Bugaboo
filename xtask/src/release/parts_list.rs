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
use serde_json::Value;
use sha2::{Digest as _, Sha256};

use super::hex;
use crate::checks::dep_licenses::declared_license;
use crate::checks::dep_register::{Rows, read_register};
use crate::wasm;

/// The systems of the first release: Windows x64 and Linux x64, static (ADR-0047, decision 7).
pub(crate) const RELEASE_TARGETS: [&str; 2] =
    ["x86_64-pc-windows-msvc", "x86_64-unknown-linux-musl"];
/// The crate that is the program.
const PROGRAM_CRATE: &str = "bb-server";
/// The browser app, as its folder is named in `package-lock.json`.
const WEB_APP_KEY: &str = "web/app";
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
/// so crates for build scripts and for tests are not in it.
pub(crate) fn rust_closure(metadata: &str, roots: &[&str]) -> anyhow::Result<Closure> {
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
            if ours {
                closure.direct.insert(key.clone());
            }
            closure.crates.entry(key).or_insert_with(|| Crate {
                name: child.name.clone(),
                version: child.version.clone(),
                license: child.license.clone().map(|text| normalize_license(&text)),
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

/// A Cargo license written with a slash (`MIT/Apache-2.0`) as the SPDX expression it means.
pub(crate) fn normalize_license(text: &str) -> String {
    text.split('/')
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" OR ")
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

/// The crates in each program of the release and in the WebAssembly modules of the browser app.
fn compiled_crates(root: &Path) -> anyhow::Result<Closure> {
    let modules: Vec<&str> = wasm::MODULES
        .iter()
        .map(|module| module.crate_name)
        .collect();
    let mut all = Closure::default();
    let walks: Vec<(&str, Vec<&str>)> = RELEASE_TARGETS
        .iter()
        .map(|target| (*target, vec![PROGRAM_CRATE]))
        .chain([(wasm::TARGET, modules)])
        .collect();
    for (target, roots) in walks {
        let part = rust_closure(&cargo_metadata(root, target)?, &roots)?;
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

// ---- npm packages -----------------------------------------------------------------------------

/// The npm packages that the browser app bundles, from the parsed `package-lock.json`: what the
/// app and its own packages depend on (not what only their tools depend on), found the way node
/// finds them, each with the checksum of the lockfile. `installed` gives the text of the
/// `package.json` that is installed at a key, for a package whose lock entry names no license.
pub(crate) fn npm_closure(
    lock: &Value,
    installed: &dyn Fn(&str) -> Option<Value>,
) -> anyhow::Result<(Vec<Part>, BTreeSet<String>)> {
    let packages = lock
        .get("packages")
        .and_then(Value::as_object)
        .context("package-lock.json has no packages")?;
    let mut found: BTreeMap<String, Part> = BTreeMap::new();
    let mut direct = BTreeSet::new();
    let mut seen = BTreeSet::new();
    let mut waiting = vec![WEB_APP_KEY.to_owned()];
    // Keys of the parts, so that their dependencies can be turned into references afterwards.
    let mut edges: Vec<(String, String)> = Vec::new();
    while let Some(key) = waiting.pop() {
        if !seen.insert(key.clone()) {
            continue;
        }
        let Some(entry) = packages.get(&key) else {
            bail!("package-lock.json has no entry {key}");
        };
        let ours = !key.contains("node_modules/");
        for name in dependency_names(entry) {
            let Some(child) = resolve(packages, &key, &name) else {
                continue; // an optional package that is not installed on this system
            };
            let child_key = child;
            if packages[&child_key].get("link").and_then(Value::as_bool) == Some(true) {
                // One of our own packages: its dependencies are the program's.
                let target = packages[&child_key]
                    .get("resolved")
                    .and_then(Value::as_str)
                    .context("a link without a target")?;
                waiting.push(target.to_owned());
                continue;
            }
            if ours {
                direct.insert(child_key.clone());
            } else {
                edges.push((key.clone(), child_key.clone()));
            }
            found
                .entry(child_key.clone())
                .or_insert_with(|| npm_part(&child_key, &packages[&child_key], installed));
            waiting.push(child_key);
        }
    }
    let reference = |key: &str| found[key].reference();
    let direct_refs: BTreeSet<String> = direct.iter().map(|key| reference(key)).collect();
    let mut parts: Vec<Part> = found.values().cloned().collect();
    for (from, to) in &edges {
        if let Some(part) = parts
            .iter_mut()
            .find(|part| part.reference() == reference(from))
        {
            part.depends_on.insert(reference(to));
        }
    }
    Ok((parts, direct_refs))
}

/// The names a package asks for: its dependencies, optional ones, and the peers it does not mark
/// optional.
fn dependency_names(entry: &Value) -> BTreeSet<String> {
    let names = |section: &str| -> BTreeSet<String> {
        entry
            .get(section)
            .and_then(Value::as_object)
            .map(|map| map.keys().cloned().collect())
            .unwrap_or_default()
    };
    let mut all = names("dependencies");
    all.extend(names("optionalDependencies"));
    let optional_peers: BTreeSet<String> = entry
        .get("peerDependenciesMeta")
        .and_then(Value::as_object)
        .map(|map| {
            map.iter()
                .filter(|(_, meta)| meta.get("optional").and_then(Value::as_bool) == Some(true))
                .map(|(name, _)| name.clone())
                .collect()
        })
        .unwrap_or_default();
    all.extend(
        names("peerDependencies")
            .difference(&optional_peers)
            .cloned(),
    );
    all
}

/// The key of the package `name` that the package at `from` gets: the nearest `node_modules`
/// folder that has it, going up (the way node looks).
fn resolve(packages: &serde_json::Map<String, Value>, from: &str, name: &str) -> Option<String> {
    let mut base = from.to_owned();
    loop {
        let candidate = if base.is_empty() {
            format!("node_modules/{name}")
        } else {
            format!("{base}/node_modules/{name}")
        };
        if packages.contains_key(&candidate) {
            return Some(candidate);
        }
        if base.is_empty() {
            return None;
        }
        base = match base.rfind("node_modules/") {
            Some(at) => base[..at].trim_end_matches('/').to_owned(),
            None => String::new(),
        };
    }
}

fn npm_part(key: &str, entry: &Value, installed: &dyn Fn(&str) -> Option<Value>) -> Part {
    let name = key.rsplit("node_modules/").next().unwrap_or(key).to_owned();
    let license = declared_license(entry)
        .or_else(|| installed(key).as_ref().and_then(declared_license))
        .map(|text| text.trim().to_owned());
    Part {
        kind: Kind::Npm,
        name,
        version: entry
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        license,
        checksum: entry
            .get("integrity")
            .and_then(Value::as_str)
            .and_then(integrity_checksum),
        purpose: None,
        depends_on: BTreeSet::new(),
        folder: Some(PathBuf::from(key)),
    }
}

/// An `integrity` value of the lockfile (`sha512-<base64>`) as a checksum.
fn integrity_checksum(integrity: &str) -> Option<Checksum> {
    let (algorithm, encoded) = integrity.split_whitespace().next()?.split_once('-')?;
    let algorithm = match algorithm {
        "sha512" => Algorithm::Sha512,
        "sha256" => Algorithm::Sha256,
        _ => return None,
    };
    Some(Checksum {
        algorithm,
        hex: hex(&base64_decode(encoded)?),
    })
}

/// Standard Base64, with or without padding.
fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut bits = 0_u32;
    let mut count = 0;
    for character in text.trim_end_matches('=').bytes() {
        let value = match character {
            b'A'..=b'Z' => character - b'A',
            b'a'..=b'z' => character - b'a' + 26,
            b'0'..=b'9' => character - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        bits = (bits << 6) | u32::from(value);
        count += 6;
        if count >= 8 {
            count -= 8;
            bytes.push(u8::try_from((bits >> count) & 0xff).ok()?);
        }
    }
    Some(bytes)
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
        let ships = row.cells.get(3).is_some_and(|cell| cell.starts_with("yes"));
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
    let lock: Value = serde_json::from_str(
        &fs::read_to_string(root.join("package-lock.json"))
            .context("could not read package-lock.json")?,
    )
    .context("package-lock.json is not valid JSON")?;
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

#[cfg(test)]
mod tests;
