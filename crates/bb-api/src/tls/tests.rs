//! The tests of tls.

use super::*;

#[test]
fn sec_10_dates_round_trip() {
    for days in [-1, 0, 59, 60, 365, 11_016, 20_000, 30_000] {
        let (y, m, d) = civil_from_days(days);
        assert_eq!(
            days_from_civil(i64::from(y), i64::from(m), i64::from(d)),
            days
        );
    }
    assert_eq!(civil_from_days(0), (1970, 1, 1));
    assert_eq!(civil_from_days(20_000), (2024, 10, 4));
}

#[test]
fn sec_10_a_generated_certificate_reports_its_end() {
    let now = bb_dev_words::accounts::CLOCK_START;
    let (pem, _) = generate(&wanted_names(&[]), now).unwrap();
    let der = CertificateDer::from_pem_slice(pem.as_bytes()).unwrap();
    let end = not_after(&der).unwrap();
    let expected = now + VALID_DAYS * SECONDS_PER_DAY;
    assert!(
        (end - expected).abs() < SECONDS_PER_DAY,
        "{end} vs {expected}"
    );
}

#[test]
fn ops_08_the_end_of_a_certificate_file_is_read_for_the_status_page() {
    let folder = tempfile::tempdir().unwrap();
    let now = now_seconds();
    ensure_self_signed(folder.path(), &[], now).unwrap();
    let end = certificate_end(&folder.path().join(CERT_FILE)).unwrap();
    assert!((end - (now + VALID_DAYS * SECONDS_PER_DAY)).abs() < 2 * SECONDS_PER_DAY);
    assert_eq!(certificate_end(&folder.path().join(KEY_FILE)), None);
    assert_eq!(certificate_end(&folder.path().join("none.pem")), None);
}

#[test]
fn sec_10_self_signed_is_kept_renewed_and_follows_the_names() {
    let folder = tempfile::tempdir().unwrap();
    let now = now_seconds();
    let read = || fs::read_to_string(folder.path().join(CERT_FILE)).unwrap();

    ensure_self_signed(folder.path(), &[], now).unwrap();
    let first = read();
    ensure_self_signed(folder.path(), &[], now + 10 * SECONDS_PER_DAY).unwrap();
    assert_eq!(read(), first, "still fresh: kept");

    let late = now + (VALID_DAYS - RENEW_BEFORE_DAYS + 1) * SECONDS_PER_DAY;
    ensure_self_signed(folder.path(), &[], late).unwrap();
    let renewed = read();
    assert_ne!(renewed, first, "close to the end: renewed");

    ensure_self_signed(folder.path(), &["bugaboo.example.test".to_owned()], late).unwrap();
    assert_ne!(read(), renewed, "other names: new certificate");
}

#[test]
fn sec_10_server_config_loads_generated_and_provided_files() {
    let folder = tempfile::tempdir().unwrap();
    let source = TlsSource::SelfSigned {
        folder: folder.path().to_path_buf(),
        names: vec![],
    };
    let config = server_config(&source).unwrap();
    assert_eq!(config.alpn_protocols.len(), 2);
    let files = TlsSource::Files {
        cert: folder.path().join(CERT_FILE),
        key: folder.path().join(KEY_FILE),
    };
    assert!(server_config(&files).is_ok());
}

#[test]
fn sec_10_bad_files_are_reported_with_the_file() {
    let folder = tempfile::tempdir().unwrap();
    let missing = TlsSource::Files {
        cert: folder.path().join("none.pem"),
        key: folder.path().join("none-key.pem"),
    };
    assert!(matches!(
        server_config(&missing),
        Err(TlsError::Read { .. })
    ));

    let empty = folder.path().join("empty.pem");
    fs::write(&empty, "not a certificate").unwrap();
    let junk = TlsSource::Files {
        cert: empty.clone(),
        key: empty,
    };
    assert!(matches!(
        server_config(&junk),
        Err(TlsError::NoCertificate { .. })
    ));
}

#[test]
fn sec_10_a_self_signed_pair_that_does_not_match_is_replaced() {
    let folder = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let now = now_seconds();
    ensure_self_signed(folder.path(), &[], now).unwrap();
    ensure_self_signed(other.path(), &[], now).unwrap();
    fs::copy(other.path().join(KEY_FILE), folder.path().join(KEY_FILE)).unwrap();
    assert!(
        load_certified(
            &certificate_path(folder.path()),
            &folder.path().join(KEY_FILE)
        )
        .is_err()
    );
    ensure_self_signed(folder.path(), &[], now).unwrap();
    assert!(
        load_certified(
            &certificate_path(folder.path()),
            &folder.path().join(KEY_FILE)
        )
        .is_ok()
    );
    assert!(!folder.path().join("self-signed-key.pem.new").exists());
}

#[cfg(unix)]
#[test]
fn sec_10_kept_files_become_owner_only_again() {
    use std::os::unix::fs::PermissionsExt as _;
    let folder = tempfile::tempdir().unwrap();
    let now = now_seconds();
    ensure_self_signed(folder.path(), &[], now).unwrap();
    let key = folder.path().join(KEY_FILE);
    fs::set_permissions(&key, fs::Permissions::from_mode(0o644)).unwrap();
    ensure_self_signed(folder.path(), &[], now).unwrap();
    assert_eq!(
        fs::metadata(&key).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn sec_10_a_key_that_does_not_match_is_unusable() {
    let one = tempfile::tempdir().unwrap();
    let two = tempfile::tempdir().unwrap();
    for folder in [&one, &two] {
        server_config(&TlsSource::SelfSigned {
            folder: folder.path().to_path_buf(),
            names: vec![],
        })
        .unwrap();
    }
    let mixed = TlsSource::Files {
        cert: one.path().join(CERT_FILE),
        key: two.path().join(KEY_FILE),
    };
    assert!(matches!(
        server_config(&mixed),
        Err(TlsError::Unusable { .. })
    ));
}

fn self_signed(folder: &Path) -> ServedCertificate {
    serve_certificate(&TlsSource::SelfSigned {
        folder: folder.to_path_buf(),
        names: vec![],
    })
    .unwrap()
}

#[test]
fn sec_10_a_certificate_near_its_end_is_renewed_in_place_and_the_end_moves() {
    let folder = tempfile::tempdir().unwrap();
    let served = self_signed(folder.path());
    let settings = served.config();
    let first = served.expires_at().unwrap();
    let now = now_seconds();
    assert_eq!(served.renew(now).unwrap(), Renewal::Kept);
    assert_eq!(served.expires_at(), Some(first));

    // Eleven months later less than 30 days are left: a new certificate is made and served.
    let late = now + (VALID_DAYS - RENEW_BEFORE_DAYS + 1) * SECONDS_PER_DAY;
    assert_eq!(served.renew(late).unwrap(), Renewal::Renewed);
    let renewed = served.expires_at().unwrap();
    assert!(
        renewed > first + 300 * SECONDS_PER_DAY,
        "{first} -> {renewed}"
    );
    assert_eq!(
        renewed,
        certificate_end(&certificate_path(folder.path())).unwrap()
    );
    assert!(
        Arc::ptr_eq(&settings, &served.config()),
        "the settings stay one object"
    );
    // Looking again changes nothing.
    assert_eq!(served.renew(late).unwrap(), Renewal::Kept);
}

/// Writes a new certificate and key as the files an owner would provide.
fn provide(folder: &Path, name: &str, now: i64) -> (PathBuf, PathBuf) {
    let (cert, key) = generate(&wanted_names(&[]), now).unwrap();
    let (cert_path, key_path) = (
        folder.join(format!("{name}-cert.pem")),
        folder.join(format!("{name}-key.pem")),
    );
    fs::write(&cert_path, cert).unwrap();
    fs::write(&key_path, key).unwrap();
    (cert_path, key_path)
}

#[test]
fn sec_10_an_own_certificate_is_read_again_when_the_files_change() {
    let folder = tempfile::tempdir().unwrap();
    let now = now_seconds();
    let (cert, key) = provide(folder.path(), "own", now);
    let served = serve_certificate(&TlsSource::Files {
        cert: cert.clone(),
        key: key.clone(),
    })
    .unwrap();
    let first = served.expires_at().unwrap();
    assert_eq!(served.renew(now).unwrap(), Renewal::Kept);

    // The owner puts in a certificate that is valid for longer.
    let (cert_pem, key_pem) = generate(&wanted_names(&[]), now + 100 * SECONDS_PER_DAY).unwrap();
    fs::write(&cert, &cert_pem).unwrap();
    fs::write(&key, &key_pem).unwrap();
    assert_eq!(served.renew(now).unwrap(), Renewal::Reloaded);
    assert!(served.expires_at().unwrap() > first + 90 * SECONDS_PER_DAY);

    // Files that cannot be used (a certificate that has no matching key, half a copy) leave
    // the certificate in use alone and say what is wrong; the next good pair is picked up.
    let in_use = served.expires_at();
    let (other_cert, _) = provide(folder.path(), "other", now);
    fs::copy(&other_cert, &cert).unwrap();
    assert!(matches!(served.renew(now), Err(TlsError::Unusable { .. })));
    fs::write(&cert, "not a certificate").unwrap();
    assert!(matches!(
        served.renew(now),
        Err(TlsError::NoCertificate { .. })
    ));
    assert_eq!(served.expires_at(), in_use);
    fs::write(&cert, &cert_pem).unwrap();
    assert_eq!(served.renew(now).unwrap(), Renewal::Kept);
}

#[test]
fn sec_10_a_changed_chain_with_the_same_first_certificate_is_served() {
    let folder = tempfile::tempdir().unwrap();
    let now = now_seconds();
    let (cert, key) = provide(folder.path(), "own", now);
    let served = serve_certificate(&TlsSource::Files {
        cert: cert.clone(),
        key: key.clone(),
    })
    .unwrap();
    assert_eq!(served.renew(now).unwrap(), Renewal::Kept);
    // The owner adds an intermediate certificate behind the first one; the first is the same.
    let leaf = fs::read_to_string(&cert).unwrap();
    let (other, _) = generate(&wanted_names(&[]), now).unwrap();
    fs::write(&cert, format!("{leaf}{other}")).unwrap();
    assert_eq!(served.renew(now).unwrap(), Renewal::Reloaded);
    assert_eq!(served.renew(now).unwrap(), Renewal::Kept);
}
