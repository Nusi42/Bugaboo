use std::fs;
use std::time::SystemTime;

use bb_core::release::checksum_line;
use bb_testing::release::TestKey;

use super::*;

const PROGRAM: &str = "bugaboo-0.6.0-linux-x64";

/// A folder like a downloaded release: the program, `SHA256SUMS` and its signature, the only one,
/// signed by a key made for the test, for the release `version`.
struct Download {
    folder: tempfile::TempDir,
    key: TestKey,
    version: &'static str,
}

impl Download {
    fn new(version: &'static str) -> Self {
        let download = Self {
            folder: tempfile::tempdir().unwrap(),
            key: TestKey::new().unwrap(),
            version,
        };
        fs::write(download.program(), b"the program").unwrap();
        let sum = bb_core::hex::encode(&Sha256::digest(b"the program"));
        download.list(checksum_line(&sum, PROGRAM).as_bytes());
        download
    }

    /// Writes `SHA256SUMS` with `bytes` and its signature.
    fn list(&self, bytes: &[u8]) {
        self.list_as(bytes, CHECKSUMS_FILE, self.version);
    }

    /// Writes `SHA256SUMS` with a signature whose trusted comment names `file` and `version`.
    fn list_as(&self, bytes: &[u8], file: &str, version: &str) {
        let comment = TrustedComment {
            file: file.to_owned(),
            version: version.to_owned(),
        };
        self.list_signed(bytes, &comment.text());
    }

    /// Writes `SHA256SUMS` with a signature whose trusted comment is `comment`.
    fn list_signed(&self, bytes: &[u8], comment: &str) {
        fs::write(self.folder.path().join(CHECKSUMS_FILE), bytes).unwrap();
        fs::write(
            self.folder
                .path()
                .join(release::signature_name(CHECKSUMS_FILE)),
            self.key.sign(bytes, comment).unwrap(),
        )
        .unwrap();
    }

    fn program(&self) -> PathBuf {
        self.folder.path().join(PROGRAM)
    }

    fn keys(&self) -> Vec<ReleaseKey> {
        vec![ReleaseKey {
            name: "release",
            public: self.key.public().leak(),
        }]
    }

    fn check(&self, installed: &str) -> Result<Checked, Refused> {
        check(&self.program(), &self.keys(), installed)
    }
}

#[test]
fn sec_18_a_list_signed_as_another_file_is_refused() {
    let download = Download::new("0.6.0");
    let sums = fs::read(download.folder.path().join(CHECKSUMS_FILE)).unwrap();
    download.list_as(&sums, PROGRAM, "0.6.0");
    assert_eq!(
        download.check("0.6.0"),
        Err(Refused::OtherList(PROGRAM.to_owned(), "0.6.0".to_owned()))
    );
    // The version the list is signed for is the version of the file.
    download.list_as(&sums, CHECKSUMS_FILE, "0.6.1");
    assert_eq!(download.check("0.6.0").unwrap().signed.version, "0.6.1");
}

#[test]
fn sec_18_the_list_alone_is_signed_and_a_changed_byte_fails() {
    let download = Download::new("0.6.0");
    let checked = download.check("0.6.0").unwrap();
    assert_eq!(checked.signed.version, "0.6.0");
    assert_eq!(checked.signed.file, PROGRAM);
    assert!(!checked.older);
    // No file but the list has a signature.
    let mut names: Vec<_> = fs::read_dir(download.folder.path())
        .unwrap()
        .map(|entry| name_of(&entry.unwrap().path()))
        .collect();
    names.sort();
    assert_eq!(names, [CHECKSUMS_FILE, "SHA256SUMS.minisig", PROGRAM]);
    let words = message::cli_update_signed(message::CliUpdateSigned {
        file: &PROGRAM,
        version: &"0.6.0",
    })
    .render();
    assert_eq!(
        words,
        format!("Signed by Bugaboo, version 0.6.0: {PROGRAM}")
    );

    fs::write(download.program(), b"the progran").unwrap();
    assert_eq!(
        download.check("0.6.0"),
        Err(Refused::OtherChecksum(PROGRAM.to_owned()))
    );
    // A list whose signature is gone is refused, with the name of what is missing.
    fs::write(download.program(), b"the program").unwrap();
    let signature = download
        .folder
        .path()
        .join(release::signature_name(CHECKSUMS_FILE));
    fs::remove_file(&signature).unwrap();
    assert_eq!(
        download.check("0.6.0"),
        Err(Refused::Missing(signature.display().to_string()))
    );
}

#[test]
fn sec_18_a_download_saved_under_another_name_is_found_by_its_checksum() {
    let download = Download::new("0.6.0");
    let saved = download.folder.path().join("bugaboo (1)");
    fs::rename(download.program(), &saved).unwrap();
    let checked = check(&saved, &download.keys(), "0.6.0").unwrap();
    assert_eq!(checked.signed.file, PROGRAM);
    // A changed list does not hold.
    let sums = download.folder.path().join(CHECKSUMS_FILE);
    fs::write(
        &sums, "changed
",
    )
    .unwrap();
    assert_eq!(
        check(&saved, &download.keys(), "0.6.0"),
        Err(Refused::Signature(
            CHECKSUMS_FILE.to_owned(),
            SignatureProblem::Changed
        ))
    );
}

#[test]
fn sec_18_a_file_missing_from_sha256sums_is_refused() {
    let download = Download::new("0.6.0");
    // A list signed by our key that names another file only.
    let sum = "ab".repeat(32);
    download.list(checksum_line(&sum, "bugaboo-0.6.0-windows-x64.exe").as_bytes());
    assert_eq!(
        download.check("0.6.0"),
        Err(Refused::NotListed(PROGRAM.to_owned()))
    );
    // A list that names it with another checksum.
    download.list(checksum_line(&sum, PROGRAM).as_bytes());
    assert_eq!(
        download.check("0.6.0"),
        Err(Refused::OtherChecksum(PROGRAM.to_owned()))
    );
    // A list that is not signed by our key does not count, whatever it says.
    let other = Download::new("0.6.0");
    fs::copy(
        other
            .folder
            .path()
            .join(release::signature_name(CHECKSUMS_FILE)),
        download
            .folder
            .path()
            .join(release::signature_name(CHECKSUMS_FILE)),
    )
    .unwrap();
    assert_eq!(
        download.check("0.6.0"),
        Err(Refused::Signature(
            CHECKSUMS_FILE.to_owned(),
            SignatureProblem::UnknownKey
        ))
    );
}

#[test]
fn dist_07_update_verify_warns_on_an_older_version() {
    let download = Download::new("0.5.1");
    let checked = download.check("0.6.0").unwrap();
    assert!(checked.older);
    let warning = message::cli_update_older(message::CliUpdateOlder {
        installed: &"0.6.0",
        version: &"0.5.1",
    })
    .render();
    assert!(
        warning.starts_with("Version 0.5.1 is older than version 0.6.0,"),
        "{warning}"
    );
    // The same version and a newer one are no downgrade.
    assert!(!Download::new("0.6.0").check("0.6.0").unwrap().older);
    assert!(!Download::new("0.6.1").check("0.6.0").unwrap().older);
}

#[test]
fn sec_18_update_verify_fails_for_an_older_release_unless_it_is_allowed() {
    let args = |download: &Download, allow_older: bool| VerifyArgs {
        file: download.program(),
        allow_older,
    };
    let older = Download::new("0.5.1");
    let refused = verify_with(&args(&older, false), &older.keys(), "0.6.0")
        .unwrap_err()
        .to_string();
    assert!(refused.contains("--allow-older"), "{refused}");
    assert!(refused.contains("0.5.1"), "{refused}");
    assert!(verify_with(&args(&older, true), &older.keys(), "0.6.0").is_ok());
    // A release candidate of the installed version is older than the version itself.
    let candidate = Download::new("0.6.0-rc1");
    assert!(verify_with(&args(&candidate, false), &candidate.keys(), "0.6.0").is_err());
    let same = Download::new("0.6.0");
    assert!(verify_with(&args(&same, false), &same.keys(), "0.6.0").is_ok());
}

/// Every file of a folder with its bytes and the time it was last changed.
fn snapshot(folder: &Path) -> Vec<(String, Vec<u8>, SystemTime)> {
    let mut files: Vec<_> = fs::read_dir(folder)
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            let changed = fs::metadata(&path).unwrap().modified().unwrap();
            (name_of(&path), fs::read(&path).unwrap(), changed)
        })
        .collect();
    files.sort();
    files
}

#[test]
fn dist_07_update_verify_never_changes_a_file() {
    let download = Download::new("0.5.1");
    let before = snapshot(download.folder.path());
    assert!(download.check("0.6.0").is_ok());
    assert_eq!(snapshot(download.folder.path()), before);
    // Nor when it refuses.
    fs::write(download.program(), b"the progran").unwrap();
    let before = snapshot(download.folder.path());
    assert!(download.check("0.6.0").is_err());
    assert_eq!(snapshot(download.folder.path()), before);
}

#[test]
fn a_list_signed_without_a_release_is_refused() {
    let download = Download::new("0.6.0");
    let sums = fs::read(download.folder.path().join(CHECKSUMS_FILE)).unwrap();
    download.list_signed(&sums, "timestamp:1556193335	file:test");
    assert_eq!(
        download.check("0.6.0"),
        Err(Refused::NoRelease(CHECKSUMS_FILE.to_owned()))
    );
}
