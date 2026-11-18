//! DEP-SBOM: the parts list is made on every full run, names every crate and package that ships,
//! gives each a license and a checksum, has no entry without a package, and both files fit their
//! schemas (COMP-01, ADR-0047 decision 3, architecture §7 *Compliance artifacts*).
//!
//! The list is made from the lockfiles by `crate::release`, written to the build folder, and
//! read back from the files, so a part that the writer drops, or an entry it invents, is found.
//! The signatures of the release folder are a later step of `cargo xtask release`.

use std::collections::BTreeSet;
use std::fs;

use anyhow::Context as _;
use serde_json::Value;

use super::{Check, Context, Finding};
use crate::checks::dep_register::{Rows, read_register};
use crate::release::parts_list::{Kind, PartsList};
use crate::release::{self, formats};

pub(crate) struct DepSbom;

impl Check for DepSbom {
    fn id(&self) -> &'static str {
        "DEP-SBOM"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let root = context.repo.root();
        let files = release::compose(root)?;
        let folder = release::work_folder(root)?;
        release::write(&files, &folder)?;
        let version = &files.list.program.version;
        let read = |name: String| -> anyhow::Result<Value> {
            let path = folder.join(&name);
            serde_json::from_str(&fs::read_to_string(&path)?)
                .with_context(|| format!("could not read back {}", path.display()))
        };
        let cyclonedx = read(formats::cyclonedx_name(version))?;
        let spdx = read(formats::spdx_name(version))?;
        let vex = read(formats::vex_name(version))?;
        let mut findings = unfinished_parts(&files.list);
        findings.extend(compare(&files.list, &cyclonedx, &spdx));
        let written = release::Files {
            list: files.list.clone(),
            cyclonedx,
            spdx,
            vex,
            notices: release::notices::Notices {
                text: String::new(),
                without_text: Vec::new(),
            },
        };
        findings.extend(
            release::schema_errors(&written)?
                .into_iter()
                .map(|error| Finding::general(format!("does not fit its schema: {error}"))),
        );
        let all = context.repo.all_files()?;
        let register = read_register(root)?;
        findings.extend(shipped_without_entry(root, &all, &register, &files.list)?);
        Ok(findings)
    }
}

/// Parts that have no license or no checksum.
fn unfinished_parts(list: &PartsList) -> Vec<Finding> {
    let mut findings = Vec::new();
    for part in &list.parts {
        let label = format!("{} {}", part.name, part.version);
        if part.license.as_deref().is_none_or(str::is_empty) {
            findings.push(Finding::at(
                &label,
                "declares no license. Pick another package or get the license clarified.",
            ));
        } else if let Some(license) = &part.license
            && !crate::checks::dep_licenses::satisfied_by(license, &|_| true)
        {
            findings.push(Finding::at(
                &label,
                format!("has the license `{license}`, which is not an SPDX expression."),
            ));
        }
        if part.checksum.is_none() {
            findings.push(Finding::at(
                &label,
                "has no checksum in the lockfile, so the parts list cannot give one.",
            ));
        }
    }
    findings
}

/// What a file of the parts list names: `(name, version)` of each part.
type Names = BTreeSet<(String, String)>;

/// The names that are in `expected` and not in `actual`, and the other way round.
fn difference(expected: &Names, actual: &Names) -> (Vec<String>, Vec<String>) {
    let text = |set: &Names, other: &Names| {
        set.difference(other)
            .map(|(name, version)| format!("{name} {version}").trim().to_owned())
            .collect()
    };
    (text(expected, actual), text(actual, expected))
}

fn findings_of(file: &str, expected: &Names, actual: &Names) -> Vec<Finding> {
    let (missing, extra) = difference(expected, actual);
    let mut findings = Vec::new();
    for name in missing {
        findings.push(Finding::at(
            file,
            format!("{name} ships and is missing from the parts list."),
        ));
    }
    for name in extra {
        findings.push(Finding::at(
            file,
            format!("{name} is an entry of the parts list and names no package of the lockfiles."),
        ));
    }
    findings
}

fn cyclonedx_names(document: &Value) -> Names {
    let text = |value: &Value, key: &str| {
        value
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned()
    };
    document
        .get("components")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|component| (text(component, "name"), text(component, "version")))
        .collect()
}

fn spdx_names(document: &Value) -> Names {
    let text = |value: &Value, key: &str| {
        value
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned()
    };
    document
        .get("packages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        // The first package is the program itself.
        .skip(1)
        .map(|package| (text(package, "name"), text(package, "versionInfo")))
        .collect()
}

/// The parts the lockfiles give against what the two written files name.
fn compare(list: &PartsList, cyclonedx: &Value, spdx: &Value) -> Vec<Finding> {
    let expected: Names = list
        .parts
        .iter()
        .map(|part| (part.name.clone(), part.version.clone()))
        .collect();
    let mut findings = findings_of("CycloneDX file", &expected, &cyclonedx_names(cyclonedx));
    findings.extend(findings_of("SPDX file", &expected, &spdx_names(spdx)));
    findings
}

/// Font files and icon sets in the sources or the build of the browser app that the register and
/// the parts list do not name. Bugaboo bundles no font files (it uses the system font stacks,
/// ADR-0026); a font that shipped would need its entry, with its license.
fn shipped_without_entry(
    root: &std::path::Path,
    files: &[String],
    register: &Rows,
    list: &PartsList,
) -> anyhow::Result<Vec<Finding>> {
    let mut all = files.to_vec();
    let dist = root.join("web/app/dist");
    if dist.is_dir() {
        all.extend(files_below(&dist, "web/app/dist"));
    }
    let mut findings: Vec<Finding> = unlisted_fonts(&all, register)
        .into_iter()
        .map(|font| {
            Finding::at(
                font,
                "is a font file with no row in the register (Bundled data) and so no entry in the parts list.",
            )
        })
        .collect();
    let mut wanted = BTreeSet::new();
    for manifest in ["web/app/package.json", "web/packages/ui/package.json"] {
        let Ok(text) = fs::read_to_string(root.join(manifest)) else {
            continue;
        };
        let json: Value = serde_json::from_str(&text).context("package.json is not valid JSON")?;
        wanted.extend(
            json.get("dependencies")
                .and_then(Value::as_object)
                .into_iter()
                .flat_map(|map| map.keys().cloned()),
        );
    }
    findings.extend(unlisted_icon_sets(&wanted, list).into_iter().map(|name| {
        Finding::at(
            name,
            "is an icon set or font package of the browser app and has no entry in the parts list.",
        )
    }));
    Ok(findings)
}

fn files_below(folder: &std::path::Path, prefix: &str) -> Vec<String> {
    let mut found = Vec::new();
    for entry in fs::read_dir(folder).into_iter().flatten().flatten() {
        let name = format!("{prefix}/{}", entry.file_name().to_string_lossy());
        if entry.path().is_dir() {
            found.extend(files_below(&entry.path(), &name));
        } else {
            found.push(name);
        }
    }
    found
}

/// Font files that are not listed in the register as bundled data.
fn unlisted_fonts(files: &[String], register: &Rows) -> Vec<String> {
    const FONT_EXTENSIONS: [&str; 5] = ["woff", "woff2", "ttf", "otf", "eot"];
    files
        .iter()
        .filter(|file| {
            file.rsplit_once('.').is_some_and(|(_, extension)| {
                FONT_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
            })
        })
        .filter(|file| !register.data.contains_key(file.as_str()))
        .cloned()
        .collect()
}

/// Icon sets and font packages among the packages the browser app asks for directly that the
/// parts list does not name.
fn unlisted_icon_sets(wanted: &BTreeSet<String>, list: &PartsList) -> Vec<String> {
    const MARKS: [&str; 8] = [
        "icons",
        "lucide",
        "fontawesome",
        "fontsource",
        "phosphor",
        "tabler",
        "feather",
        "typeface",
    ];
    let listed: BTreeSet<&str> = list
        .parts
        .iter()
        .filter(|part| part.kind == Kind::Npm)
        .map(|part| part.name.as_str())
        .collect();
    wanted
        .iter()
        .filter(|name| MARKS.iter().any(|mark| name.contains(mark)))
        .filter(|name| !listed.contains(name.as_str()))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests;
