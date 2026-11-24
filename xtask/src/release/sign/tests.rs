use bb_core::release::{CHECKSUMS_FILE, RELEASE_PUBLIC_KEYS, ReleaseKey, signature_name};

use super::*;
use crate::release::folder::{payload, problems};
use crate::release::formats::cyclonedx_name;

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
    assert!(keygen(&key, &root).is_err());
    assert!(!key.exists(), "nothing is written where a key is refused");
    // A place outside both is taken.
    let outside = tempfile::tempdir().unwrap();
    refuse_inside_a_folder_of_work(&outside.path().join("release.key"), &root).unwrap();
}

/// A key folder reached through a link (on Windows a junction) is a place like any other: the
/// key is made and opened there.
#[cfg(windows)]
#[test]
fn sec_18_a_key_folder_behind_a_link_is_taken() {
    let root = super::super::repository_root();
    let place = tempfile::tempdir().unwrap();
    let target = place.path().join("keys");
    fs::create_dir(&target).unwrap();
    let link = place.path().join("settings");
    let made = std::process::Command::new("cmd.exe")
        .args(["/C", "mklink", "/J"])
        .arg(&link)
        .arg(&target)
        .output()
        .unwrap();
    assert!(made.status.success(), "{made:?}");
    let out = link.join("release.key");
    let public = keygen(&out, &root).unwrap();
    assert!(target.join("release.key").is_file());
    assert_eq!(public_of(&open_key(&out, &root).unwrap()).unwrap(), public);
}

#[test]
fn sec_18_the_secret_key_is_its_owner_s_alone() {
    let root = super::super::repository_root();
    let offline = tempfile::tempdir().unwrap();
    let out = offline.path().join("release.key");
    keygen(&out, &root).unwrap();
    assert!(!private::open_mode(0o600));
    assert!(private::open_mode(0o640));
    assert!(private::open_mode(0o604));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = |path: &Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&out), 0o600);
        open_key(&out, &root).unwrap();
        fs::set_permissions(&out, fs::Permissions::from_mode(0o644)).unwrap();
        let refused = open_key(&out, &root).unwrap_err().to_string();
        assert!(
            refused.contains("can be read by other accounts"),
            "{refused}"
        );
        assert!(refused.contains("chmod 600"), "{refused}");
    }
    #[cfg(windows)]
    {
        // Only the owner, nothing inherited from the folder.
        let owner = private::current_sid().unwrap();
        let sddl = private::access_list(&out).unwrap();
        let entries = bb_core::access_list::access_entries(&sddl).unwrap();
        assert!(!entries.is_empty(), "{sddl}");
        assert!(entries.iter().all(|entry| entry.who == owner), "{sddl}");
        assert!(sddl.contains("D:P"), "inheritance is off: {sddl}");
        open_key(&out, &root).unwrap();
        // The users of the computer may read it: refused, with the commands that close it.
        let granted = std::process::Command::new("icacls.exe")
            .arg(&out)
            .args(["/grant", "*S-1-5-32-545:R"])
            .output()
            .unwrap();
        assert!(granted.status.success());
        let refused = open_key(&out, &root).unwrap_err().to_string();
        assert!(
            refused.contains("can be read by other accounts"),
            "{refused}"
        );
        assert!(refused.contains("/inheritance:r"), "{refused}");
        // One other account is refused too, as with the mode `0600`; the system is not.
        for (sid, accepted) in [("S-1-5-18", true), ("S-1-5-19", false)] {
            let reset = std::process::Command::new("icacls.exe")
                .arg(&out)
                .args(["/remove:g", "*S-1-5-32-545"])
                .output()
                .unwrap();
            assert!(reset.status.success());
            let granted = std::process::Command::new("icacls.exe")
                .arg(&out)
                .args(["/grant", &format!("*{sid}:R")])
                .output()
                .unwrap();
            assert!(granted.status.success());
            assert_eq!(open_key(&out, &root).is_ok(), accepted, "{sid}");
            let removed = std::process::Command::new("icacls.exe")
                .arg(&out)
                .args(["/remove:g", &format!("*{sid}")])
                .output()
                .unwrap();
            assert!(removed.status.success());
        }
    }
}

#[test]
fn sec_18_a_damaged_key_without_a_passphrase_is_an_error_and_never_a_question() {
    let plain = KeyPair::generate_unencrypted_keypair()
        .unwrap()
        .sk
        .to_box(None)
        .unwrap()
        .into_string();
    let locked = KeyPair::generate_encrypted_keypair(Some("old passphrase".to_owned()))
        .unwrap()
        .sk
        .to_box(None)
        .unwrap()
        .into_string();
    assert!(!has_passphrase(&plain));
    assert!(has_passphrase(&locked));
    // One character of the secret part changed: the KDF field still says "no passphrase", so
    // reading it fails at once instead of asking on the console (which would wait forever here).
    let mut lines: Vec<String> = plain.lines().map(str::to_owned).collect();
    let key = &mut lines[1];
    let at = key.len() / 2;
    let changed = if &key[at..=at] == "A" { "B" } else { "A" };
    key.replace_range(at..=at, changed);
    let damaged = lines.join("\n");
    assert!(!has_passphrase(&damaged));
    assert!(key_of(&damaged, None).is_err());
    // Text that is no key at all is an error too.
    assert!(key_of("not a key", None).is_err());
}

#[test]
fn sec_18_keygen_writes_a_key_without_a_passphrase_that_signs_without_asking() {
    let root = super::super::repository_root();
    let offline = tempfile::tempdir().unwrap();
    let out = offline.path().join("release.key");
    let public = keygen(&out, &root).unwrap();
    // The key is not encrypted, so reading it asks nothing: a prompt would wait for the console
    // and this test would never end.
    let text = fs::read_to_string(&out).unwrap();
    SecretKey::from_unencrypted_box(text.clone().into()).unwrap();
    let key = open_key(&out, &root).unwrap();
    assert_eq!(public_of(&key).unwrap(), public);
    // What it signs verifies against the public key it printed.
    let signature = minisign::sign(None, &key, &b"a file"[..], None, None).unwrap();
    let printed = PublicKey::from_base64(&public).unwrap();
    minisign::verify(
        &printed,
        &signature,
        std::io::Cursor::new(b"a file"),
        true,
        false,
        false,
    )
    .unwrap();
}

#[test]
fn sec_18_an_older_key_with_a_passphrase_still_opens_with_it() {
    let pair = KeyPair::generate_encrypted_keypair(Some("old passphrase".to_owned())).unwrap();
    let text = pair.sk.to_box(None).unwrap().into_string();
    assert!(SecretKey::from_unencrypted_box(text.clone().into()).is_err());
    let key = key_of(&text, Some("old passphrase".to_owned())).unwrap();
    assert_eq!(public_of(&key).unwrap(), pair.pk.to_base64());
    assert!(key_of(&text, Some("wrong".to_owned())).is_err());
}

/// A release folder of `version` with a small stand-in for every program and the notices, and
/// the parts list of this repository with its VEX statements.
fn release_folder(version: &str) -> tempfile::TempDir {
    let folder = tempfile::tempdir().unwrap();
    for name in payload(version) {
        fs::write(folder.path().join(&name), format!("the file {name}")).unwrap();
    }
    let parts = crate::release::to_text(&crate::release::this_repository().cyclonedx);
    fs::write(folder.path().join(cyclonedx_name(version)), parts).unwrap();
    folder
}

/// A key pair made for the test, and the list of keys that holds its public key.
fn test_key() -> (SecretKey, [ReleaseKey; 1]) {
    let pair = KeyPair::generate_unencrypted_keypair().unwrap();
    let keys = [ReleaseKey {
        name: "release",
        public: pair.pk.to_base64().leak(),
    }];
    (pair.sk, keys)
}

#[test]
fn sec_18_keygen_writes_a_key_that_signs() {
    let root = super::super::repository_root();
    let offline = tempfile::tempdir().unwrap();
    let out = offline.path().join("release.key");
    let public = keygen(&out, &root).unwrap();
    assert!(offline.path().join("release.key.pub").is_file());
    // A key is never written over.
    assert!(keygen(&out, &root).is_err());
    let key = open_key(&out, &root).unwrap();
    assert_eq!(public_of(&key).unwrap(), public);
    let keys = [ReleaseKey {
        name: "release",
        public: public.leak(),
    }];
    let folder = release_folder("0.6.0");
    sign_folder(folder.path(), "0.6.0", &key).unwrap();
    assert_eq!(
        problems(folder.path(), "0.6.0", &keys),
        Vec::<String>::new()
    );
}

#[test]
fn sec_18_the_release_folder_holds_six_files_and_one_signature() {
    let (key, keys) = test_key();
    let folder = release_folder("0.6.0");
    // Before signing, the list of checksums and its signature are missing.
    let unsigned = problems(folder.path(), "0.6.0", &keys);
    assert_eq!(unsigned, ["SHA256SUMS is missing."], "{unsigned:?}");

    sign_folder(folder.path(), "0.6.0", &key).unwrap();
    assert_eq!(
        problems(folder.path(), "0.6.0", &keys),
        Vec::<String>::new()
    );
    let mut names: Vec<String> = fs::read_dir(folder.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "SHA256SUMS",
            "SHA256SUMS.minisig",
            "bugaboo-0.6.0-linux-x64",
            "bugaboo-0.6.0-notices.txt",
            "bugaboo-0.6.0-windows-x64.exe",
            "bugaboo-0.6.0.cdx.json",
        ]
    );
    // `SHA256SUMS` lists the four other files, in the order of the payload.
    let sums = fs::read_to_string(folder.path().join(CHECKSUMS_FILE)).unwrap();
    let listed: Vec<String> = bb_core::release::checksum_entries(&sums)
        .into_iter()
        .map(|(_, name)| name)
        .collect();
    assert_eq!(listed, payload("0.6.0"));
    // Another key does not count.
    let (_, other) = test_key();
    assert_eq!(
        problems(folder.path(), "0.6.0", &other),
        ["SHA256SUMS.minisig does not hold: UnknownKey."]
    );
    // A file changed after signing is found; so is a signature that is gone.
    let program = &payload("0.6.0")[0];
    fs::write(folder.path().join(program), "another program").unwrap();
    fs::remove_file(folder.path().join(signature_name(CHECKSUMS_FILE))).unwrap();
    assert_eq!(
        problems(folder.path(), "0.6.0", &keys),
        [
            format!("{program} has no entry in SHA256SUMS with its checksum."),
            "SHA256SUMS.minisig is missing.".to_owned(),
        ]
    );
    // A folder signed as another version is refused.
    let renamed = release_folder("0.6.0");
    sign_folder(renamed.path(), "0.6.0", &key).unwrap();
    assert!(!problems(renamed.path(), "0.6.1", &keys).is_empty());
}

#[test]
fn sec_18_sha256sums_names_the_payload_and_nothing_else() {
    let (key, keys) = test_key();
    let folder = release_folder("0.6.0");
    sign_folder(folder.path(), "0.6.0", &key).unwrap();
    let signed = fs::read_to_string(folder.path().join(CHECKSUMS_FILE)).unwrap();
    // The list `sums`, signed anew.
    let resign = |sums: &str| {
        fs::write(folder.path().join(CHECKSUMS_FILE), sums).unwrap();
        let signature = minisign::sign(
            None,
            &key,
            sums.as_bytes(),
            Some(&folder::trusted_comment(CHECKSUMS_FILE, "0.6.0")),
            None,
        )
        .unwrap();
        fs::write(
            folder.path().join(signature_name(CHECKSUMS_FILE)),
            signature.into_string(),
        )
        .unwrap();
    };
    // A list that names one more file is refused, and so is the file.
    let extra = bb_core::release::checksum_line(&"ab".repeat(32), "bugaboo-0.6.0.spdx.json");
    resign(&format!("{signed}{extra}"));
    fs::write(folder.path().join("bugaboo-0.6.0.spdx.json"), "{}").unwrap();
    assert_eq!(
        problems(folder.path(), "0.6.0", &keys),
        [
            "SHA256SUMS lists bugaboo-0.6.0.spdx.json, which is not a file of the release.",
            "bugaboo-0.6.0.spdx.json does not belong in the release folder.",
        ]
    );
    // A line that is not an entry is refused too.
    fs::remove_file(folder.path().join("bugaboo-0.6.0.spdx.json")).unwrap();
    resign(&format!("{signed}something else\n"));
    assert_eq!(
        problems(folder.path(), "0.6.0", &keys),
        ["SHA256SUMS holds a line that is not a checksum and a name."]
    );
}

#[test]
fn sec_18_the_parts_list_of_a_release_carries_its_vex_statements() {
    let (key, keys) = test_key();
    let folder = release_folder("0.6.0");
    let name = cyclonedx_name("0.6.0");
    let mut document = crate::release::this_repository().cyclonedx.clone();
    document.as_object_mut().unwrap().remove("vulnerabilities");
    fs::write(
        folder.path().join(&name),
        crate::release::to_text(&document),
    )
    .unwrap();
    sign_folder(folder.path(), "0.6.0", &key).unwrap();
    assert_eq!(
        problems(folder.path(), "0.6.0", &keys),
        [format!(
            "{name} carries no VEX statements (no `vulnerabilities`)."
        )]
    );
    // A statement without its analysis, and one the schema does not take.
    document["vulnerabilities"] = serde_json::json!([
        {"id": "RUSTSEC-2023-0001"},
        {"id": "RUSTSEC-2023-0002", "analysis": {"state": "fine"}},
    ]);
    fs::write(
        folder.path().join(&name),
        crate::release::to_text(&document),
    )
    .unwrap();
    sign_folder(folder.path(), "0.6.0", &key).unwrap();
    let found = problems(folder.path(), "0.6.0", &keys);
    assert!(
        found.contains(&format!(
            "{name}: the statement on RUSTSEC-2023-0001 has no analysis."
        )),
        "{found:?}"
    );
    assert!(
        found
            .iter()
            .any(|problem| problem.starts_with(&format!("{name} does not fit its schema"))),
        "{found:?}"
    );
}

#[test]
fn sec_18_a_file_that_does_not_belong_in_the_release_folder_is_named() {
    let (key, keys) = test_key();
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
