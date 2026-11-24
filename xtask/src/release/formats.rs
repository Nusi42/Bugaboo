//! The parts list as `CycloneDX` 1.6 and SPDX 2.3 JSON, and the check of both against their schemas
//! (COMP-01). The schemas are test data, copied from their published versions
//! (`tools/testdata/sbom/`).

use std::collections::BTreeSet;

use bb_api_words::about;
use jsonschema::{Retrieve, Uri, Validator};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

use super::parts_list::{Algorithm, Checksum, Kind, Part, PartsList};

/// The version of the `CycloneDX` format.
const CYCLONEDX_VERSION: &str = "1.6";
/// The version of the SPDX format.
const SPDX_VERSION: &str = "SPDX-2.3";
/// Whose name is written as the maker of the files.
const MAKER: &str = "bugaboo-xtask";

const CYCLONEDX_SCHEMA: &str = include_str!("../../../tools/testdata/sbom/bom-1.6.schema.json");
const CYCLONEDX_LICENSES: &str = include_str!("../../../tools/testdata/sbom/spdx.schema.json");
const CYCLONEDX_SIGNATURES: &str =
    include_str!("../../../tools/testdata/sbom/jsf-0.82.schema.json");
const SPDX_SCHEMA: &str = include_str!("../../../tools/testdata/sbom/spdx-2.3.schema.json");

/// The names of the parts list of a release for a version.
pub(crate) fn cyclonedx_name(version: &str) -> String {
    about::release_name(version, about::CYCLONEDX_END)
}

pub(crate) fn spdx_name(version: &str) -> String {
    about::release_name(version, about::SPDX_END)
}

/// The `serialNumber` of a document made from `digest` (a hash in hexadecimal): a version 4 UUID
/// shape, so that two different lists never share one and two runs on one list do.
pub(crate) fn serial_number(digest: &str) -> String {
    format!(
        "urn:uuid:{}-{}-4{}-a{}-{}",
        &digest[0..8],
        &digest[8..12],
        &digest[13..16],
        &digest[17..20],
        &digest[20..32]
    )
}

/// A name for the document that two different lists never share: the hash of what the parts say.
fn content_hash(list: &PartsList) -> String {
    let mut hasher = Sha256::new();
    hasher.update(list.program.reference());
    for part in &list.parts {
        hasher.update(part.reference());
        hasher.update(part.license.as_deref().unwrap_or(""));
        if let Some(checksum) = &part.checksum {
            hasher.update(&checksum.hex);
        }
    }
    bb_core::hex::encode(&hasher.finalize())
}

fn cyclonedx_algorithm(checksum: &Checksum) -> &'static str {
    match checksum.algorithm {
        Algorithm::Sha256 => "SHA-256",
        Algorithm::Sha512 => "SHA-512",
    }
}

fn spdx_algorithm(checksum: &Checksum) -> &'static str {
    match checksum.algorithm {
        Algorithm::Sha256 => "SHA256",
        Algorithm::Sha512 => "SHA512",
    }
}

/// The license identifiers `CycloneDX` knows (its own copy of the SPDX list).
fn known_licenses() -> BTreeSet<String> {
    serde_json::from_str::<Value>(CYCLONEDX_LICENSES)
        .ok()
        .and_then(|schema| schema.get("enum").cloned())
        .and_then(|list| list.as_array().cloned())
        .map(|list| {
            list.iter()
                .filter_map(|id| id.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// A license as `CycloneDX` writes it: an identifier it knows, else the expression or the name.
fn cyclonedx_licenses(license: Option<&str>, known: &BTreeSet<String>) -> Value {
    match license {
        None => json!([]),
        Some(text) if known.contains(text) => json!([{"license": {"id": text}}]),
        Some(text) if text.contains(' ') || text.contains('(') => json!([{"expression": text}]),
        Some(text) => json!([{"license": {"name": text}}]),
    }
}

fn cyclonedx_component(part: &Part, known: &BTreeSet<String>) -> Value {
    let mut component = json!({
        "type": if part.kind == Kind::File { "file" } else { "library" },
        "bom-ref": part.reference(),
        "name": part.name,
    });
    let map = component.as_object_mut().into_iter().next();
    if let Some(map) = map {
        if !part.version.is_empty() {
            map.insert("version".into(), json!(part.version));
        }
        if let Some(purpose) = &part.purpose {
            map.insert("description".into(), json!(purpose));
        }
        map.insert("scope".into(), json!("required"));
        if let Some(checksum) = &part.checksum {
            map.insert(
                "hashes".into(),
                json!([{"alg": cyclonedx_algorithm(checksum), "content": checksum.hex}]),
            );
        }
        map.insert(
            "licenses".into(),
            cyclonedx_licenses(part.license.as_deref(), known),
        );
        if let Some(purl) = part.purl() {
            map.insert("purl".into(), json!(purl));
        }
    }
    component
}

/// The parts list as a `CycloneDX` 1.6 document.
pub(crate) fn cyclonedx(list: &PartsList) -> Value {
    let known = known_licenses();
    let program = &list.program;
    let digest = content_hash(list);
    let serial = serial_number(&digest);
    let mut dependencies = vec![json!({
        "ref": program.reference(),
        "dependsOn": list.direct.iter().collect::<Vec<_>>(),
    })];
    for part in &list.parts {
        dependencies.push(json!({
            "ref": part.reference(),
            "dependsOn": part.depends_on.iter().collect::<Vec<_>>(),
        }));
    }
    json!({
        "$schema": "http://cyclonedx.org/schema/bom-1.6.schema.json",
        "bomFormat": "CycloneDX",
        "specVersion": CYCLONEDX_VERSION,
        "serialNumber": serial,
        "version": 1,
        "metadata": {
            "timestamp": list.created,
            "tools": {"components": [{"type": "application", "name": MAKER}]},
            "component": {
                "type": "application",
                "bom-ref": program.reference(),
                "name": program.name,
                "version": program.version,
                "licenses": cyclonedx_licenses(Some(&program.license), &known),
            },
        },
        "components": list.parts.iter().map(|part| cyclonedx_component(part, &known)).collect::<Vec<_>>(),
        "dependencies": dependencies,
    })
}

/// The parts list as an SPDX 2.3 document.
pub(crate) fn spdx(list: &PartsList) -> Value {
    let program = &list.program;
    let program_id = "SPDXRef-Program";
    let id_of = |index: usize| format!("SPDXRef-Package-{index}");
    let mut ids = std::collections::BTreeMap::new();
    for (index, part) in list.parts.iter().enumerate() {
        ids.insert(part.reference(), id_of(index));
    }
    let mut packages = vec![json!({
        "SPDXID": program_id,
        "name": program.name,
        "versionInfo": program.version,
        "downloadLocation": "NOASSERTION",
        "filesAnalyzed": false,
        "licenseConcluded": "NOASSERTION",
        "licenseDeclared": program.license,
        "copyrightText": "NOASSERTION",
        "primaryPackagePurpose": "APPLICATION",
    })];
    let mut relationships = vec![json!({
        "spdxElementId": "SPDXRef-DOCUMENT",
        "relationshipType": "DESCRIBES",
        "relatedSpdxElement": program_id,
    })];
    for (index, part) in list.parts.iter().enumerate() {
        let mut package = json!({
            "SPDXID": id_of(index),
            "name": part.name,
            "downloadLocation": "NOASSERTION",
            "filesAnalyzed": false,
            "licenseConcluded": "NOASSERTION",
            "licenseDeclared": part.license.as_deref().unwrap_or("NOASSERTION"),
            "copyrightText": "NOASSERTION",
        });
        if let Some(map) = package.as_object_mut() {
            if !part.version.is_empty() {
                map.insert("versionInfo".into(), json!(part.version));
            }
            if let Some(purpose) = &part.purpose {
                map.insert("description".into(), json!(purpose));
            }
            if let Some(checksum) = &part.checksum {
                map.insert(
                    "checksums".into(),
                    json!([{"algorithm": spdx_algorithm(checksum), "checksumValue": checksum.hex}]),
                );
            }
            if let Some(purl) = part.purl() {
                map.insert(
                    "externalRefs".into(),
                    json!([{
                        "referenceCategory": "PACKAGE-MANAGER",
                        "referenceType": "purl",
                        "referenceLocator": purl,
                    }]),
                );
            }
        }
        packages.push(package);
        for needed in &part.depends_on {
            if let Some(target) = ids.get(needed) {
                relationships.push(json!({
                    "spdxElementId": id_of(index),
                    "relationshipType": "DEPENDS_ON",
                    "relatedSpdxElement": target,
                }));
            }
        }
    }
    for needed in &list.direct {
        if let Some(target) = ids.get(needed) {
            relationships.push(json!({
                "spdxElementId": program_id,
                "relationshipType": "DEPENDS_ON",
                "relatedSpdxElement": target,
            }));
        }
    }
    json!({
        "spdxVersion": SPDX_VERSION,
        "dataLicense": "CC0-1.0",
        "SPDXID": "SPDXRef-DOCUMENT",
        "name": format!("{}-{}", program.name, program.version),
        "documentNamespace": format!(
            "https://spdx.org/spdxdocs/{}-{}-{}",
            program.name,
            program.version,
            content_hash(list)
        ),
        "creationInfo": {"created": list.created, "creators": [format!("Tool: {MAKER}")]},
        "packages": packages,
        "relationships": relationships,
    })
}

/// Gives the schemas that the `CycloneDX` schema points to, by their file name.
struct Neighbours;

impl Retrieve for Neighbours {
    fn retrieve(
        &self,
        uri: &Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let text = match uri.path().as_str().rsplit('/').next().unwrap_or_default() {
            "spdx.schema.json" => CYCLONEDX_LICENSES,
            "jsf-0.82.schema.json" => CYCLONEDX_SIGNATURES,
            other => return Err(format!("the schema {other} is not kept as test data").into()),
        };
        Ok(serde_json::from_str(text)?)
    }
}

fn validator(schema: &str, with_neighbours: bool) -> anyhow::Result<Validator> {
    let schema: Value = serde_json::from_str(schema)?;
    let mut options = jsonschema::draft7::options();
    if with_neighbours {
        options = options.with_retriever(Neighbours);
    }
    options
        .build(&schema)
        .map_err(|error| anyhow::anyhow!("the schema cannot be used: {error}"))
}

/// What is wrong with a `CycloneDX` 1.6 document, one line each; empty when it is valid.
pub(crate) fn cyclonedx_errors(document: &Value) -> anyhow::Result<Vec<String>> {
    Ok(errors(&validator(CYCLONEDX_SCHEMA, true)?, document))
}

/// What is wrong with an SPDX 2.3 document, one line each; empty when it is valid.
pub(crate) fn spdx_errors(document: &Value) -> anyhow::Result<Vec<String>> {
    Ok(errors(&validator(SPDX_SCHEMA, false)?, document))
}

fn errors(validator: &Validator, document: &Value) -> Vec<String> {
    validator
        .iter_errors(document)
        .take(10)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect()
}

#[cfg(test)]
mod tests;
