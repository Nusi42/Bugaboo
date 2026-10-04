//! The anchor file of the security log (SEC-15): where the end of the log is written down outside
//! the database.
//!
//! After every append, one line `<seq> <hash> <signature>` goes to an append-only file in the
//! data folder (`logs/security-head.log`). The signature is an HMAC under the log key. Somebody
//! who can only change the database can then neither cut entries off the end nor replace them:
//! [`Database::verify_security_log`](crate::Database::verify_security_log) requires the database
//! to have at least the newest anchored entry, with the same hash.
//!
//! The `logs/` folder is made private by the server with the rest of the data folder; this file
//! does not create it, and creates the file itself owner-only on Unix.
//!
//! The file is written *after* the entry is committed, so it is never ahead of the database. A
//! crash between the two leaves the file one entry behind, which is harmless. A line that is
//! only half written (a crash while writing) is ignored; any other line without a valid
//! signature is a forgery. Somebody who can change the data folder can delete the file; the
//! verification then reports the log as not anchored.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use bb_core::security_log::Digest;

use crate::error::StoreError;
use crate::keys::{KeyRing, SigningKey};

const ANCHOR_DOMAIN: &[u8] = b"bugaboo.security-log.anchor.v1";

/// What the anchor file knows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Anchored {
    /// There is no file, or no line in it yet.
    Nothing,
    /// The newest entry that was written down.
    At(i64, Digest),
    /// A line has no valid signature.
    Forged,
}

/// The key and the file that make the security log tamper-evident.
#[derive(Clone, Debug)]
pub struct LogSeal {
    key: SigningKey,
    pub(crate) file: PathBuf,
}

impl LogSeal {
    /// The seal of this installation: the log key of `keys` and the anchor file `file`
    /// (`logs/security-head.log` in the data folder).
    #[must_use]
    pub fn new(keys: &KeyRing, file: impl Into<PathBuf>) -> Self {
        Self::with_key(keys.log_key(), file)
    }

    /// A seal with a given key.
    #[must_use]
    pub fn with_key(key: SigningKey, file: impl Into<PathBuf>) -> Self {
        Self {
            key,
            file: file.into(),
        }
    }

    pub(crate) const fn key(&self) -> &SigningKey {
        &self.key
    }

    /// Writes down that the log has reached `seq` with `hash`.
    pub(crate) fn record(&self, seq: i64, hash: &Digest) -> Result<(), StoreError> {
        let signature = self
            .key
            .sign(&[ANCHOR_DOMAIN, &seq.to_be_bytes(), hash.as_bytes()])?;
        let line = format!("{seq} {} {}\n", hex(hash.as_bytes()), hex(&signature));
        let mut file = open_append(&self.file).map_err(StoreError::AnchorFile)?;
        // One write call per line keeps lines whole when two appends finish together.
        file.write_all(line.as_bytes())
            .map_err(StoreError::AnchorFile)?;
        file.sync_data().map_err(StoreError::AnchorFile)
    }

    /// The newest entry written down.
    pub(crate) fn latest(&self) -> Result<Anchored, StoreError> {
        let text = match fs::read(&self.file) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Anchored::Nothing);
            }
            Err(error) => return Err(StoreError::AnchorFile(error)),
        };
        let mut newest: Option<(i64, Digest)> = None;
        // Only lines that end with a newline are whole; the rest is a write that was cut off.
        let whole = text
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(&text[..0], |end| &text[..=end]);
        for line in String::from_utf8_lossy(whole).lines() {
            let Some((seq, hash)) = self.read_line(line) else {
                return Ok(Anchored::Forged);
            };
            if newest.is_none_or(|(known, _)| seq >= known) {
                newest = Some((seq, hash));
            }
        }
        Ok(newest.map_or(Anchored::Nothing, |(seq, hash)| Anchored::At(seq, hash)))
    }

    fn read_line(&self, line: &str) -> Option<(i64, Digest)> {
        let mut parts = line.split(' ');
        let seq: i64 = parts.next()?.parse().ok()?;
        let hash: [u8; 32] = unhex(parts.next()?)?;
        let signature: [u8; 32] = unhex(parts.next()?)?;
        if parts.next().is_some() {
            return None;
        }
        self.key
            .verify(&[ANCHOR_DOMAIN, &seq.to_be_bytes(), &hash], &signature)
            .then(|| (seq, Digest::new(hash)))
    }
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(text, "{byte:02x}");
    }
    text
}

fn unhex<const N: usize>(text: &str) -> Option<[u8; N]> {
    if text.len() != N * 2 || !text.is_ascii() {
        return None;
    }
    let mut bytes = [0_u8; N];
    for (at, slot) in bytes.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&text[at * 2..at * 2 + 2], 16).ok()?;
    }
    Some(bytes)
}

#[cfg(unix)]
fn open_append(path: &Path) -> std::io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt as _;
    fs::OpenOptions::new()
        .append(true)
        .create(true)
        .mode(0o600)
        .open(path)
}

// On Windows the file inherits the access rights of the data folder (WP-04).
#[cfg(not(unix))]
fn open_append(path: &Path) -> std::io::Result<fs::File> {
    fs::OpenOptions::new().append(true).create(true).open(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seal() -> (tempfile::TempDir, LogSeal) {
        let folder = tempfile::tempdir().unwrap();
        fs::create_dir(folder.path().join("logs")).unwrap();
        let seal = LogSeal::with_key(
            SigningKey::generate().unwrap(),
            folder.path().join("logs").join("security-head.log"),
        );
        (folder, seal)
    }

    fn digest(byte: u8) -> Digest {
        Digest::new([byte; 32])
    }

    #[test]
    fn the_newest_line_wins_also_when_lines_come_out_of_order() {
        let (_folder, seal) = seal();
        assert_eq!(seal.latest().unwrap(), Anchored::Nothing);
        seal.record(1, &digest(1)).unwrap();
        seal.record(3, &digest(3)).unwrap();
        seal.record(2, &digest(2)).unwrap();
        assert_eq!(seal.latest().unwrap(), Anchored::At(3, digest(3)));
    }

    #[test]
    fn bad_example_a_line_without_the_right_signature_is_a_forgery() {
        let (_folder, seal) = seal();
        seal.record(1, &digest(1)).unwrap();
        let other = LogSeal::with_key(SigningKey::generate().unwrap(), seal.file.clone());
        other.record(2, &digest(2)).unwrap();
        assert_eq!(seal.latest().unwrap(), Anchored::Forged);
    }

    #[test]
    fn a_half_written_last_line_is_ignored() {
        let (_folder, seal) = seal();
        seal.record(1, &digest(1)).unwrap();
        let mut file = open_append(&seal.file).unwrap();
        file.write_all(b"2 abcd").unwrap();
        assert_eq!(seal.latest().unwrap(), Anchored::At(1, digest(1)));
    }
}
