//! HTTPS certificates (SEC-10): a self-signed certificate made on first start and renewed before
//! it expires, or a certificate and key the owner provides. rustls with the `ring` provider.
//!
//! This module reports problems as [`TlsError`] values; the program turns them into plain
//! sentences from the message catalog.

use std::fmt;
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, date_time_ymd};
use rustls::ServerConfig;
use rustls::crypto::ring::default_provider;
use rustls::pki_types::pem::PemObject as _;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;

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

/// When the first certificate in the PEM file `cert` stops being valid, in seconds since 1970 (for
/// the status page). `None` when the file cannot be read or holds no certificate.
#[must_use]
pub fn certificate_end(cert: &Path) -> Option<i64> {
    let pem = fs::read(cert).ok()?;
    let der = CertificateDer::from_pem_slice(&pem).ok()?;
    not_after(&der)
}

/// The server-side TLS settings for `source`. A self-signed certificate is made (or renewed)
/// here if needed. The settings never change while the server runs; [`serve_certificate`] gives
/// the ones that follow a renewed or replaced certificate.
///
/// # Errors
/// Returns what is wrong with the files, or why a certificate could not be made.
pub fn server_config(source: &TlsSource) -> Result<Arc<ServerConfig>, TlsError> {
    serve_certificate(source).map(|served| served.config())
}

/// What a check of the certificate found (see [`ServedCertificate::renew`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Renewal {
    /// The certificate in use is still the right one.
    Kept,
    /// The server made a new certificate (it was close to its end, or its names changed) and
    /// serves it from now on.
    Renewed,
    /// The owner put another certificate in the files, and the server serves it from now on.
    Reloaded,
}

/// The certificate the server hands out. Connections that start after a swap get the new
/// certificate; connections that are open keep the one they started with.
#[derive(Debug)]
struct Swappable(RwLock<Arc<CertifiedKey>>);

impl Swappable {
    fn new(key: Arc<CertifiedKey>) -> Self {
        Self(RwLock::new(key))
    }

    fn current(&self) -> Arc<CertifiedKey> {
        Arc::clone(&self.0.read().unwrap_or_else(PoisonError::into_inner))
    }

    fn replace(&self, key: Arc<CertifiedKey>) {
        *self.0.write().unwrap_or_else(PoisonError::into_inner) = key;
    }
}

impl ResolvesServerCert for Swappable {
    fn resolve(&self, _hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        Some(self.current())
    }
}

/// The certificate in use, as the status page shows it and the renewal compares it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Served {
    /// Which certificate it is: a short digest of its first (own) certificate.
    id: u64,
    /// When it stops being valid, in seconds since 1970.
    ends: Option<i64>,
}

/// The TLS settings of a running server together with what keeps them current (SEC-10, M1-F03):
/// [`renew`](Self::renew) makes a new generated certificate when under 30 days are left and
/// picks up own certificate files that changed, without a restart.
pub struct ServedCertificate {
    source: TlsSource,
    resolver: Arc<Swappable>,
    config: Arc<ServerConfig>,
    served: Mutex<Served>,
}

impl ServedCertificate {
    /// The settings for the listener. They stay the same object for the life of the server; the
    /// certificate inside them is swapped.
    #[must_use]
    pub fn config(&self) -> Arc<ServerConfig> {
        Arc::clone(&self.config)
    }

    /// When the certificate in use stops being valid, in seconds since 1970; `None` when the
    /// certificate has no readable end.
    #[must_use]
    pub fn expires_at(&self) -> Option<i64> {
        self.served
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .ends
    }

    /// Looks at the certificate at the moment `now` (seconds since 1970) and swaps in a new one
    /// when it is due. A self-signed certificate is renewed when less than 30 days are left or
    /// the wanted names changed (see `ensure_self_signed`); an own certificate is read again and
    /// swapped when the files hold another one. A pair of files that cannot be used is left
    /// alone: the old certificate keeps serving, and the error says what is wrong.
    ///
    /// # Errors
    /// Returns what is wrong with the files, or why a certificate could not be made. The
    /// certificate in use is not changed then.
    pub fn renew(&self, now: i64) -> Result<Renewal, TlsError> {
        let (cert, key) = match &self.source {
            TlsSource::SelfSigned { folder, names } => {
                ensure_self_signed(folder, names, now)?;
                (folder.join(CERT_FILE), folder.join(KEY_FILE))
            }
            TlsSource::Files { cert, key } => (cert.clone(), key.clone()),
        };
        let (certified, served) = load_certified(&cert, &key)?;
        let mut current = self.served.lock().unwrap_or_else(PoisonError::into_inner);
        if current.id == served.id {
            return Ok(Renewal::Kept);
        }
        self.resolver.replace(certified);
        *current = served;
        Ok(match self.source {
            TlsSource::SelfSigned { .. } => Renewal::Renewed,
            TlsSource::Files { .. } => Renewal::Reloaded,
        })
    }
}

/// The certificate and key of `source` (a self-signed pair is made or renewed first), ready to
/// be served and to be swapped later.
///
/// # Errors
/// Returns what is wrong with the files, or why a certificate could not be made.
pub fn serve_certificate(source: &TlsSource) -> Result<ServedCertificate, TlsError> {
    let (cert, key) = match source {
        TlsSource::SelfSigned { folder, names } => {
            ensure_self_signed(folder, names, now_seconds())?;
            (folder.join(CERT_FILE), folder.join(KEY_FILE))
        }
        TlsSource::Files { cert, key } => (cert.clone(), key.clone()),
    };
    let (certified, served) = load_certified(&cert, &key)?;
    let resolver = Arc::new(Swappable::new(certified));
    let config = settings_for(Arc::clone(&resolver))?;
    Ok(ServedCertificate {
        source: source.clone(),
        resolver,
        config,
        served: Mutex::new(served),
    })
}

/// The TLS settings that hand out whatever certificate `resolver` holds.
fn settings_for(resolver: Arc<Swappable>) -> Result<Arc<ServerConfig>, TlsError> {
    let mut config = ServerConfig::builder_with_provider(Arc::new(default_provider()))
        .with_safe_default_protocol_versions()
        .map_err(|error| unusable(&error))?
        .with_no_client_auth()
        .with_cert_resolver(resolver);
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(Arc::new(config))
}

fn unusable(error: &rustls::Error) -> TlsError {
    TlsError::Unusable {
        reason: error.to_string(),
    }
}

/// Reads a certificate chain and key. Fails when the key does not belong to the certificate.
fn load_certified(cert: &Path, key: &Path) -> Result<(Arc<CertifiedKey>, Served), TlsError> {
    let chain = CertificateDer::pem_file_iter(cert)
        .map_err(|error| pem_error(cert, error, false))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| pem_error(cert, error, false))?;
    let Some(leaf) = chain.first() else {
        return Err(TlsError::NoCertificate {
            path: cert.to_path_buf(),
        });
    };
    let key_der = PrivateKeyDer::from_pem_file(key).map_err(|error| pem_error(key, error, true))?;
    // The whole chain and the key tell the certificate in use from the one in the files: an owner
    // may replace the intermediate certificates or the key and keep the first certificate.
    let mut parts: Vec<&[u8]> = chain.iter().map(AsRef::as_ref).collect();
    parts.push(key_der.secret_der());
    let served = Served {
        id: digest(&parts),
        ends: not_after(leaf.as_ref()),
    };
    let certified = CertifiedKey::from_der(chain, key_der, &default_provider())
        .map_err(|error| unusable(&error))?;
    Ok((Arc::new(certified), served))
}

/// A short digest of the parts (every certificate of the chain and the key, each with its length
/// in front): enough to tell the certificate in use from the one in the files, not a security
/// measure.
fn digest(parts: &[&[u8]]) -> u64 {
    // FNV-1a.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for part in parts {
        let length = u64::try_from(part.len()).unwrap_or(u64::MAX).to_be_bytes();
        for byte in length.iter().chain(part.iter()) {
            hash = (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3);
        }
    }
    hash
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

/// Makes sure `folder` holds a certificate and key that belong together, are valid for the
/// wanted names and will still be valid in `RENEW_BEFORE_DAYS` days; otherwise writes new
/// ones. Files that are kept get owner-only permissions again.
fn ensure_self_signed(folder: &Path, extra: &[String], now: i64) -> Result<(), TlsError> {
    let wanted = wanted_names(extra);
    let cert_path = folder.join(CERT_FILE);
    let key_path = folder.join(KEY_FILE);
    let names_path = folder.join(NAMES_FILE);
    let end = fs::read_to_string(&cert_path).ok().and_then(|pem| {
        let der = CertificateDer::from_pem_slice(pem.as_bytes()).ok()?;
        not_after(&der)
    });
    let same_names = fs::read_to_string(&names_path)
        .is_ok_and(|text| text.lines().eq(wanted.iter().map(String::as_str)));
    let fresh = end.is_some_and(|end| end - now > RENEW_BEFORE_DAYS * SECONDS_PER_DAY);
    // A key that does not match (a half-finished write, a copied file) is replaced too.
    if fresh && same_names && load_certified(&cert_path, &key_path).is_ok() {
        #[cfg(unix)]
        for path in [&key_path, &cert_path, &names_path] {
            restrict_file(path)?;
        }
        return Ok(());
    }
    let (cert_pem, key_pem) = generate(&wanted, now)?;
    // Key first and certificate second: a certificate on disk always has its key. The names
    // come last, so a stop in between only causes one more renewal.
    write_private(&key_path, key_pem.as_bytes())?;
    write_private(&cert_path, cert_pem.as_bytes())?;
    write_private(
        &names_path,
        wanted
            .join(
                "
",
            )
            .as_bytes(),
    )?;
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

/// Writes a secret file that only the owner can read (architecture §6), all at once: the
/// bytes go into a new file next to it, which then replaces the old one, so a stop in the
/// middle never leaves half a file.
fn write_private(path: &Path, bytes: &[u8]) -> Result<(), TlsError> {
    let failed = |source| TlsError::Write {
        path: path.to_path_buf(),
        source,
    };
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".new");
    let temporary = PathBuf::from(temporary);
    {
        let mut file = create_private(&temporary).map_err(failed)?;
        file.write_all(bytes).map_err(failed)?;
        file.sync_all().map_err(failed)?;
    }
    fs::rename(&temporary, path).map_err(failed)
}

#[cfg(unix)]
fn create_private(path: &Path) -> io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt as _;
    fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
}

/// On Windows the `tls` folder carries an access list for its owner only, which new files
/// inherit (the program sets it up with the rest of the data folder).
#[cfg(not(unix))]
fn create_private(path: &Path) -> io::Result<fs::File> {
    fs::File::create(path)
}

/// Owner-only again for a file that already exists (Linux: mode 0600).
#[cfg(unix)]
fn restrict_file(path: &Path) -> Result<(), TlsError> {
    use std::os::unix::fs::PermissionsExt as _;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|source| TlsError::Write {
        path: path.to_path_buf(),
        source,
    })
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
        let (cert_pem, key_pem) =
            generate(&wanted_names(&[]), now + 100 * SECONDS_PER_DAY).unwrap();
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
}
