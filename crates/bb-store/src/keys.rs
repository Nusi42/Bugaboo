//! The master keys (SEC-16): 32 random bytes each, kept in `keys/master.key` of the data folder,
//! outside the database. The folder is created owner-only by the server (WP-04); on Unix the file
//! is made readable by its owner only and refused when it is not.
//!
//! The file lists the keys, oldest first, one per line: `<id> <64 hex digits>`. The newest key
//! encrypts; older keys stay until nothing uses them any more (rotation, [`crate::secrets`]).
//! Keys live in memory only as [`Zeroizing`] bytes and never appear in `Debug` output.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use bb_core::technical::TechnicalName;
use hmac::{Hmac, KeyInit as _, Mac as _};
use sha2::Sha256;
use zeroize::Zeroizing;

use crate::error::{SecretProblem, StoreError};

/// The key file's name inside the keys folder.
const KEY_FILE: &str = "master.key";
/// The same file while it is being replaced.
const NEW_KEY_FILE: &str = "master.key.new";
/// The size of a key.
pub(crate) const KEY_LENGTH: usize = 32;
/// The label the signing key for the security log's checkpoints is derived with.
const SIGNING_LABEL: &[u8] = b"bugaboo.security-log.checkpoint-key.v1";

type HmacSha256 = Hmac<Sha256>;

/// Fresh secure random bytes.
pub(crate) fn random_bytes<const N: usize>() -> Result<[u8; N], StoreError> {
    let mut bytes = [0_u8; N];
    getrandom::fill(&mut bytes).map_err(|_| StoreError::Secret(SecretProblem::NoRandomness))?;
    Ok(bytes)
}

/// One master key.
#[derive(Clone)]
pub struct MasterKey {
    id: TechnicalName,
    bytes: Zeroizing<[u8; KEY_LENGTH]>,
}

impl std::fmt::Debug for MasterKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MasterKey")
            .field("id", &self.id)
            .field("bytes", &"<hidden>")
            .finish()
    }
}

impl MasterKey {
    fn generate(id: TechnicalName) -> Result<Self, StoreError> {
        Ok(Self {
            id,
            bytes: Zeroizing::new(random_bytes()?),
        })
    }

    /// The ID stored with every value this key wrapped.
    #[must_use]
    pub const fn id(&self) -> &TechnicalName {
        &self.id
    }

    pub(crate) fn bytes(&self) -> &[u8; KEY_LENGTH] {
        &self.bytes
    }

    /// The key that signs the checkpoints of the security log, derived from this master key so
    /// it needs no storage of its own.
    pub(crate) fn signing_key(&self) -> Result<SigningKey, StoreError> {
        let mut mac = HmacSha256::new_from_slice(self.bytes.as_slice())
            .map_err(|_| StoreError::Secret(SecretProblem::BadKeyFile))?;
        mac.update(SIGNING_LABEL);
        Ok(SigningKey(Zeroizing::new(
            mac.finalize().into_bytes().into(),
        )))
    }
}

/// A key derived from a master key that signs with HMAC-SHA256.
pub struct SigningKey(Zeroizing<[u8; KEY_LENGTH]>);

impl std::fmt::Debug for SigningKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SigningKey(<hidden>)")
    }
}

impl SigningKey {
    /// The HMAC-SHA256 of `parts` joined, under this key.
    pub(crate) fn sign(&self, parts: &[&[u8]]) -> Result<[u8; 32], StoreError> {
        let mut mac = self.mac()?;
        for part in parts {
            mac.update(part);
        }
        Ok(mac.finalize().into_bytes().into())
    }

    /// Whether `signature` is the HMAC of `parts`; the comparison takes the same time whatever
    /// the signature is.
    pub(crate) fn verify(&self, parts: &[&[u8]], signature: &[u8]) -> bool {
        let Ok(mut mac) = self.mac() else {
            return false;
        };
        for part in parts {
            mac.update(part);
        }
        mac.verify_slice(signature).is_ok()
    }

    fn mac(&self) -> Result<HmacSha256, StoreError> {
        HmacSha256::new_from_slice(self.0.as_slice())
            .map_err(|_| StoreError::Secret(SecretProblem::BadKeyFile))
    }
}

/// The master keys of this installation, oldest first.
#[derive(Debug)]
pub struct KeyRing {
    keys: Vec<MasterKey>,
    folder: PathBuf,
}

impl KeyRing {
    /// Reads the key file in `folder` (the `keys/` folder of the data folder), or makes it with
    /// one new key when this is the first start.
    ///
    /// # Errors
    /// Fails when the file cannot be read or written, has a line that is not a key, or (on Unix)
    /// is readable by anybody but its owner.
    pub fn open_or_create(folder: &Path) -> Result<Self, StoreError> {
        let path = folder.join(KEY_FILE);
        let keys = match fs::read_to_string(&path) {
            Ok(text) => {
                check_permissions(&path)?;
                parse(&text)?
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let first = MasterKey::generate(key_id(1)?)?;
                write_file(folder, std::slice::from_ref(&first), true)?;
                vec![first]
            }
            Err(error) => return Err(StoreError::KeyFile(error)),
        };
        Ok(Self {
            keys,
            folder: folder.to_owned(),
        })
    }

    /// The key that encrypts new values.
    ///
    /// # Errors
    /// Never fails for a ring made by [`Self::open_or_create`]; the error is for a defect.
    pub fn newest(&self) -> Result<&MasterKey, StoreError> {
        self.keys
            .last()
            .ok_or(StoreError::Secret(SecretProblem::BadKeyFile))
    }

    /// The key with this ID, if the file still has it.
    pub(crate) fn get(&self, id: &TechnicalName) -> Option<&MasterKey> {
        self.keys.iter().find(|key| key.id() == id)
    }

    /// Every key, oldest first.
    pub(crate) fn all(&self) -> &[MasterKey] {
        &self.keys
    }

    /// The IDs of all keys, oldest first.
    #[must_use]
    pub fn ids(&self) -> Vec<TechnicalName> {
        self.keys.iter().map(|key| key.id.clone()).collect()
    }

    /// Adds a new key and saves the file. Old keys stay, so everything still decrypts. This is
    /// the first step of a rotation; the second re-wraps the secrets (`Database::rotate_master_key`).
    pub(crate) fn add_key(&mut self) -> Result<(), StoreError> {
        let number = self.keys.last().map_or(1, |key| id_number(key.id()) + 1);
        let key = MasterKey::generate(key_id(number)?)?;
        let mut keys = self.keys.clone();
        keys.push(key);
        write_file(&self.folder, &keys, false)?;
        self.keys = keys;
        Ok(())
    }

    /// Removes the keys that nothing uses any more (never the newest) and saves the file.
    pub(crate) fn retire_unused(&mut self, in_use: &[TechnicalName]) -> Result<(), StoreError> {
        let newest = self.newest()?.id().clone();
        let kept: Vec<MasterKey> = self
            .keys
            .iter()
            .filter(|key| key.id() == &newest || in_use.contains(key.id()))
            .cloned()
            .collect();
        if kept.len() != self.keys.len() {
            write_file(&self.folder, &kept, false)?;
            self.keys = kept;
        }
        Ok(())
    }
}

fn key_id(number: u64) -> Result<TechnicalName, StoreError> {
    TechnicalName::new(&format!("k{number}"))
        .map_err(|_| StoreError::Secret(SecretProblem::BadKeyFile))
}

/// The number in a key ID like `k3`; 0 for an ID of another shape.
fn id_number(id: &TechnicalName) -> u64 {
    id.as_str()
        .strip_prefix('k')
        .and_then(|digits| digits.parse().ok())
        .unwrap_or(0)
}

fn parse(text: &str) -> Result<Vec<MasterKey>, StoreError> {
    let bad = || StoreError::Secret(SecretProblem::BadKeyFile);
    let mut keys: Vec<MasterKey> = Vec::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let (id, hex) = line.split_once(' ').ok_or_else(bad)?;
        let id = TechnicalName::new(id).map_err(|_| bad())?;
        if keys.iter().any(|key| key.id() == &id) {
            return Err(bad());
        }
        let bytes = from_hex(hex.trim()).ok_or_else(bad)?;
        keys.push(MasterKey { id, bytes });
    }
    if keys.is_empty() {
        return Err(bad());
    }
    Ok(keys)
}

fn from_hex(text: &str) -> Option<Zeroizing<[u8; KEY_LENGTH]>> {
    if text.len() != KEY_LENGTH * 2 || !text.is_ascii() {
        return None;
    }
    let mut bytes = Zeroizing::new([0_u8; KEY_LENGTH]);
    for (at, slot) in bytes.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&text[at * 2..at * 2 + 2], 16).ok()?;
    }
    Some(bytes)
}

fn to_hex(bytes: &[u8]) -> Zeroizing<String> {
    let mut text = Zeroizing::new(String::with_capacity(bytes.len() * 2));
    for byte in bytes {
        text.push(char::from_digit(u32::from(byte >> 4), 16).unwrap_or('0'));
        text.push(char::from_digit(u32::from(byte & 0x0f), 16).unwrap_or('0'));
    }
    text
}

/// Writes the whole file. A change goes to a new file that replaces the old one, so a crash
/// leaves either the old or the new content.
fn write_file(folder: &Path, keys: &[MasterKey], first: bool) -> Result<(), StoreError> {
    let mut text = Zeroizing::new(String::new());
    for key in keys {
        text.push_str(key.id().as_str());
        text.push(' ');
        text.push_str(&to_hex(key.bytes()));
        text.push('\n');
    }
    fs::create_dir_all(folder).map_err(StoreError::KeyFile)?;
    let target = folder.join(KEY_FILE);
    let temporary = folder.join(NEW_KEY_FILE);
    let _ = fs::remove_file(&temporary);
    let mut file = create_private(&temporary, first).map_err(StoreError::KeyFile)?;
    file.write_all(text.as_bytes())
        .map_err(StoreError::KeyFile)?;
    file.sync_all().map_err(StoreError::KeyFile)?;
    drop(file);
    if first {
        // A second process that made the first key at the same moment must not be overwritten.
        fs::hard_link(&temporary, &target).map_err(StoreError::KeyFile)?;
        fs::remove_file(&temporary).map_err(StoreError::KeyFile)?;
    } else {
        fs::rename(&temporary, &target).map_err(StoreError::KeyFile)?;
    }
    Ok(())
}

#[cfg(unix)]
fn create_private(path: &Path, _first: bool) -> std::io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt as _;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}

// On Windows the file inherits the owner-only access of the `keys/` folder (WP-04).
#[cfg(not(unix))]
fn create_private(path: &Path, _first: bool) -> std::io::Result<fs::File> {
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
}

/// Refuses a key file that others can read. Only Unix has file modes; on Windows the file has
/// the owner-only access of the `keys/` folder it is in (M1-F21).
fn check_permissions(path: &Path) -> Result<(), StoreError> {
    let metadata = fs::metadata(path).map_err(StoreError::KeyFile)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(StoreError::KeyFile(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "the master key file must be readable by its owner only (mode 0600)",
            )));
        }
    }
    #[cfg(not(unix))]
    let _ = metadata;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_start_makes_one_key_and_the_next_start_reads_it() {
        let folder = tempfile::tempdir().unwrap();
        let first = KeyRing::open_or_create(folder.path()).unwrap();
        assert_eq!(first.ids(), [key_id(1).unwrap()]);
        let again = KeyRing::open_or_create(folder.path()).unwrap();
        assert_eq!(
            again.newest().unwrap().bytes(),
            first.newest().unwrap().bytes()
        );
        let text = fs::read_to_string(folder.path().join(KEY_FILE)).unwrap();
        assert!(
            text.starts_with("k1 ") && text.len() == 3 + 64 + 1,
            "{} bytes",
            text.len()
        );
    }

    #[test]
    fn adding_a_key_keeps_the_old_one_and_retiring_drops_unused_old_ones() {
        let folder = tempfile::tempdir().unwrap();
        let mut ring = KeyRing::open_or_create(folder.path()).unwrap();
        ring.add_key().unwrap();
        ring.add_key().unwrap();
        assert_eq!(
            ring.ids(),
            [key_id(1).unwrap(), key_id(2).unwrap(), key_id(3).unwrap()]
        );
        assert_eq!(
            KeyRing::open_or_create(folder.path()).unwrap().ids().len(),
            3
        );
        ring.retire_unused(&[key_id(1).unwrap()]).unwrap();
        assert_eq!(
            ring.ids(),
            [key_id(1).unwrap(), key_id(3).unwrap()],
            "the newest key is never retired"
        );
        assert_eq!(
            KeyRing::open_or_create(folder.path()).unwrap().ids().len(),
            2
        );
    }

    #[test]
    fn bad_example_a_damaged_key_file_is_refused() {
        for text in [
            "",
            "k1\n",
            "k1 abcd\n",
            &format!("K1 {}\n", "0".repeat(64)),
            &format!("k1 {}\nk1 {}\n", "0".repeat(64), "1".repeat(64)),
            &format!("k1 {}\n", "zz".repeat(32)),
        ] {
            assert!(parse(text).is_err(), "{text}");
        }
        assert!(parse(&format!("k1 {}\n", "ab".repeat(32))).is_ok());
    }

    #[test]
    fn keys_never_show_in_debug_output() {
        let folder = tempfile::tempdir().unwrap();
        let ring = KeyRing::open_or_create(folder.path()).unwrap();
        let key = ring.newest().unwrap();
        let shown = format!("{ring:?} {key:?} {:?}", key.signing_key().unwrap());
        let hex = to_hex(key.bytes());
        assert!(!shown.contains(hex.as_str()));
        assert!(shown.contains("<hidden>"));
    }

    #[test]
    fn the_signing_key_signs_and_checks_in_a_way_that_depends_on_every_byte() {
        let folder = tempfile::tempdir().unwrap();
        let ring = KeyRing::open_or_create(folder.path()).unwrap();
        let key = ring.newest().unwrap().signing_key().unwrap();
        let signature = key.sign(&[b"a", b"bc"]).unwrap();
        assert!(key.verify(&[b"abc"], &signature));
        assert!(!key.verify(&[b"xyz"], &signature));
        let other = KeyRing::open_or_create(tempfile::tempdir().unwrap().path()).unwrap();
        assert!(
            !other
                .newest()
                .unwrap()
                .signing_key()
                .unwrap()
                .verify(&[b"abc"], &signature)
        );
    }

    #[cfg(unix)]
    #[test]
    fn bad_example_a_key_file_others_can_read_is_refused() {
        use std::os::unix::fs::PermissionsExt as _;
        let folder = tempfile::tempdir().unwrap();
        KeyRing::open_or_create(folder.path()).unwrap();
        let path = folder.path().join(KEY_FILE);
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(KeyRing::open_or_create(folder.path()).is_err());
    }
}
