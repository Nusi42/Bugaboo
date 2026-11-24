use bb_core::release::{CHECKSUMS_FILE, ReleaseKey, signature_name};
use minisign::KeyPair;

use super::*;
use crate::release::{this_repository, to_text};

/// A folder of the release `release` as the earlier layout published it: the payload of today, the
/// parts list without statements, the SPDX file, a separate VEX file, the notes, `SHA256SUMS` and a
/// signature for each file.
fn published(release: &str) -> tempfile::TempDir {
    let folder = tempfile::tempdir().unwrap();
    let files = this_repository();
    let write = |name: String, text: &str| fs::write(folder.path().join(name), text).unwrap();
    for name in folder::payload(release) {
        write(name.clone(), &format!("the file {name}"));
    }
    let statements = files.cyclonedx["vulnerabilities"].clone();
    write(
        formats::cyclonedx_name(release),
        &to_text(&formats::cyclonedx(&files.list)),
    );
    let old_vex = serde_json::json!({"bomFormat": "CycloneDX", "vulnerabilities": statements});
    write(
        about::release_name(release, about::VEX_END),
        &to_text(&old_vex),
    );
    write(about::release_name(release, about::SPDX_END), "{}");
    write(folder::notes_name(release), "# notes");
    write(CHECKSUMS_FILE.to_owned(), "old");
    for name in folder::payload(release) {
        write(signature_name(&name), "old signature");
    }
    folder
}

#[test]
fn sec_18_a_release_of_the_earlier_layout_repacks_into_the_six_files() {
    let from = published("0.6.0");
    let out = tempfile::tempdir().unwrap();
    let to = out.path().join("0.6.0");
    repack(from.path(), &to, "0.6.0").unwrap();
    let mut names: Vec<String> = fs::read_dir(&to)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    let mut wanted = folder::payload("0.6.0");
    wanted.sort();
    assert_eq!(names, wanted);
    // The statements are in the parts list now, as `cargo xtask release` puts them.
    let text = fs::read_to_string(to.join(formats::cyclonedx_name("0.6.0"))).unwrap();
    assert_eq!(text, to_text(&this_repository().cyclonedx));
    // The programs and the notices are taken as they are.
    let program = &folder::payload("0.6.0")[0];
    assert_eq!(
        fs::read_to_string(to.join(program)).unwrap(),
        format!("the file {program}")
    );

    // Signed, it passes the check of a release folder.
    let pair = KeyPair::generate_unencrypted_keypair().unwrap();
    let keys = [ReleaseKey {
        name: "release",
        public: pair.pk.to_base64().leak(),
    }];
    assert!(!folder::problems(&to, "0.6.0", &keys).is_empty());
    super::super::sign::sign_folder(&to, "0.6.0", &pair.sk).unwrap();
    assert_eq!(folder::problems(&to, "0.6.0", &keys), Vec::<String>::new());
    // The published files are only read.
    assert!(
        from.path()
            .join(about::release_name("0.6.0", about::VEX_END))
            .is_file()
    );
}

#[test]
fn sec_18_repack_refuses_a_folder_without_the_files_of_the_release() {
    let from = published("0.6.0");
    let out = tempfile::tempdir().unwrap();
    // The files of another release.
    let refused = repack(from.path(), &out.path().join("0.6.1"), "0.6.1").unwrap_err();
    assert!(refused.to_string().contains("0.6.1"), "{refused:#}");
    // A program that is missing.
    fs::remove_file(from.path().join(&folder::payload("0.6.0")[0])).unwrap();
    assert!(repack(from.path(), &out.path().join("0.6.0"), "0.6.0").is_err());
    // The release folder itself.
    let from = published("0.6.0");
    assert!(repack(from.path(), from.path(), "0.6.0").is_err());
    assert!(from.path().join(CHECKSUMS_FILE).is_file());
}
