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
//!
//! A restore of an older backup puts back a log that ends before the anchor does (M2-F09). The
//! restore then writes a *new start*: one line `restart <seq> <hash> <lost-from> <lost-to>
//! <signature>` with the head of the restored log and the range of entries that are gone, signed
//! under a domain of its own. Lines before the newest new start no longer count, so the anchor
//! vouches for the restored log from there on, and the line keeps saying what was lost.
//!
//! The file would grow for ever, so the job that prunes the log trims it after it deleted blocks
//! (M1-F26, ADR-0045, decision 5): only the newest line counts, so only it is kept, after the newest
//! new start copied as it was, so the file keeps saying what a restore lost. The trimmed file is
//! written next to the old one and renamed into place; a crash between leaves the old file whole.
//! A line appended while the file is trimmed may be lost, which leaves the file behind the
//! database, which is harmless (as above).

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use bb_core::hex;
use bb_core::security_log::Digest;

use crate::error::StoreError;
use crate::keys::{KeyRing, SigningKey};

const ANCHOR_DOMAIN: &[u8] = b"bugaboo.security-log.anchor.v1";
/// The domain of a new start after a restore: a line of one kind can never pass for the other.
const RESTART_DOMAIN: &[u8] = b"bugaboo.security-log.anchor.restart.v1";
/// The first word of a new start.
const RESTART: &str = "restart";

/// The extension of the trimmed anchor file while it is written, before it is renamed into place.
const TRIMMED_EXTENSION: &str = "trimmed";

/// The newest new start of the anchor file: the line as it was written, and the head of the restored
/// log it names (`None` for an empty log).
#[derive(Clone, Debug, PartialEq, Eq)]
struct Restart {
    line: String,
    head: Option<(i64, Digest)>,
}

/// What the lines of the anchor file say: the newest entry, and the newest new start.
struct Lines {
    anchored: Anchored,
    restart: Option<Restart>,
}

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
        let line = format!(
            "{seq} {} {}\n",
            hex::encode(hash.as_bytes()),
            hex::encode(&signature)
        );
        self.append_line(&line)
    }

    /// Writes down a new start after a restore: the log now ends at `head` (`None` for an empty
    /// log), and the entries `lost.0` to `lost.1` that the anchor knew are gone. Lines before it
    /// no longer count.
    pub(crate) fn restart(
        &self,
        head: Option<(i64, Digest)>,
        lost: (i64, i64),
    ) -> Result<(), StoreError> {
        let (seq, hash) = head.unwrap_or((0, Digest::new([0; Digest::LENGTH])));
        let parts = restart_parts(seq, &hash, lost);
        let signature = self.key.sign(&parts.each_ref().map(Vec::as_slice))?;
        let line = format!(
            "{RESTART} {seq} {} {} {} {}
",
            hex::encode(hash.as_bytes()),
            lost.0,
            lost.1,
            hex::encode(&signature)
        );
        self.append_line(&line)
    }

    fn append_line(&self, line: &str) -> Result<(), StoreError> {
        let mut file = open_append(&self.file).map_err(StoreError::AnchorFile)?;
        // One write call per line keeps lines whole when two appends finish together.
        file.write_all(line.as_bytes())
            .map_err(StoreError::AnchorFile)?;
        file.sync_data().map_err(StoreError::AnchorFile)
    }

    /// The newest entry written down.
    pub(crate) fn latest(&self) -> Result<Anchored, StoreError> {
        Ok(self.lines()?.anchored)
    }

    /// The newest entry written down, and the newest new start; a forged line makes the file
    /// [`Anchored::Forged`] and nothing else.
    fn lines(&self) -> Result<Lines, StoreError> {
        let forged = Lines {
            anchored: Anchored::Forged,
            restart: None,
        };
        let text = match fs::read(&self.file) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Lines {
                    anchored: Anchored::Nothing,
                    restart: None,
                });
            }
            Err(error) => return Err(StoreError::AnchorFile(error)),
        };
        let mut newest: Option<(i64, Digest)> = None;
        let mut restart: Option<Restart> = None;
        // Only lines that end with a newline are whole; the rest is a write that was cut off.
        let whole = text
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(&text[..0], |end| &text[..=end]);
        for line in String::from_utf8_lossy(whole).lines() {
            if let Some(rest) = line
                .strip_prefix(RESTART)
                .and_then(|rest| rest.strip_prefix(' '))
            {
                let Some((seq, hash)) = self.read_restart(rest) else {
                    return Ok(forged);
                };
                // A new start: what came before belongs to a log that was replaced.
                newest = (seq > 0).then_some((seq, hash));
                restart = Some(Restart {
                    line: line.to_owned(),
                    head: newest,
                });
                continue;
            }
            let Some((seq, hash)) = self.read_line(line) else {
                return Ok(forged);
            };
            if newest.is_none_or(|(known, _)| seq >= known) {
                newest = Some((seq, hash));
            }
        }
        Ok(Lines {
            anchored: newest.map_or(Anchored::Nothing, |(seq, hash)| Anchored::At(seq, hash)),
            restart,
        })
    }

    /// Trims the file to its newest new start and its newest line, by [`Self::write_trimmed`] and a
    /// rename. Returns whether it did: a file without a line, or with a forged one, is left as it
    /// is, so a forgery stays for the verification to report.
    pub(crate) fn trim(&self) -> Result<bool, StoreError> {
        let Some(trimmed) = self.write_trimmed()? else {
            return Ok(false);
        };
        fs::rename(&trimmed, &self.file).map_err(StoreError::AnchorFile)?;
        Ok(true)
    }

    /// The first half of [`Self::trim`]: writes the newest new start as it was and the newest line,
    /// signed again when the new start does not name it already, to a file next to the anchor file
    /// and says where. The anchor file itself is not touched.
    fn write_trimmed(&self) -> Result<Option<PathBuf>, StoreError> {
        let Lines { anchored, restart } = self.lines()?;
        let head = match anchored {
            Anchored::At(seq, hash) => Some((seq, hash)),
            Anchored::Nothing => None,
            Anchored::Forged => return Ok(None),
        };
        if head.is_none() && restart.is_none() {
            return Ok(None);
        }
        let trimmed = self.file.with_extension(TRIMMED_EXTENSION);
        // A trimmed file of a run that stopped half way is old: it is replaced.
        match fs::remove_file(&trimmed) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                return Err(StoreError::AnchorFile(error));
            }
            _ => {}
        }
        let written = Self::with_key(self.key.clone(), &trimmed);
        if let Some(restart) = &restart {
            written.append_line(&format!("{}\n", restart.line))?;
        }
        if let Some((seq, hash)) = head
            && restart.as_ref().is_none_or(|restart| restart.head != head)
        {
            written.record(seq, &hash)?;
        }
        Ok(Some(trimmed))
    }

    fn read_line(&self, line: &str) -> Option<(i64, Digest)> {
        let mut parts = line.split(' ');
        let seq: i64 = parts.next()?.parse().ok()?;
        let hash: [u8; 32] = hex::decode(parts.next()?)?;
        let signature: [u8; 32] = hex::decode(parts.next()?)?;
        if parts.next().is_some() {
            return None;
        }
        self.key
            .verify(&[ANCHOR_DOMAIN, &seq.to_be_bytes(), &hash], &signature)
            .then(|| (seq, Digest::new(hash)))
    }

    fn read_restart(&self, rest: &str) -> Option<(i64, Digest)> {
        let mut parts = rest.split(' ');
        let seq: i64 = parts.next()?.parse().ok()?;
        let hash = Digest::new(hex::decode(parts.next()?)?);
        let lost: (i64, i64) = (parts.next()?.parse().ok()?, parts.next()?.parse().ok()?);
        let signature: [u8; 32] = hex::decode(parts.next()?)?;
        if parts.next().is_some() {
            return None;
        }
        let parts = restart_parts(seq, &hash, lost);
        self.key
            .verify(&parts.each_ref().map(Vec::as_slice), &signature)
            .then_some((seq, hash))
    }
}

/// What the signature of a new start covers.
fn restart_parts(seq: i64, hash: &Digest, lost: (i64, i64)) -> [Vec<u8>; 5] {
    [
        RESTART_DOMAIN.to_vec(),
        seq.to_be_bytes().to_vec(),
        hash.as_bytes().to_vec(),
        lost.0.to_be_bytes().to_vec(),
        lost.1.to_be_bytes().to_vec(),
    ]
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
    fn sec_15_a_new_start_after_a_restore_replaces_what_came_before() {
        let (_folder, seal) = seal();
        seal.record(1, &digest(1)).unwrap();
        seal.record(5, &digest(5)).unwrap();
        // The restored log ends at 3: the anchor follows it, and lines after count again.
        seal.restart(Some((3, digest(3))), (4, 5)).unwrap();
        assert_eq!(seal.latest().unwrap(), Anchored::At(3, digest(3)));
        seal.record(4, &digest(14)).unwrap();
        assert_eq!(seal.latest().unwrap(), Anchored::At(4, digest(14)));
        // A restored log with no entries.
        seal.restart(None, (1, 4)).unwrap();
        assert_eq!(seal.latest().unwrap(), Anchored::Nothing);
    }

    #[test]
    fn bad_example_a_new_start_signed_by_another_key_is_a_forgery() {
        let (_folder, seal) = seal();
        seal.record(5, &digest(5)).unwrap();
        let other = LogSeal::with_key(SigningKey::generate().unwrap(), seal.file.clone());
        other.restart(Some((1, digest(1))), (2, 5)).unwrap();
        assert_eq!(seal.latest().unwrap(), Anchored::Forged);
    }

    #[test]
    fn sec_15_the_anchor_file_is_trimmed_and_keeps_its_newest_line() {
        let (_folder, seal) = seal();
        assert!(!seal.trim().unwrap(), "no file: nothing to trim");
        for seq in 1..=5 {
            seal.record(seq, &digest(u8::try_from(seq).unwrap()))
                .unwrap();
        }
        let before = fs::read(&seal.file).unwrap();
        // A crash between the write and the rename leaves the old file whole.
        let trimmed = seal.write_trimmed().unwrap().unwrap();
        assert_eq!(fs::read(&seal.file).unwrap(), before);
        assert_eq!(seal.latest().unwrap(), Anchored::At(5, digest(5)));
        assert!(trimmed.exists());
        // The next trim replaces what the stopped one left.
        assert!(seal.trim().unwrap());
        assert!(!trimmed.exists());
        let text = fs::read_to_string(&seal.file).unwrap();
        assert_eq!(text.lines().count(), 1);
        assert_eq!(seal.latest().unwrap(), Anchored::At(5, digest(5)));
        // The log goes on after a trim.
        seal.record(6, &digest(6)).unwrap();
        assert_eq!(seal.latest().unwrap(), Anchored::At(6, digest(6)));
    }

    #[test]
    fn sec_15_a_trim_after_a_new_start_keeps_the_head_of_the_restored_log() {
        let (_folder, seal) = seal();
        seal.record(5, &digest(5)).unwrap();
        seal.restart(Some((3, digest(3))), (4, 5)).unwrap();
        let text = fs::read_to_string(&seal.file).unwrap();
        let restart = text.lines().nth(1).unwrap().to_owned();
        assert!(seal.trim().unwrap());
        assert_eq!(seal.latest().unwrap(), Anchored::At(3, digest(3)));
        // The new start alone names the head: it is the whole file, as it was written.
        assert_eq!(
            fs::read_to_string(&seal.file).unwrap(),
            format!("{restart}\n")
        );
        seal.record(4, &digest(14)).unwrap();
        assert!(seal.trim().unwrap());
        assert_eq!(seal.latest().unwrap(), Anchored::At(4, digest(14)));
        // The new start stays as it was, so the file keeps saying what the restore lost.
        let text = fs::read_to_string(&seal.file).unwrap();
        assert_eq!(text.lines().count(), 2);
        assert_eq!(text.lines().next(), Some(restart.as_str()));
        // A restored log with no entries: the new start is kept too.
        seal.restart(None, (1, 4)).unwrap();
        assert!(seal.trim().unwrap());
        assert_eq!(seal.latest().unwrap(), Anchored::Nothing);
        assert_eq!(fs::read_to_string(&seal.file).unwrap().lines().count(), 1);
    }

    #[test]
    fn bad_example_a_forged_anchor_file_is_not_trimmed() {
        let (_folder, seal) = seal();
        seal.record(1, &digest(1)).unwrap();
        let other = LogSeal::with_key(SigningKey::generate().unwrap(), seal.file.clone());
        other.record(2, &digest(2)).unwrap();
        let before = fs::read(&seal.file).unwrap();
        assert!(!seal.trim().unwrap());
        assert_eq!(fs::read(&seal.file).unwrap(), before);
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
