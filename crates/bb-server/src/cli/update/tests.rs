use std::fs;
use std::time::SystemTime;

use bb_core::release::checksum_line;
use bb_testing::release::TestKey;

use super::*;

const PROGRAM: &str = "bugaboo-0.6.0-linux-x64";

/// A folder like a downloaded release: the program, `SHA256SUMS` and both signatures, signed by a
/// key made for the test, for the release `version`.
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
        download.put(PROGRAM, b"the program");
        let sum = bb_core::hex::encode(&Sha256::digest(b"the program"));
        download.put(CHECKSUMS_FILE, checksum_line(&sum, PROGRAM).as_bytes());
        download
    }

    /// Writes the file `name` and its signature.
    fn put(&self, name: &str, bytes: &[u8]) {
        self.put_as(name, bytes, name, self.version);
    }

    /// Writes the file `name` and a signature whose trusted comment names `file` and `version`.
    fn put_as(&self, name: &str, bytes: &[u8], file: &str, version: &str) {
        let comment = TrustedComment {
            file: file.to_owned(),
            version: version.to_owned(),
        };
        fs::write(self.folder.path().join(name), bytes).unwrap();
        fs::write(
            self.folder.path().join(release::signature_name(name)),
            self.key.sign(bytes, &comment.text()).unwrap(),
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
fn sec_18_a_list_signed_for_another_file_or_release_is_refused() {
    let download = Download::new("0.6.0");
    let sums = fs::read(download.folder.path().join(CHECKSUMS_FILE)).unwrap();
    for (file, version) in [(CHECKSUMS_FILE, "0.5.1"), (PROGRAM, "0.6.0")] {
        download.put_as(CHECKSUMS_FILE, &sums, file, version);
        assert_eq!(
            download.check("0.6.0"),
            Err(Refused::OtherList(PROGRAM.to_owned(), "0.6.0".to_owned())),
            "{file} {version}"
        );
    }
    download.put_as(CHECKSUMS_FILE, &sums, CHECKSUMS_FILE, "0.6.0");
    assert!(download.check("0.6.0").is_ok());
}

#[test]
fn sec_18_a_signed_file_verifies_and_a_changed_byte_fails() {
    let download = Download::new("0.6.0");
    let checked = download.check("0.6.0").unwrap();
    assert_eq!(checked.signed.version, "0.6.0");
    assert_eq!(checked.signed.file, PROGRAM);
    assert!(!checked.older);
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
        Err(Refused::Signature(
            PROGRAM.to_owned(),
            SignatureProblem::Changed
        ))
    );
    // A file whose signature is gone is refused too, with the name of what is missing.
    fs::remove_file(
        download
            .folder
            .path()
            .join(release::signature_name(PROGRAM)),
    )
    .unwrap();
    assert!(matches!(download.check("0.6.0"), Err(Refused::Missing(_))));
}

#[test]
fn sec_18_a_file_missing_from_sha256sums_is_refused() {
    let download = Download::new("0.6.0");
    // A list signed by our key that names another file only.
    let sum = "ab".repeat(32);
    download.put(
        CHECKSUMS_FILE,
        checksum_line(&sum, "bugaboo-0.6.0-windows-x64.exe").as_bytes(),
    );
    assert_eq!(
        download.check("0.6.0"),
        Err(Refused::NotListed(PROGRAM.to_owned()))
    );
    // A list that names it with another checksum.
    download.put(CHECKSUMS_FILE, checksum_line(&sum, PROGRAM).as_bytes());
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
fn a_file_signed_without_a_release_is_refused() {
    let download = Download::new("0.6.0");
    let comment = "timestamp:1556193335\tfile:test";
    fs::write(
        download
            .folder
            .path()
            .join(release::signature_name(PROGRAM)),
        download.key.sign(b"the program", comment).unwrap(),
    )
    .unwrap();
    assert_eq!(
        download.check("0.6.0"),
        Err(Refused::NoRelease(PROGRAM.to_owned()))
    );
}
