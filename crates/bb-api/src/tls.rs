//! HTTPS certificates (SEC-10): a self-signed certificate made on first start and renewed before
//! it expires, or a certificate and key the owner provides. rustls with the `ring` provider.
//!
//! This module reports problems as [`TlsError`] values; the program turns them into plain
//! sentences from the message catalog.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, date_time_ymd};
use rustls::ServerConfig;
use rustls::crypto::ring::default_provider;
use rustls::pki_types::pem::PemObject as _;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};

/// File names inside the data folder's `tls/` folder.
const CERT_FILE: &str = "self-signed-cert.pem";
const KEY_FILE: &str = "self-signed-key.pem";
/// The names the certificate was made for, one per line; a change makes a new certificate.
const NAMES_FILE: &str = "self-signed-names.txt";
/// How long a generated certificate is valid, and how long before its end it is renewed.
const VALID_DAYS: i64 = 365;
const RENEW_BEFORE_DAYS: i64 = 30;
const SECONDS_PER_DAY: i64 = 86_400;
/// Names every generated certificate is valid for: this computer.
const LOCAL_NAMES: [&str; 3] = ["localhost", "127.0.0.1", "::1"];
const COMMON_NAME: &str = "Bugaboo";

/// Where the certificate and key come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TlsSource {
    /// Made and renewed by the server, kept in `folder`. `names` are added to the local names.
    SelfSigned {
        /// The data folder's `tls/` folder.
        folder: PathBuf,
        /// Extra host names or addresses the certificate must be valid for.
        names: Vec<String>,
    },
    /// Certificate chain and private key supplied by the owner (PEM files).
    Files {
        /// The certificate chain file.
        cert: PathBuf,
        /// The private key file.
        key: PathBuf,
    },
}

/// What went wrong with a certificate. `path` is the file concerned.
#[derive(Debug)]
pub enum TlsError {
    /// A file could not be read.
    Read {
        /// The file.
        path: PathBuf,
        /// The reason from the operating system.
        source: io::Error,
    },
    /// A file could not be written.
    Write {
        /// The file.
        path: PathBuf,
        /// The reason from the operating system.
        source: io::Error,
    },
    /// The certificate file holds no certificate.
    NoCertificate {
        /// The file.
        path: PathBuf,
    },
    /// The key file holds no private key.
    NoKey {
        /// The file.
        path: PathBuf,
    },
    /// The certificate and key were read but do not work together or are not usable.
    Unusable {
        /// The reason from the TLS library.
        reason: String,
    },
    /// A new certificate could not be made (for example an invalid host name).
    Generate {
        /// The reason from the certificate library.
        reason: String,
    },
}

impl fmt::Display for TlsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } | Self::Write { path, source } => {
                write!(f, "{}: {source}", path.display())
            }
            Self::NoCertificate { path } | Self::NoKey { path } => {
                write!(f, "{}: nothing usable inside", path.display())
            }
            Self::Unusable { reason } | Self::Generate { reason } => f.write_str(reason),
        }
    }
}

impl std::error::Error for TlsError {}

/// The certificate file a self-signed setup keeps in `folder` (what people can trust).
#[must_use]
pub fn certificate_path(folder: &Path) -> PathBuf {
    folder.join(CERT_FILE)
}

/// The server-side TLS settings for `source`. A self-signed certificate is made (or renewed)
/// here if needed.
///
/// # Errors
/// Returns what is wrong with the files, or why a certificate could not be made.
pub fn server_config(source: &TlsSource) -> Result<Arc<ServerConfig>, TlsError> {
    let (cert, key) = match source {
        TlsSource::SelfSigned { folder, names } => {
            ensure_self_signed(folder, names, now_seconds())?;
            (folder.join(CERT_FILE), folder.join(KEY_FILE))
        }
        TlsSource::Files { cert, key } => (cert.clone(), key.clone()),
    };
    let chain = CertificateDer::pem_file_iter(&cert)
        .map_err(|error| pem_error(&cert, error, false))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| pem_error(&cert, error, false))?;
    if chain.is_empty() {
        return Err(TlsError::NoCertificate { path: cert });
    }
    let key_der =
        PrivateKeyDer::from_pem_file(&key).map_err(|error| pem_error(&key, error, true))?;
    let unusable = |error: rustls::Error| TlsError::Unusable {
        reason: error.to_string(),
    };
    let mut config = ServerConfig::builder_with_provider(Arc::new(default_provider()))
        .with_safe_default_protocol_versions()
        .map_err(unusable)?
        .with_no_client_auth()
        .with_single_cert(chain, key_der)
        .map_err(unusable)?;
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(Arc::new(config))
}

fn pem_error(path: &Path, error: rustls::pki_types::pem::Error, key: bool) -> TlsError {
    use rustls::pki_types::pem::Error;
    let path = path.to_path_buf();
    match error {
        Error::Io(source) => TlsError::Read { path, source },
        _ if key => TlsError::NoKey { path },
        _ => TlsError::NoCertificate { path },
    }
}

/// Makes sure `folder` holds a certificate that is valid for the wanted names and will still
/// be valid in `RENEW_BEFORE_DAYS` days; otherwise writes a new one (and a new key).
fn ensure_self_signed(folder: &Path, extra: &[String], now: i64) -> Result<(), TlsError> {
    let wanted = wanted_names(extra);
    let cert_path = folder.join(CERT_FILE);
    let end = fs::read_to_string(&cert_path).ok().and_then(|pem| {
        let der = CertificateDer::from_pem_slice(pem.as_bytes()).ok()?;
        not_after(&der)
    });
    let same_names = fs::read_to_string(folder.join(NAMES_FILE))
        .is_ok_and(|text| text.lines().eq(wanted.iter().map(String::as_str)));
    let fresh = end.is_some_and(|end| end - now > RENEW_BEFORE_DAYS * SECONDS_PER_DAY);
    if fresh && same_names && folder.join(KEY_FILE).is_file() {
        return Ok(());
    }
    let (cert_pem, key_pem) = generate(&wanted, now)?;
    write_private(&folder.join(KEY_FILE), key_pem.as_bytes())?;
    write_private(&cert_path, cert_pem.as_bytes())?;
    write_private(&folder.join(NAMES_FILE), wanted.join("\n").as_bytes())?;
    Ok(())
}

fn wanted_names(extra: &[String]) -> Vec<String> {
    let mut names: Vec<String> = LOCAL_NAMES.iter().map(|name| (*name).to_owned()).collect();
    for name in extra {
        if !names.contains(name) {
            names.push(name.clone());
        }
    }
    names
}

/// A new certificate and its key as PEM text, valid from yesterday (so a computer whose clock
/// is a little behind still accepts it) for `VALID_DAYS` days.
fn generate(names: &[String], now: i64) -> Result<(String, String), TlsError> {
    let failed = |error: rcgen::Error| TlsError::Generate {
        reason: error.to_string(),
    };
    let today = now.div_euclid(SECONDS_PER_DAY);
    let date = |days| {
        let (year, month, day) = civil_from_days(days);
        date_time_ymd(year, month, day)
    };
    let mut params = CertificateParams::new(names.to_vec()).map_err(failed)?;
    let mut subject = DistinguishedName::new();
    subject.push(DnType::CommonName, COMMON_NAME);
    params.distinguished_name = subject;
    params.not_before = date(today - 1);
    params.not_after = date(today + VALID_DAYS);
    let key = KeyPair::generate().map_err(failed)?;
    let cert = params.self_signed(&key).map_err(failed)?;
    Ok((cert.pem(), key.serialize_pem()))
}

/// Writes a secret file that only the owner can read (architecture §6).
fn write_private(path: &Path, bytes: &[u8]) -> Result<(), TlsError> {
    let failed = |source| TlsError::Write {
        path: path.to_path_buf(),
        source,
    };
    #[cfg(unix)]
    {
        use std::io::Write as _;
        use std::os::unix::fs::OpenOptionsExt as _;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)
            .map_err(failed)?;
        file.write_all(bytes).map_err(failed)
    }
    // On Windows the `tls` folder carries an access list for its owner only, which new files
    // inherit (the program sets it up with the rest of the data folder).
    #[cfg(not(unix))]
    {
        fs::write(path, bytes).map_err(failed)
    }
}

fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX)
        })
}

/// Days since 1970-01-01 to (year, month, day), proleptic Gregorian (Howard Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i32, u8, u8) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (
        i32::try_from(year).unwrap_or(i32::MAX),
        u8::try_from(month).unwrap_or(1),
        u8::try_from(day).unwrap_or(1),
    )
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let shifted_month = (month + 9) % 12;
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// One DER element: its tag, its content, and what follows it.
fn read_der(data: &[u8]) -> Option<(u8, &[u8], &[u8])> {
    let (&tag, rest) = data.split_first()?;
    let (&first, rest) = rest.split_first()?;
    let (length, rest) = if first < 0x80 {
        (usize::from(first), rest)
    } else {
        let count = usize::from(first & 0x7f);
        if count == 0 || count > 4 || rest.len() < count {
            return None;
        }
        let length = rest[..count]
            .iter()
            .fold(0_usize, |sum, &byte| (sum << 8) | usize::from(byte));
        (length, &rest[count..])
    };
    (rest.len() >= length).then(|| (tag, &rest[..length], &rest[length..]))
}

/// The end of a certificate's validity ("not after"), in seconds since 1970.
fn not_after(cert: &[u8]) -> Option<i64> {
    let (_, certificate, _) = read_der(cert)?;
    let (_, tbs, _) = read_der(certificate)?;
    let (tag, _, mut rest) = read_der(tbs)?;
    // The version (context tag 0) is optional; then come serial, signature and issuer.
    let skip = if tag == 0xA0 { 3 } else { 2 };
    for _ in 0..skip {
        rest = read_der(rest)?.2;
    }
    let (_, validity, _) = read_der(rest)?;
    let (_, _, after_start) = read_der(validity)?;
    let (tag, end, _) = read_der(after_start)?;
    parse_time(tag, end)
}

/// A DER `UTCTime` (tag 0x17) or `GeneralizedTime` (tag 0x18) in UTC.
fn parse_time(tag: u8, text: &[u8]) -> Option<i64> {
    let text = std::str::from_utf8(text).ok()?.strip_suffix('Z')?;
    let (year, rest) = match tag {
        0x17 => {
            let two: i64 = text.get(..2)?.parse().ok()?;
            let year = if two >= 50 { 1900 + two } else { 2000 + two };
            (year, text.get(2..)?)
        }
        0x18 => (text.get(..4)?.parse().ok()?, text.get(4..)?),
        _ => return None,
    };
    let part = |at: usize| -> Option<i64> { rest.get(at..at + 2)?.parse().ok() };
    let days = days_from_civil(year, part(0)?, part(2)?);
    Some(days * SECONDS_PER_DAY + part(4)? * 3600 + part(6)? * 60 + part(8).unwrap_or(0))
}

#[cfg(test)]
mod tests {
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
        let now = 1_800_000_000;
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
}
