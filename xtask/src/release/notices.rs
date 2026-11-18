//! The third-party notices of a release (COMP-02): the license text of every part, each text once
//! with the parts that carry it, then the notices of code and data inside crates
//! (`docs/third-party/notices/`) and the data files that ship.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use sha2::{Digest as _, Sha256};

use super::hex;
use super::parts_list::{Kind, Part, PartsList};
use crate::checks::dep_licenses::{spdx_words, with_or_for_slash};
use crate::checks::dep_register::Rows;

/// The start of the name of a file that holds a license or a notice, in lower case.
const LICENSE_FILE_STARTS: [&str; 5] = ["license", "licence", "copying", "unlicense", "notice"];
/// A license file bigger than this is not a license text.
const LARGEST_LICENSE_FILE: u64 = 200_000;

/// The standard texts of the licenses that crates without a license file declare, from the SPDX
/// license list (`docs/third-party/license-texts/`, copied unchanged).
const STANDARD_TEXTS: [(&str, &str); 6] = [
    (
        "MIT",
        include_str!("../../../docs/third-party/license-texts/MIT.txt"),
    ),
    (
        "Apache-2.0",
        include_str!("../../../docs/third-party/license-texts/Apache-2.0.txt"),
    ),
    (
        "BSD-3-Clause",
        include_str!("../../../docs/third-party/license-texts/BSD-3-Clause.txt"),
    ),
    (
        "BSD-2-Clause",
        include_str!("../../../docs/third-party/license-texts/BSD-2-Clause.txt"),
    ),
    (
        "ISC",
        include_str!("../../../docs/third-party/license-texts/ISC.txt"),
    ),
    (
        "MPL-2.0",
        include_str!("../../../docs/third-party/license-texts/MPL-2.0.txt"),
    ),
];

/// The licenses whose standard text stands in for a missing license file: for `A OR B` the first
/// one that has a text (the expression lists them in the order the package prefers), for `A AND B`
/// all of them, none when one has no text.
pub(crate) fn standard_licenses(expression: &str) -> Vec<&'static str> {
    let words = spdx_words(&with_or_for_slash(expression));
    let ids: Vec<&str> = words
        .iter()
        .map(String::as_str)
        .filter(|word| !matches!(*word, "OR" | "AND" | "WITH" | "(" | ")"))
        .collect();
    let known = |id: &str| {
        STANDARD_TEXTS
            .iter()
            .find(|(name, _)| *name == id)
            .map(|(name, _)| *name)
    };
    if words.iter().any(|word| word == "AND") {
        let all: Vec<&'static str> = ids.iter().filter_map(|id| known(id)).collect();
        return if all.len() == ids.len() {
            all
        } else {
            Vec::new()
        };
    }
    ids.iter().find_map(|id| known(id)).into_iter().collect()
}

/// The standard text of `license` with the authors of `package` as the copyright holders (the
/// year is left out: the package does not say it).
pub(crate) fn standard_text(license: &str, authors: &[String], package: &str) -> Option<String> {
    let (_, template) = STANDARD_TEXTS.iter().find(|(name, _)| *name == license)?;
    let names: Vec<&str> = authors
        .iter()
        .map(|author| author.split('<').next().unwrap_or(author).trim())
        .filter(|name| !name.is_empty())
        .collect();
    let holders = if names.is_empty() {
        format!("the authors of {package}")
    } else {
        names.join(", ")
    };
    Some(
        template
            .replace("<year> <copyright holders>", &holders)
            .replace("<year> <owner>", &holders)
            .replace("<year>, <owner>", &holders),
    )
}

/// The notices, and the parts for which no license text was found in the package.
#[derive(Debug)]
pub(crate) struct Notices {
    pub(crate) text: String,
    /// `name version` of each crate or package that ships no license file.
    pub(crate) without_text: Vec<String>,
}

/// The license texts in the folder of a package: `(file name, text)`, in order of file name.
pub(crate) fn license_texts(folder: &Path) -> Vec<(String, String)> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut texts = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let lower = name.to_ascii_lowercase();
        let metadata = entry.metadata();
        let is_license = LICENSE_FILE_STARTS
            .iter()
            .any(|start| lower.starts_with(start));
        if is_license
            && metadata
                .as_ref()
                .is_ok_and(|data| data.is_file() && data.len() <= LARGEST_LICENSE_FILE)
            && let Ok(bytes) = fs::read(entry.path())
        {
            let text = String::from_utf8_lossy(&bytes).replace("\r\n", "\n");
            let text = text.trim().to_owned();
            if !text.is_empty() {
                texts.push((name, text));
            }
        }
    }
    texts.sort();
    texts
}

fn label(part: &Part) -> String {
    format!("{} {}", part.name, part.version)
}

/// Makes the notices of `list`.
pub(crate) fn notices(root: &Path, list: &PartsList, register: &Rows) -> anyhow::Result<Notices> {
    // text -> (the first line of the group, the parts that carry it)
    let mut groups: BTreeMap<String, (String, BTreeSet<String>)> = BTreeMap::new();
    let mut without_text = Vec::new();
    for part in list.parts.iter().filter(|part| part.kind != Kind::File) {
        let texts = part
            .folder
            .as_deref()
            .map(license_texts)
            .unwrap_or_default();
        let mut texts: Vec<String> = texts.into_iter().map(|(_, text)| text).collect();
        let mut standard = false;
        if texts.is_empty() {
            // No license file in the package: the standard text of the declared license stands in.
            let declared = part.license.as_deref().unwrap_or_default();
            texts = standard_licenses(declared)
                .into_iter()
                .filter_map(|id| standard_text(id, &part.authors, &part.name))
                .map(|text| text.trim().to_owned())
                .collect();
            standard = !texts.is_empty();
            if texts.is_empty() {
                without_text.push(label(part));
            }
        }
        for text in texts {
            let key = hex(&Sha256::digest(text.as_bytes()));
            groups
                .entry(key)
                .or_insert_with(|| (text, BTreeSet::new()))
                .1
                .insert(format!(
                    "{} ({}{})",
                    label(part),
                    part.license.as_deref().unwrap_or("no license declared"),
                    if standard { ", standard text" } else { "" }
                ));
        }
    }
    let program = &list.program;
    let mut out = format!(
        "Third-party notices of {} {}\n\n{} is licensed under {} (the file LICENSE). It contains \
         the parts below, each under its own license. Each license text is given once, with the \
         parts that carry it.\n",
        program.name, program.version, program.name, program.license
    );
    let mut ordered: Vec<&(String, BTreeSet<String>)> = groups.values().collect();
    ordered.sort_by(|a, b| a.1.cmp(&b.1));
    for (number, (text, parts)) in ordered.iter().enumerate() {
        let _ = write!(
            out,
            "\n================================================================\nLicense text {} of {}\nParts: {}\n\n{}\n",
            number + 1,
            ordered.len(),
            parts.iter().cloned().collect::<Vec<_>>().join(", "),
            text
        );
    }
    if !without_text.is_empty() {
        let _ = write!(
            out,
            "\n================================================================\nParts that ship no license file in their package, with a license that has no standard text; the license \n             they declare is in the \
             parts list:\n{}\n",
            without_text.join("\n")
        );
    }
    for part in list.parts.iter().filter(|part| part.kind == Kind::File) {
        let _ = write!(
            out,
            "\n================================================================\n{}\n",
            part.name
        );
        if let Some(row) = register.data.get(&part.name) {
            let _ = writeln!(out, "License: {}", row.license);
        } else if let Ok(text) = fs::read_to_string(root.join(&part.name)) {
            let _ = write!(out, "\n{}\n", text.replace("\r\n", "\n").trim_end());
        }
    }
    Ok(Notices {
        text: out,
        without_text,
    })
}

#[cfg(test)]
mod tests;
