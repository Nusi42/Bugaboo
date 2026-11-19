//! The release folder (SEC-18, ADR-0047 decisions 1 and 7): `target/release-folder/<version>/` with
//! the programs for Windows x64 and Linux x64, the parts list, the notices, the VEX statements, the
//! release notes, `SHA256SUMS`, and a signature for each of these.
//!
//! What a folder must hold is written once here ([`payload`]); the writer, the signer and the
//! check of a finished folder (the `release` part of DEP-SBOM) read it. A test build carries
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

use super::formats;

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

/// The name of the release notes in the folder.
pub(crate) fn notes_name(release: &str) -> String {
    format!("bugaboo-{release}-notes.md")
}

/// Every file of the release named `release` that is listed in `SHA256SUMS` and signed: the
/// programs, the parts list in both formats, the notices, the VEX statements and the notes.
pub(crate) fn payload(release: &str) -> Vec<String> {
    let mut names: Vec<String> = PLATFORMS
        .iter()
        .map(|platform| program_name(release, *platform))
        .collect();
    names.extend([
        about::release_name(release, about::CYCLONEDX_END),
        about::release_name(release, about::SPDX_END),
        about::release_name(release, about::NOTICES_END),
        formats::vex_name(release),
        notes_name(release),
    ]);
    names
}

/// Every file that is signed: the payload and `SHA256SUMS`.
pub(crate) fn signed_files(release: &str) -> Vec<String> {
    let mut names = payload(release);
    names.push(CHECKSUMS_FILE.to_owned());
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
/// a file or a signature that is missing, a checksum that differs from `SHA256SUMS`, a signature
/// that does not hold or names another file or version, and anything in the folder that does not
/// belong in it. Empty when the folder is complete, signed, and holds nothing else.
pub(crate) fn problems(folder: &Path, release: &str, keys: &[ReleaseKey]) -> Vec<String> {
    let mut found = Vec::new();
    let sums = fs::read_to_string(folder.join(CHECKSUMS_FILE)).unwrap_or_default();
    for name in signed_files(release) {
        let path = folder.join(&name);
        let Ok(bytes) = fs::read(&path) else {
            found.push(format!("{name} is missing."));
            continue;
        };
        if name != CHECKSUMS_FILE {
            let sum = bb_core::hex::encode(&Sha256::digest(&bytes));
            if release::checksum_of(&sums, &name).as_deref() != Some(sum.as_str()) {
                found.push(format!(
                    "{name} has no entry in {CHECKSUMS_FILE} with its checksum."
                ));
            }
        }
        let signature_path = folder.join(signature_name(&name));
        let Ok(signature) = fs::read_to_string(&signature_path) else {
            found.push(format!("{} is missing.", signature_name(&name)));
            continue;
        };
        match release::verify(&bytes, &signature, keys) {
            Ok(comment) if comment == trusted_comment(&name, release) => {}
            Ok(comment) => found.push(format!(
                "{} names `{comment}`, not this file of the release {release}.",
                signature_name(&name)
            )),
            Err(problem) => found.push(format!(
                "{} does not hold: {problem:?}.",
                signature_name(&name)
            )),
        }
    }
    found.extend(strangers(folder, release));
    found
}

/// What stands in `folder` that is neither a signed file of the release named `release` nor its
/// signature, one sentence each, by name.
fn strangers(folder: &Path, release: &str) -> Vec<String> {
    let signed = signed_files(release);
    let signatures: Vec<String> = signed.iter().map(|name| signature_name(name)).collect();
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut found: Vec<String> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| !signed.contains(name) && !signatures.contains(name))
        .map(|name| format!("{name} does not belong in the release folder."))
        .collect();
    found.sort();
    found
}

/// The problems of the folder against the keys the program carries.
pub(crate) fn problems_with_our_keys(folder: &Path, release: &str) -> Vec<String> {
    problems(folder, release, RELEASE_PUBLIC_KEYS)
}
