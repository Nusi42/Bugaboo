//! The release folder (SEC-18, ADR-0047 decisions 1 and 7, as amended): `target/release-folder/
//! <version>/` with exactly six files: the programs for Windows x64 and Linux x64, the notices, the
//! `CycloneDX` parts list with the VEX statements in it, `SHA256SUMS` naming these four, and
//! `SHA256SUMS.minisig`, the one signature. The release notes are written beside the folder: they
//! are the text of the release, not a file of it.
//!
//! What a folder must hold is written once here ([`payload`], [`files`]); the writer, the signer
//! and the check of a finished folder (the `release` part of DEP-SBOM) read it. A test build carries
//! `-test` in its one release name ([`Build::release_name`]): in the name of the folder, of every
//! file and in the version of every trusted comment, so its signed files never pass for a release.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use bb_api_words::about;
use bb_core::release::{
    self, CHECKSUMS_FILE, KEYS_ARE_FOR_TESTS, RELEASE_PUBLIC_KEYS, ReleaseKey, TrustedComment,
    checksum_line, signature_name,
};
use sha2::{Digest as _, Sha256};

use super::{formats, vex};

/// What a release folder is built from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Build {
    /// Whether the tag `v<version>` is on the commit it is built from.
    pub(crate) tagged: bool,
    /// Whether the program trusts only test keys ([`KEYS_ARE_FOR_TESTS`]).
    pub(crate) test_keys: bool,
}

impl Build {
    /// The build of this program from a commit that has its tag or not.
    pub(crate) const fn of_this_program(tagged: bool) -> Self {
        Self {
            tagged,
            test_keys: KEYS_ARE_FOR_TESTS,
        }
    }

    /// Whether this is a test build: its tag is not on the commit, or the keys are for tests.
    pub(crate) const fn is_test(self) -> bool {
        !self.tagged || self.test_keys
    }

    /// The name of the release of `version`: `<version>`, or `<version>-test` for a test build.
    /// It names the folder and every file, and is the version of every trusted comment.
    pub(crate) fn release_name(self, version: &str) -> String {
        if self.is_test() {
            format!("{version}-test")
        } else {
            version.to_owned()
        }
    }
}

/// A system a release has a program for: its name in the file and the target it is built for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Platform {
    /// The word in the name of the file: `windows-x64`.
    pub(crate) word: &'static str,
    /// The Rust target of the program, `None` for the computer's own (Windows).
    pub(crate) target: Option<&'static str>,
    /// The ending of the program's file.
    pub(crate) end: &'static str,
}

/// The programs of a release (ADR-0047, decision 7): Windows x64 and Linux x64, static. ARM64
/// joins when its build tools are there.
pub(crate) const PLATFORMS: [Platform; 2] = [
    Platform {
        word: "windows-x64",
        target: None,
        end: ".exe",
    },
    Platform {
        word: "linux-x64",
        target: Some("x86_64-unknown-linux-musl"),
        end: "",
    },
];

/// The Linux program, built in WSL.
pub(crate) const LINUX: Platform = PLATFORMS[1];

/// The name of the program for `platform` in the release named `release`.
pub(crate) fn program_name(release: &str, platform: Platform) -> String {
    format!("bugaboo-{release}-{}{}", platform.word, platform.end)
}

/// The name of the release notes, written beside the folder ([`notes_path`]).
pub(crate) fn notes_name(release: &str) -> String {
    format!("bugaboo-{release}-notes.md")
}

/// Where the release notes of the folder `folder` go: beside it, as the text of the release.
pub(crate) fn notes_path(folder: &Path, release: &str) -> PathBuf {
    folder.with_file_name(notes_name(release))
}

/// Every file of the release named `release` that `SHA256SUMS` lists: the programs, the notices
/// and the `CycloneDX` parts list with the VEX statements.
pub(crate) fn payload(release: &str) -> Vec<String> {
    let mut names: Vec<String> = PLATFORMS
        .iter()
        .map(|platform| program_name(release, *platform))
        .collect();
    names.extend([
        about::release_name(release, about::NOTICES_END),
        formats::cyclonedx_name(release),
    ]);
    names
}

/// Every file of a finished folder of the release named `release`, and nothing else: the payload,
/// `SHA256SUMS` and its signature, the only one.
pub(crate) fn files(release: &str) -> Vec<String> {
    let mut names = payload(release);
    names.extend([CHECKSUMS_FILE.to_owned(), signature_name(CHECKSUMS_FILE)]);
    names
}

/// Where the folders of releases go: `release-folder/` of the build folder.
pub(crate) fn release_root(root: &Path) -> anyhow::Result<PathBuf> {
    Ok(crate::metadata::target_directory(root)?.join("release-folder"))
}

/// The SHA-256 checksum of a file, in lower-case hexadecimal.
pub(crate) fn checksum(path: &Path) -> anyhow::Result<String> {
    let bytes = fs::read(path).with_context(|| format!("could not read {}", path.display()))?;
    Ok(bb_core::hex::encode(&Sha256::digest(&bytes)))
}

/// Writes `SHA256SUMS` for the payload of the release named `release` in `folder`, in the order
/// of [`payload`].
pub(crate) fn write_checksums(folder: &Path, release: &str) -> anyhow::Result<()> {
    let mut text = String::new();
    for name in payload(release) {
        text.push_str(&checksum_line(&checksum(&folder.join(&name))?, &name));
    }
    super::write_file(folder, CHECKSUMS_FILE, &text)
}

/// The trusted comment of the signature of `name` in the release named `release`.
pub(crate) fn trusted_comment(name: &str, release: &str) -> String {
    TrustedComment {
        file: name.to_owned(),
        version: release.to_owned(),
    }
    .text()
}

/// What is wrong with the folder of the release named `release` against `keys`, one sentence each:
/// a file that is missing, a checksum that differs from `SHA256SUMS` or an entry of it that names
/// no file of the payload, a signature of `SHA256SUMS` that does not hold or names another file or
/// version, a parts list that does not fit its schema or carries no VEX statements, and anything
/// in the folder that does not belong in it. Empty when the folder holds the six files, signed,
/// and nothing else.
pub(crate) fn problems(folder: &Path, release: &str, keys: &[ReleaseKey]) -> Vec<String> {
    let mut found = Vec::new();
    let sums = fs::read_to_string(folder.join(CHECKSUMS_FILE)).ok();
    let listed = sums
        .as_deref()
        .map(release::checksum_entries)
        .unwrap_or_default();
    let payload = payload(release);
    for name in &payload {
        let Ok(bytes) = fs::read(folder.join(name)) else {
            found.push(format!("{name} is missing."));
            continue;
        };
        let entry = (bb_core::hex::encode(&Sha256::digest(&bytes)), name.clone());
        if sums.is_some() && !listed.contains(&entry) {
            found.push(format!(
                "{name} has no entry in {CHECKSUMS_FILE} with its checksum."
            ));
        }
    }
    if sums
        .as_deref()
        .is_some_and(|text| text.lines().count() != listed.len())
    {
        found.push(format!(
            "{CHECKSUMS_FILE} holds a line that is not a checksum and a name."
        ));
    }
    for (_, name) in &listed {
        if !payload.contains(name) {
            found.push(format!(
                "{CHECKSUMS_FILE} lists {name}, which is not a file of the release."
            ));
        }
    }
    found.extend(signature_problems(folder, release, keys, sums.as_deref()));
    found.extend(parts_list_problems(folder, release));
    found.extend(strangers(folder, release));
    found
}

/// What is wrong with `SHA256SUMS` (`sums`, `None` when it cannot be read) and its signature.
fn signature_problems(
    folder: &Path,
    release: &str,
    keys: &[ReleaseKey],
    sums: Option<&str>,
) -> Vec<String> {
    let signature_name = signature_name(CHECKSUMS_FILE);
    let Some(sums) = sums else {
        return vec![format!("{CHECKSUMS_FILE} is missing.")];
    };
    let Ok(signature) = fs::read_to_string(folder.join(&signature_name)) else {
        return vec![format!("{signature_name} is missing.")];
    };
    match release::verify(sums.as_bytes(), &signature, keys) {
        Ok(comment) if comment == trusted_comment(CHECKSUMS_FILE, release) => Vec::new(),
        Ok(comment) => vec![format!(
            "{signature_name} names `{comment}`, not the {CHECKSUMS_FILE} of the release {release}."
        )],
        Err(problem) => vec![format!("{signature_name} does not hold: {problem:?}.")],
    }
}

/// What is wrong with the `CycloneDX` parts list of the folder: its schema, and its VEX statements.
fn parts_list_problems(folder: &Path, release: &str) -> Vec<String> {
    let name = formats::cyclonedx_name(release);
    let Ok(text) = fs::read_to_string(folder.join(&name)) else {
        // Named as missing with the payload.
        return Vec::new();
    };
    let document = match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(document) => document,
        Err(error) => return vec![format!("{name} is not JSON: {error}.")],
    };
    let mut found: Vec<String> = match formats::cyclonedx_errors(&document) {
        Ok(errors) => errors
            .into_iter()
            .map(|error| format!("{name} does not fit its schema: {error}."))
            .collect(),
        Err(error) => vec![format!("{name} could not be checked: {error}.")],
    };
    found.extend(vex::release_problems(&document, &name));
    found
}

/// What stands in `folder` that is not one of the [`files`] of the release named `release`, one
/// sentence each, by name.
fn strangers(folder: &Path, release: &str) -> Vec<String> {
    let wanted = files(release);
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut found: Vec<String> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| !wanted.contains(name))
        .map(|name| format!("{name} does not belong in the release folder."))
        .collect();
    found.sort();
    found
}

/// The problems of the folder against the keys the program carries.
pub(crate) fn problems_with_our_keys(folder: &Path, release: &str) -> Vec<String> {
    problems(folder, release, RELEASE_PUBLIC_KEYS)
}
