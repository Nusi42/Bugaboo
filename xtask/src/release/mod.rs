//! What a release carries besides the programs: the parts list (SBOM), the third-party notices
//! and the VEX statements (COMP-01, COMP-02, COMP-03; ADR-0047, decisions 3 and 4).
//!
//! All three are written from the lockfiles, the register and the accepted advisories, never by
//! hand, and in a fixed order, so two runs of one commit write the same bytes.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use anyhow::Context as _;

pub(crate) mod formats;
pub(crate) mod notices;
pub(crate) mod parts_list;

/// The bytes as lower-case hexadecimal.
pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}

/// A JSON document as the text a release file holds: indented, with a final line break.
pub(crate) fn to_text(document: &serde_json::Value) -> String {
    let mut text = serde_json::to_string_pretty(document).unwrap_or_default();
    text.push('\n');
    text
}

/// Writes `text` to `folder/name`, making the folder first.
pub(crate) fn write_file(folder: &Path, name: &str, text: &str) -> anyhow::Result<()> {
    fs::create_dir_all(folder).with_context(|| format!("could not create {}", folder.display()))?;
    let path = folder.join(name);
    fs::write(&path, text).with_context(|| format!("could not write {}", path.display()))
}

/// Everything made from one state of the repository.
pub(crate) struct Files {
    pub(crate) list: parts_list::PartsList,
    pub(crate) cyclonedx: serde_json::Value,
    pub(crate) spdx: serde_json::Value,
    pub(crate) notices: notices::Notices,
}

/// Makes the parts list in both formats and the notices from the repository at `root`.
pub(crate) fn compose(root: &Path) -> anyhow::Result<Files> {
    let list = parts_list::collect(root, &parts_list::commit_time(root))?;
    let register = crate::checks::dep_register::read_register(root)?;
    Ok(Files {
        cyclonedx: formats::cyclonedx(&list),
        spdx: formats::spdx(&list),
        notices: notices::notices(root, &list, &register)?,
        list,
    })
}

/// Writes the three files into `folder` under their release names, and nothing else.
pub(crate) fn write(files: &Files, folder: &Path) -> anyhow::Result<()> {
    let version = &files.list.program.version;
    write_file(
        folder,
        &formats::cyclonedx_name(version),
        &to_text(&files.cyclonedx),
    )?;
    write_file(folder, &formats::spdx_name(version), &to_text(&files.spdx))?;
    write_file(folder, &formats::notices_name(version), &files.notices.text)
}

/// Where `cargo xtask parts-list` and the check DEP-SBOM write the files: in the build folder.
pub(crate) fn work_folder(root: &Path) -> anyhow::Result<std::path::PathBuf> {
    Ok(crate::metadata::target_directory(root)?.join("parts-list"))
}

/// The files of this very repository, made once for all the tests that read them.
#[cfg(test)]
pub(crate) fn this_repository() -> &'static Files {
    static FILES: std::sync::OnceLock<Files> = std::sync::OnceLock::new();
    FILES.get_or_init(|| compose(&repository_root()).expect("the parts list of this repository"))
}

#[cfg(test)]
pub(crate) fn repository_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the repository")
        .to_path_buf()
}

/// What is wrong with the two files against their schemas, one line each, naming the file.
pub(crate) fn schema_errors(files: &Files) -> anyhow::Result<Vec<String>> {
    let version = &files.list.program.version;
    let mut lines = Vec::new();
    for error in formats::cyclonedx_errors(&files.cyclonedx)? {
        lines.push(format!("{}: {error}", formats::cyclonedx_name(version)));
    }
    for error in formats::spdx_errors(&files.spdx)? {
        lines.push(format!("{}: {error}", formats::spdx_name(version)));
    }
    Ok(lines)
}
