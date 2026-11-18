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
use crate::checks::dep_register::Rows;

/// The start of the name of a file that holds a license or a notice, in lower case.
const LICENSE_FILE_STARTS: [&str; 5] = ["license", "licence", "copying", "unlicense", "notice"];
/// A license file bigger than this is not a license text.
const LARGEST_LICENSE_FILE: u64 = 200_000;

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
        if texts.is_empty() {
            without_text.push(label(part));
        }
        for (_, text) in texts {
            let key = hex(&Sha256::digest(text.as_bytes()));
            groups
                .entry(key)
                .or_insert_with(|| (text, BTreeSet::new()))
                .1
                .insert(format!(
                    "{} ({})",
                    label(part),
                    part.license.as_deref().unwrap_or("no license declared")
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
            "\n================================================================\nParts that ship no license file in their package; the license they declare is in the \
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
