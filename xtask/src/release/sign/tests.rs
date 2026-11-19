use bb_core::release::{CHECKSUMS_FILE, RELEASE_PUBLIC_KEYS, ReleaseKey, signature_name};

use super::*;
use crate::release::folder::{payload, problems, signed_files};

/// A Git working tree in a temporary folder, standing for a work folder.
fn work_folder() -> tempfile::TempDir {
    let folder = tempfile::tempdir().unwrap();
    fs::create_dir(folder.path().join(".git")).unwrap();
    folder
}

#[test]
fn sec_18_signing_refuses_a_key_inside_the_repository_or_a_work_folder() {
    let root = super::super::repository_root();
    // Inside this repository, also through `..`, and in a folder that does not exist yet.
    for inside in [
        root.join("target/release.key"),
        root.join("crates/../keys/new/release.key"),
    ] {
        let refused = refuse_inside_a_folder_of_work(&inside, &root).unwrap_err();
        assert!(
            refused.to_string().contains("outside the repository"),
            "{refused}"
        );
    }
    // Inside another work folder (any Git working tree).
    let other = work_folder();
    let key = other.path().join("deep/release.key");
    assert!(refuse_inside_a_folder_of_work(&key, &root).is_err());
    assert!(keygen(&key, &root, Some("passphrase".to_owned())).is_err());
    assert!(!key.exists(), "nothing is written where a key is refused");
    // An offline place outside both is taken.
    let outside = tempfile::tempdir().unwrap();
    refuse_inside_a_folder_of_work(&outside.path().join("release.key"), &root).unwrap();
}

/// A release folder of `version` with a small stand-in for every file.
fn release_folder(version: &str) -> tempfile::TempDir {
    let folder = tempfile::tempdir().unwrap();
    for name in payload(version) {
        fs::write(folder.path().join(&name), format!("the file {name}")).unwrap();
    }
    folder
}

#[test]
fn sec_18_the_release_folder_holds_every_file_and_a_signature_for_each() {
    let root = super::super::repository_root();
    let offline = tempfile::tempdir().unwrap();
    let out = offline.path().join("release.key");
    let public = keygen(&out, &root, Some("passphrase".to_owned())).unwrap();
    assert!(offline.path().join("release.key.pub").is_file());
    // A key is never written over.
    assert!(keygen(&out, &root, Some("other".to_owned())).is_err());
    let key = SecretKey::from_file(&out, Some("passphrase".to_owned())).unwrap();
    assert_eq!(public_of(&key).unwrap(), public);
    let keys = [ReleaseKey {
        name: "release",
        public: public.clone().leak(),
    }];

    let folder = release_folder("0.6.0");
    // Before signing, every signature and the list of checksums are missing.
    let unsigned = problems(folder.path(), "0.6.0", &keys);
    assert!(
        unsigned
            .iter()
            .any(|problem| problem.contains(CHECKSUMS_FILE)),
        "{unsigned:?}"
    );
    assert!(
        unsigned.len() >= signed_files("0.6.0").len(),
        "{unsigned:?}"
    );

    sign_folder(folder.path(), "0.6.0", &key).unwrap();
    assert_eq!(
        problems(folder.path(), "0.6.0", &keys),
        Vec::<String>::new()
    );
    for name in signed_files("0.6.0") {
        assert!(
            folder.path().join(signature_name(&name)).is_file(),
            "{name}"
        );
    }
    // The parts lists, the notices, the VEX statements and the notes are among the signed files.
    for end in [
        ".cdx.json",
        ".spdx.json",
        "-notices.txt",
        ".vex.json",
        "-notes.md",
    ] {
        assert!(
            payload("0.6.0").iter().any(|name| name.ends_with(end)),
            "{end}"
        );
    }
    // Another key does not count.
    let other = [ReleaseKey {
        name: "other",
        public: "RWRK0RL2nc9pmFKXiIWQtp+OIHiivdKULp6A7zjkGeiA/c8MMkyBZ5XD",
    }];
    assert!(!problems(folder.path(), "0.6.0", &other).is_empty());
    // A file changed after signing, and a signature that is gone, are found.
    let program = &payload("0.6.0")[0];
    fs::write(folder.path().join(program), "another program").unwrap();
    fs::remove_file(folder.path().join(signature_name(&payload("0.6.0")[2]))).unwrap();
    let found = problems(folder.path(), "0.6.0", &keys);
    assert!(
        found
            .iter()
            .any(|problem| problem.starts_with(program.as_str())),
        "{found:?}"
    );
    assert!(
        found.iter().any(|problem| problem.contains("is missing")),
        "{found:?}"
    );
    // A folder signed as another version is refused.
    let renamed = release_folder("0.6.0");
    sign_folder(renamed.path(), "0.6.0", &key).unwrap();
    assert!(!problems(renamed.path(), "0.6.1", &keys).is_empty());
}

#[test]
fn sec_18_a_file_that_does_not_belong_in_the_release_folder_is_named() {
    let offline = tempfile::tempdir().unwrap();
    let out = offline.path().join("release.key");
    let public = keygen(&out, &super::super::repository_root(), Some("p".to_owned())).unwrap();
    let key = SecretKey::from_file(&out, Some("p".to_owned())).unwrap();
    let keys = [ReleaseKey {
        name: "release",
        public: public.leak(),
    }];
    let folder = release_folder("0.6.0-test");
    sign_folder(folder.path(), "0.6.0-test", &key).unwrap();
    assert_eq!(
        problems(folder.path(), "0.6.0-test", &keys),
        Vec::<String>::new()
    );
    fs::write(folder.path().join("notes-draft.md"), "left behind").unwrap();
    fs::create_dir(folder.path().join("old")).unwrap();
    assert_eq!(
        problems(folder.path(), "0.6.0-test", &keys),
        [
            "notes-draft.md does not belong in the release folder.",
            "old does not belong in the release folder.",
        ]
    );
}

#[test]
fn sec_18_signing_with_real_keys_needs_the_tag_and_one_of_the_keys() {
    let ours = RELEASE_PUBLIC_KEYS[0].public;
    let other = "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3";
    for (tagged, test_keys, public, refused) in [
        // While the program trusts only test keys, every build is a marked test build.
        (false, true, other, None),
        (true, true, other, None),
        // With the real keys: only from the tag, and only with one of them.
        (true, false, ours, None),
        (false, false, ours, Some("is not on this commit")),
        (
            true,
            false,
            other,
            Some("is not one of RELEASE_PUBLIC_KEYS"),
        ),
    ] {
        let build = Build { tagged, test_keys };
        let found = tag_refusal(build, "0.6.0").or_else(|| key_refusal(build, public));
        match refused {
            None => assert_eq!(found, None, "{tagged} {test_keys} {public}"),
            Some(words) => assert!(
                found.as_deref().is_some_and(|text| text.contains(words)),
                "{found:?}"
            ),
        }
    }
}
