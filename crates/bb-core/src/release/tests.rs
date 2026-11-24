use bb_testing::release::TestKey;
use jiff::civil::date;

use super::*;

/// A file of the release 0.6.0 signed by `key`, with our trusted comment.
fn signed_file(key: &TestKey, bytes: &[u8]) -> String {
    let comment = TrustedComment {
        file: "bugaboo-0.6.0-linux-x64".to_owned(),
        version: "0.6.0".to_owned(),
    };
    key.sign(bytes, &comment.text()).unwrap()
}

/// `ReleaseKey`s for the public keys of test keys.
fn keys_of(publics: &[String]) -> Vec<ReleaseKey> {
    publics
        .iter()
        .map(|public| ReleaseKey {
            name: "test",
            public: public.clone().leak(),
        })
        .collect()
}

#[test]
fn sec_18_a_file_signed_by_the_backup_key_verifies_and_one_by_another_key_is_refused() {
    let (release, backup, other) = (
        TestKey::new().unwrap(),
        TestKey::new().unwrap(),
        TestKey::new().unwrap(),
    );
    let keys = keys_of(&[release.public(), backup.public()]);
    let bytes = b"the program";
    for key in [&release, &backup] {
        let comment = verify(bytes, &signed_file(key, bytes), &keys).unwrap();
        let read = TrustedComment::read(&comment).unwrap();
        assert_eq!(read.version, "0.6.0");
        assert_eq!(read.file, "bugaboo-0.6.0-linux-x64");
    }
    assert_eq!(
        verify(bytes, &signed_file(&other, bytes), &keys),
        Err(SignatureProblem::UnknownKey)
    );
    // A changed byte, and a signature that is not one at all.
    let signature = signed_file(&backup, bytes);
    assert_eq!(
        verify(b"the progran", &signature, &keys),
        Err(SignatureProblem::Changed)
    );
    assert_eq!(
        verify(bytes, "not a signature", &keys),
        Err(SignatureProblem::Unreadable)
    );
}

#[test]
fn sec_18_a_changed_trusted_comment_breaks_the_signature() {
    let key = TestKey::new().unwrap();
    let keys = keys_of(&[key.public()]);
    let bytes = b"the program";
    let signature = signed_file(&key, bytes);
    let older = signature.replace("version:0.6.0", "version:0.7.0");
    assert_ne!(older, signature);
    assert_eq!(verify(bytes, &older, &keys), Err(SignatureProblem::Changed));
}

/// A signature made by the minisign tool itself (the vector of the `minisign-verify` crate): the
/// four bytes `test`, signed with the hash of the file (the kind we sign with), and its public key.
const TOOL_PUBLIC_KEY: &str = "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3";
const TOOL_SIGNATURE: &str = "untrusted comment: signature from minisign secret key
RUQf6LRCGA9i559r3g7V1qNyJDApGip8MfqcadIgT9CuhV3EMhHoN1mGTkUidF/z7SrlQgXdy8ofjb7bNJJylDOocrCo8KLzZwo=
trusted comment: timestamp:1556193335\tfile:test
y/rUw2y8/hOUYjZU71eHp/Wo1KZ40fGy2VJEDl34XMJM+TX48Ss/17u3IvIfbVR1FkZZSNCisQbuQY+bHwhEBg==
";

#[test]
fn sec_18_our_signatures_read_in_the_minisign_format() {
    let tool = [ReleaseKey {
        name: "tool",
        public: TOOL_PUBLIC_KEY,
    }];
    // What the minisign tool signed, our check reads.
    let comment = verify(b"test", TOOL_SIGNATURE, &tool).unwrap();
    assert_eq!(comment, "timestamp:1556193335\tfile:test");
    assert_eq!(
        verify(b"Test", TOOL_SIGNATURE, &tool),
        Err(SignatureProblem::Changed)
    );
    // It names no version, so it is no signature of a release of ours.
    assert_eq!(TrustedComment::read(&comment), None);
    // What we sign has the four lines of the format, and the trusted comment in the tool's form.
    let key = TestKey::new().unwrap();
    let ours = signed_file(&key, b"test");
    let lines: Vec<&str> = ours.lines().collect();
    assert_eq!(lines.len(), 4, "{ours}");
    assert!(lines[0].starts_with("untrusted comment: "), "{ours}");
    assert!(
        lines[1].starts_with("RU"),
        "the hash of the file is signed: {ours}"
    );
    assert_eq!(
        lines[2],
        "trusted comment: file:bugaboo-0.6.0-linux-x64\tversion:0.6.0"
    );
}

#[test]
fn sec_18_the_keys_are_two_and_each_reads_as_a_minisign_key() {
    assert_eq!(RELEASE_PUBLIC_KEYS.len(), 2);
    for key in RELEASE_PUBLIC_KEYS {
        assert!(
            minisign_verify::PublicKey::from_base64(key.public).is_ok(),
            "{}",
            key.name
        );
    }
    assert_ne!(RELEASE_PUBLIC_KEYS[0].public, RELEASE_PUBLIC_KEYS[1].public);
}

#[test]
fn sec_18_a_checksum_is_found_by_the_name_of_its_file() {
    let sum = "ab".repeat(32);
    let other = "cd".repeat(32);
    let sums = format!(
        "{}{}{other} *bugaboo-0.6.0-windows-x64.exe\n",
        checksum_line(&sum, "bugaboo-0.6.0-linux-x64"),
        checksum_line(&other, "bugaboo-0.6.0-linux-x64.minisig"),
    );
    assert_eq!(
        checksum_of(&sums, "bugaboo-0.6.0-linux-x64").as_deref(),
        Some(sum.as_str())
    );
    assert_eq!(
        checksum_of(&sums, "bugaboo-0.6.0-windows-x64.exe").as_deref(),
        Some(other.as_str())
    );
    assert_eq!(checksum_of(&sums, "bugaboo-0.6.0-linux"), None);
    assert_eq!(checksum_of("short  bugaboo\n", "bugaboo"), None);
}

#[test]
fn sec_18_the_list_of_checksums_reads_as_its_entries_in_order() {
    let sum = "AB".repeat(32);
    let other = "cd".repeat(32);
    let sums = format!(
        "{}not a line\n{other} *bugaboo-0.6.0-windows-x64.exe\n",
        checksum_line(&sum, "bugaboo-0.6.0-linux-x64"),
    );
    assert_eq!(
        checksum_entries(&sums),
        [
            ("ab".repeat(32), "bugaboo-0.6.0-linux-x64".to_owned()),
            (other, "bugaboo-0.6.0-windows-x64.exe".to_owned()),
        ]
    );
}

#[test]
fn dist_07_versions_are_compared_by_their_three_numbers() {
    assert_eq!(is_older("0.5.1", "0.6.0"), Some(true));
    assert_eq!(is_older("0.6.0", "0.6.0"), Some(false));
    assert_eq!(is_older("0.10.0", "0.9.3"), Some(false));
    assert_eq!(is_older("1.0.0-test", "1.0.1"), Some(true));
    assert_eq!(is_older("1.0", "1.0.1"), None);
    assert_eq!(is_older("1.0.0.0", "1.0.1"), None);
    assert_eq!(is_older("+1.0.0", "1.0.1"), None);
}

#[test]
fn sec_18_a_pre_release_is_older_than_its_release_by_semantic_versioning() {
    assert_eq!(is_older("0.6.0-rc1", "0.6.0"), Some(true));
    assert_eq!(is_older("0.6.0", "0.6.0-rc1"), Some(false));
    assert_eq!(is_older("0.6.0-rc1", "0.6.0-rc1"), Some(false));
    // Part by part: numbers by value and below words, a shorter list below a longer one.
    assert_eq!(is_older("1.0.0-rc.2", "1.0.0-rc.10"), Some(true));
    assert_eq!(is_older("1.0.0-alpha", "1.0.0-beta"), Some(true));
    assert_eq!(is_older("1.0.0-1", "1.0.0-alpha"), Some(true));
    assert_eq!(is_older("1.0.0-alpha", "1.0.0-alpha.1"), Some(true));
    assert_eq!(is_older("1.0.0-beta", "1.0.0-alpha.1"), Some(false));
    // What follows a `+` does not count.
    assert_eq!(is_older("1.0.0+build.5", "1.0.0"), Some(false));
    assert_eq!(is_older("1.0.0-rc1+build", "1.0.0"), Some(true));
}

#[test]
fn comp_04_a_preview_is_supported_until_the_next_release_and_1_0_for_years() {
    let day = date(2026, 12, 1);
    assert_eq!(
        support_period("0.6.0", day),
        Some(SupportPeriod::UntilNextRelease)
    );
    assert_eq!(
        support_period("1.2.0", day),
        Some(SupportPeriod::Until(date(2026 + SUPPORT_YEARS, 12, 1)))
    );
    assert_eq!(support_period("one", day), None);
    assert!(this_support_period().is_some());
}

#[test]
fn comp_04_a_patch_release_is_supported_from_the_day_of_its_minor_release() {
    // 1.2.0 came out on 2026-12-01; 1.2.3 a year later gets no more time than 1.2.0.
    let minor = date(2026, 12, 1);
    assert_eq!(
        support_period("1.2.3", minor),
        support_period("1.2.0", minor)
    );
    // The two days of this build fit: X.Y.0 is never after this release, and is this release when
    // it is X.Y.0 itself.
    assert!(MINOR_RELEASE_DATE <= RELEASE_DATE);
    let (_, _, patch) = numbers(VERSION).unwrap();
    if patch == 0 {
        assert_eq!(MINOR_RELEASE_DATE, RELEASE_DATE);
    }
}

#[test]
fn comp_03_an_organization_contact_is_taken_only_as_one_address() {
    for (setting, address) in [
        ("security@example.org", Some("mailto:security@example.org")),
        (" mailto:a@example.org ", Some("mailto:a@example.org")),
        (
            "https://example.org/report",
            Some("https://example.org/report"),
        ),
        ("", None),
        ("http://example.org", None),
        ("https://", None),
        ("a@example.org\nPolicy: x", None),
        ("two words@example.org", None),
        ("javascript:a@example.org", None),
        ("nobody", None),
    ] {
        assert_eq!(contact_address(setting).as_deref(), address, "{setting:?}");
    }
}

#[test]
fn comp_03_security_txt_has_the_fields_of_rfc_9116() {
    let text = security_txt("security@example.org", date(2026, 12, 1));
    assert_eq!(
        text,
        format!(
            "Contact: mailto:security@example.org\nContact: {OUR_SECURITY_CONTACT}\n\
             Expires: 2027-12-01T00:00:00Z\nPolicy: {SECURITY_POLICY}\nPreferred-Languages: en, de\n"
        )
    );
    // Without a contact of the organization, ours is the only one.
    let ours = security_txt("", date(2026, 12, 1));
    assert!(ours.starts_with(&format!("Contact: {OUR_SECURITY_CONTACT}\n")));
}
