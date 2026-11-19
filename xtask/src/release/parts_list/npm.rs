//! The npm packages of the parts list: what the browser app bundles, from `package-lock.json`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, bail};
use serde_json::Value;

use super::{Algorithm, Checksum, Kind, Part};
use crate::checks::dep_licenses::declared_license;
use crate::steps::WEB_APP;

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
    let mut waiting = vec![WEB_APP.to_owned()];
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

/// The lockfile `package-lock.json` of the repository, read.
pub(crate) fn read_lock(root: &Path) -> anyhow::Result<Value> {
    serde_json::from_str(
        &fs::read_to_string(root.join("package-lock.json"))
            .context("could not read package-lock.json")?,
    )
    .context("package-lock.json is not valid JSON")
}

/// The names the packages of our own (the workspaces of the lockfile) ask for, except each other:
/// what the browser app depends on directly, read from the lockfile without walking it.
pub(crate) fn workspace_dependencies(lock: &Value) -> BTreeSet<String> {
    let packages = lock.get("packages").and_then(Value::as_object);
    let is_link = |name: &str| {
        packages
            .and_then(|map| map.get(&format!("node_modules/{name}")))
            .and_then(|entry| entry.get("link"))
            .and_then(Value::as_bool)
            == Some(true)
    };
    packages
        .into_iter()
        .flatten()
        .filter(|(key, _)| !key.is_empty() && !key.contains("node_modules/"))
        .filter_map(|(_, entry)| entry.get("dependencies").and_then(Value::as_object))
        .flat_map(|map| map.keys())
        .filter(|name| !is_link(name))
        .cloned()
        .collect()
}

/// The packages the lockfile marks as installed for the running program: every entry in
/// `node_modules` that is not only for development (`dev`, `devOptional`) and not one of our own
/// packages (a link), as `(name, version)`. A second way to the set [`npm_closure`] finds by
/// walking the dependencies, so each holds the other to account.
pub(crate) fn locked_not_dev(lock: &Value) -> BTreeSet<(String, String)> {
    lock.get("packages")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter(|(key, entry)| {
            key.contains("node_modules/")
                && ["dev", "devOptional", "link"]
                    .iter()
                    .all(|flag| entry.get(flag).and_then(Value::as_bool) != Some(true))
        })
        .map(|(key, entry)| {
            (
                key.rsplit("node_modules/").next().unwrap_or(key).to_owned(),
                entry
                    .get("version")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
            )
        })
        .collect()
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

pub(super) fn npm_part(
    key: &str,
    entry: &Value,
    installed: &dyn Fn(&str) -> Option<Value>,
) -> Part {
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
        authors: Vec::new(),
        depends_on: BTreeSet::new(),
        folder: Some(PathBuf::from(key)),
    }
}

/// An `integrity` value of the lockfile (`sha512-<base64>`) as a checksum.
pub(super) fn integrity_checksum(integrity: &str) -> Option<Checksum> {
    let (algorithm, encoded) = integrity.split_whitespace().next()?.split_once('-')?;
    let algorithm = match algorithm {
        "sha512" => Algorithm::Sha512,
        "sha256" => Algorithm::Sha256,
        _ => return None,
    };
    Some(Checksum {
        algorithm,
        hex: bb_core::hex::encode(&base64_decode(encoded)?),
    })
}

/// Standard Base64, with or without padding.
pub(super) fn base64_decode(text: &str) -> Option<Vec<u8>> {
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
