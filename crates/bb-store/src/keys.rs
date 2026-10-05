//! The keys (SEC-16, SEC-15): two files in the `keys/` folder of the data folder, outside the
//! database. The server makes the folder owner-only (WP-04); on Unix the files are made readable
//! by their owner only and refused when they are not.
//!
//! - `master.key` lists the **master keys**, oldest first, one per line: `<id> <64 hex digits>`.
//!   The newest encrypts; older keys stay until nothing uses them any more (rotation,
//!   [`crate::secrets`]).
//! - `log.key` holds the **log key**: 64 hex digits. It signs the hash chain and the checkpoints
//!   of the security log. It never changes, so rotating the master key does not touch the log.
//! - `csrf.key` holds the **CSRF key**: 64 hex digits. The token of a session is an HMAC of the
//!   session ID under this key (SEC-12). It never changes, and no stored data depends on it: if the
//!   file is lost, a new one is made and people sign in again (the old tokens stop matching), so
//!   it is made even when the database already has data.
//!
//! Keys are made on the first start only. If the database already holds secrets or a security log
//! and a key file is missing, [`Database::open_keys`](crate::Database::open_keys) refuses instead
//! of making new keys, which could never read what the old ones protected. Keys live in memory as
//! [`Zeroizing`] bytes, are read through [`Zeroizing`] buffers, and never appear in `Debug`
//! output.
//!
//! A [`KeyRing`] checks the modification time and size of `master.key` whenever it is asked for a
//! key, and reads the file again when it changed, so a server keeps up with a rotation made by the
//! command line without a restart.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

use bb_core::technical::TechnicalName;
use hmac::{Hmac, KeyInit as _, Mac as _};
use sha2::Sha256;
use zeroize::Zeroizing;

use crate::error::{SecretProblem, StoreError};

/// The master key file's name inside the keys folder.
const MASTER_FILE: &str = "master.key";
/// The log key file's name.
const LOG_FILE: &str = "log.key";
/// The CSRF key file's name.
const CSRF_FILE: &str = "csrf.key";
/// The same master file while it is being replaced.
const NEW_MASTER_FILE: &str = "master.key.new";
/// The size of a key.
pub(crate) const KEY_LENGTH: usize = 32;
/// How long a start waits for the other process that made the key files at the same moment.
const WINNER_PATIENCE: Duration = Duration::from_secs(2);

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
}

/// A key that signs with HMAC-SHA256: the log key, or a key made for a test.
#[derive(Clone)]
pub struct SigningKey(Zeroizing<[u8; KEY_LENGTH]>);

impl std::fmt::Debug for SigningKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SigningKey(<hidden>)")
    }
}

impl SigningKey {
    /// A new random key.
    ///
    /// # Errors
    /// Fails when the system gives no secure random bytes.
    pub fn generate() -> Result<Self, StoreError> {
        Ok(Self(Zeroizing::new(random_bytes()?)))
    }

    /// The bytes of the key, for code that makes its own HMACs under it (the CSRF token of
    /// `bb-auth`). They are wiped from memory when dropped.
    #[must_use]
    pub fn to_bytes(&self) -> Zeroizing<[u8; KEY_LENGTH]> {
        self.0.clone()
    }

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

/// What was read from the master file, and how the file looked then.
struct State {
    keys: Arc<Vec<MasterKey>>,
    stamp: Stamp,
}

/// The modification time and size of a file: when either changes, the file is read again.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Stamp(Option<SystemTime>, u64);

impl Stamp {
    fn of(path: &Path) -> Result<Self, StoreError> {
        let metadata = fs::metadata(path).map_err(StoreError::KeyFile)?;
        Ok(Self(metadata.modified().ok(), metadata.len()))
    }
}

/// The keys of this installation.
pub struct KeyRing {
    folder: PathBuf,
    log_key: SigningKey,
    csrf_key: SigningKey,
    state: RwLock<State>,
}

impl std::fmt::Debug for KeyRing {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("KeyRing")
            .field("folder", &self.folder)
            .field("keys", &"<hidden>")
            .finish_non_exhaustive()
    }
}

impl KeyRing {
    /// Reads the key files in `folder` (the `keys/` folder of the data folder). A file that is
    /// missing is made when `create` is set (the first start) and is an error otherwise.
    ///
    /// # Errors
    /// Fails with [`StoreError::KeysMissing`] for a missing file when `create` is not set, when a
    /// file cannot be read or written or is damaged, and (on Unix) when a file can be read by
    /// anybody but its owner.
    pub fn open(folder: &Path, create: bool) -> Result<Self, StoreError> {
        let master_path = folder.join(MASTER_FILE);
        let log_path = folder.join(LOG_FILE);
        if !create && (!master_path.exists() || !log_path.exists()) {
            return Err(StoreError::KeysMissing);
        }
        if create {
            fs::create_dir_all(folder).map_err(StoreError::KeyFile)?;
        }
        let keys = if let Some(text) = read_complete(&master_path)? {
            parse_master(&text)?
        } else {
            let first = MasterKey::generate(key_id(1)?)?;
            write_first(&master_path, &render_master(std::slice::from_ref(&first)))?;
            // Whoever made the file first wins; read what is really there.
            parse_master(&wait_for(&master_path)?)?
        };
        let log_key = if let Some(text) = read_complete(&log_path)? {
            parse_log(&text)?
        } else {
            write_first(&log_path, &render_log(&SigningKey::generate()?))?;
            parse_log(&wait_for(&log_path)?)?
        };
        // Nothing stored depends on the CSRF key, so it is made whenever it is missing.
        let csrf_path = folder.join(CSRF_FILE);
        let csrf_key = if let Some(text) = read_complete(&csrf_path)? {
            parse_log(&text)?
        } else {
            write_first(&csrf_path, &render_log(&SigningKey::generate()?))?;
            parse_log(&wait_for(&csrf_path)?)?
        };
        let stamp = Stamp::of(&master_path)?;
        Ok(Self {
            folder: folder.to_owned(),
            log_key,
            csrf_key,
            state: RwLock::new(State {
                keys: Arc::new(keys),
                stamp,
            }),
        })
    }

    /// [`Self::open`] that makes missing files. For tests and tools that have no database to ask;
    /// the server uses [`Database::open_keys`](crate::Database::open_keys).
    ///
    /// # Errors
    /// The errors of [`Self::open`].
    pub fn open_or_create(folder: &Path) -> Result<Self, StoreError> {
        Self::open(folder, true)
    }

    fn master_path(&self) -> PathBuf {
        self.folder.join(MASTER_FILE)
    }

    /// The files a backup of the keys must hold: the master keys, which read every stored secret,
    /// and the log key, which verifies the security log. (The CSRF key is made again when it is
    /// missing and protects nothing that is stored.)
    #[must_use]
    pub fn backup_files(&self) -> [PathBuf; 2] {
        [self.master_path(), self.folder.join(LOG_FILE)]
    }

    /// The master keys as the file has them now.
    fn current(&self) -> Result<Arc<Vec<MasterKey>>, StoreError> {
        let path = self.master_path();
        let stamp = Stamp::of(&path)?;
        {
            let state = self.read_state()?;
            if state.stamp == stamp {
                return Ok(Arc::clone(&state.keys));
            }
        }
        let text = read_complete(&path)?.ok_or(StoreError::KeysMissing)?;
        let keys = Arc::new(parse_master(&text)?);
        let mut state = self
            .state
            .write()
            .map_err(|_| StoreError::Secret(SecretProblem::BadKeyFile))?;
        state.keys = Arc::clone(&keys);
        state.stamp = stamp;
        Ok(keys)
    }

    fn read_state(&self) -> Result<std::sync::RwLockReadGuard<'_, State>, StoreError> {
        self.state
            .read()
            .map_err(|_| StoreError::Secret(SecretProblem::BadKeyFile))
    }

    /// The key that encrypts new values, as the key file has it now.
    ///
    /// # Errors
    /// Fails when the key file cannot be read.
    pub fn newest(&self) -> Result<MasterKey, StoreError> {
        self.current()?
            .last()
            .cloned()
            .ok_or(StoreError::Secret(SecretProblem::BadKeyFile))
    }

    /// The key with this ID, if the file has it.
    pub(crate) fn get(&self, id: &TechnicalName) -> Result<Option<MasterKey>, StoreError> {
        Ok(self.current()?.iter().find(|key| key.id() == id).cloned())
    }

    /// The IDs of all keys, oldest first.
    ///
    /// # Errors
    /// Fails when the key file cannot be read.
    pub fn ids(&self) -> Result<Vec<TechnicalName>, StoreError> {
        Ok(self.current()?.iter().map(|key| key.id.clone()).collect())
    }

    /// The log key that signs the security log. It never changes.
    #[must_use]
    pub fn log_key(&self) -> SigningKey {
        self.log_key.clone()
    }

    /// The CSRF key that the token of a session is made with. It never changes.
    #[must_use]
    pub fn csrf_key(&self) -> SigningKey {
        self.csrf_key.clone()
    }

    /// Adds a new key and saves the file. Old keys stay, so everything still decrypts. This is
    /// the first step of a rotation; the second wraps the secrets again
    /// (`Database::rotate_master_key`).
    pub(crate) fn add_key(&self) -> Result<(), StoreError> {
        let mut keys = self.current()?.as_ref().clone();
        let number = keys.last().map_or(1, |key| id_number(key.id()) + 1);
        keys.push(MasterKey::generate(key_id(number)?)?);
        replace_master(&self.folder, &keys)?;
        self.current().map(|_| ())
    }

    /// Removes the keys that nothing uses any more (never the newest) and saves the file.
    pub(crate) fn retire_unused(&self, in_use: &[TechnicalName]) -> Result<(), StoreError> {
        let keys = self.current()?;
        let newest = keys
            .last()
            .map(|key| key.id().clone())
            .ok_or(StoreError::Secret(SecretProblem::BadKeyFile))?;
        let kept: Vec<MasterKey> = keys
            .iter()
            .filter(|key| key.id() == &newest || in_use.contains(key.id()))
            .cloned()
            .collect();
        if kept.len() != keys.len() {
            replace_master(&self.folder, &kept)?;
            self.current()?;
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

/// The text of a key file that is complete (ends with a new line), or `None` when there is no such
/// file. A file that is empty or cut short is being written by another process that started at
/// the same moment, so this waits for it. The bytes are wiped when the result is dropped.
fn read_complete(path: &Path) -> Result<Option<Zeroizing<String>>, StoreError> {
    match read_file(path)? {
        Some(text) if text.ends_with('\n') => Ok(Some(text)),
        Some(_) => wait_for(path).map(Some),
        None => Ok(None),
    }
}

/// The text of a key file, or `None` when there is no such file. The bytes are wiped when the
/// result is dropped.
fn read_file(path: &Path) -> Result<Option<Zeroizing<String>>, StoreError> {
    match fs::read(path) {
        Ok(bytes) => {
            let bytes = Zeroizing::new(bytes);
            check_permissions(path)?;
            let text = std::str::from_utf8(&bytes)
                .map_err(|_| StoreError::Secret(SecretProblem::BadKeyFile))?;
            Ok(Some(Zeroizing::new(text.to_owned())))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(StoreError::KeyFile(error)),
    }
}

/// Reads a file that another process may still be writing: tries again until it is complete or
/// the patience is over.
fn wait_for(path: &Path) -> Result<Zeroizing<String>, StoreError> {
    let started = std::time::Instant::now();
    loop {
        if let Some(text) = read_file(path)?
            && text.ends_with('\n')
        {
            return Ok(text);
        }
        if started.elapsed() > WINNER_PATIENCE {
            return Err(StoreError::Secret(SecretProblem::BadKeyFile));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn parse_master(text: &str) -> Result<Vec<MasterKey>, StoreError> {
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

fn parse_log(text: &str) -> Result<SigningKey, StoreError> {
    from_hex(text.trim())
        .map(SigningKey)
        .ok_or(StoreError::Secret(SecretProblem::BadKeyFile))
}

fn render_master(keys: &[MasterKey]) -> Zeroizing<String> {
    let mut text = Zeroizing::new(String::new());
    for key in keys {
        text.push_str(key.id().as_str());
        text.push(' ');
        text.push_str(&to_hex(key.bytes()));
        text.push('\n');
    }
    text
}

fn render_log(key: &SigningKey) -> Zeroizing<String> {
    let mut text = to_hex(key.0.as_slice());
    text.push('\n');
    text
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

/// Makes the file with this content, unless it exists already. A process that loses the race
/// leaves the winner's file alone (the caller reads it with [`wait_for`]).
fn write_first(path: &Path, text: &str) -> Result<(), StoreError> {
    match create_private(path) {
        Ok(mut file) => {
            file.write_all(text.as_bytes())
                .map_err(StoreError::KeyFile)?;
            file.sync_all().map_err(StoreError::KeyFile)?;
            sync_folder(path);
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(StoreError::KeyFile(error)),
    }
}

/// Replaces `master.key` with a new file in one step, so a crash leaves the old or the new
/// content, and makes the replacement durable.
fn replace_master(folder: &Path, keys: &[MasterKey]) -> Result<(), StoreError> {
    let temporary = folder.join(NEW_MASTER_FILE);
    let _ = fs::remove_file(&temporary);
    let mut file = create_private(&temporary).map_err(StoreError::KeyFile)?;
    file.write_all(render_master(keys).as_bytes())
        .map_err(StoreError::KeyFile)?;
    file.sync_all().map_err(StoreError::KeyFile)?;
    drop(file);
    fs::rename(&temporary, folder.join(MASTER_FILE)).map_err(StoreError::KeyFile)?;
    sync_folder(&folder.join(MASTER_FILE));
    Ok(())
}

/// Makes the change of a folder durable (Unix). Failing to do so is not worth stopping for.
fn sync_folder(file: &Path) {
    #[cfg(unix)]
    if let Some(folder) = file.parent()
        && let Ok(handle) = fs::File::open(folder)
    {
        let _ = handle.sync_all();
    }
    #[cfg(not(unix))]
    let _ = file;
}

#[cfg(unix)]
fn create_private(path: &Path) -> std::io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt as _;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}

// On Windows the file inherits the owner-only access of the `keys/` folder (WP-04).
#[cfg(not(unix))]
fn create_private(path: &Path) -> std::io::Result<fs::File> {
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
}

/// Refuses a key file that others can read. Only Unix has file modes; on Windows the file has
/// the owner-only access of the `keys/` folder it is in (M1-F22).
fn check_permissions(path: &Path) -> Result<(), StoreError> {
    let metadata = fs::metadata(path).map_err(StoreError::KeyFile)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(StoreError::KeyFile(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "a key file must be readable by its owner only (mode 0600)",
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

    fn id(number: u64) -> TechnicalName {
        key_id(number).unwrap()
    }

    #[test]
    fn the_first_start_makes_both_files_and_the_next_start_reads_them() {
        let folder = tempfile::tempdir().unwrap();
        let first = KeyRing::open_or_create(folder.path()).unwrap();
        assert_eq!(first.ids().unwrap(), [id(1)]);
        let again = KeyRing::open(folder.path(), false).unwrap();
        assert_eq!(
            again.newest().unwrap().bytes(),
            first.newest().unwrap().bytes()
        );
        let probe: &[u8] = b"probe";
        assert_eq!(
            again.log_key().sign(&[probe]).unwrap(),
            first.log_key().sign(&[probe]).unwrap()
        );
        let text = fs::read_to_string(folder.path().join(MASTER_FILE)).unwrap();
        assert!(
            text.starts_with("k1 ") && text.len() == 3 + 64 + 1,
            "{} bytes",
            text.len()
        );
        assert_eq!(
            fs::read_to_string(folder.path().join(LOG_FILE))
                .unwrap()
                .len(),
            65
        );
    }

    #[test]
    fn sec_16_missing_key_files_are_not_made_again_unless_it_is_the_first_start() {
        let folder = tempfile::tempdir().unwrap();
        assert!(matches!(
            KeyRing::open(folder.path(), false),
            Err(StoreError::KeysMissing)
        ));
        KeyRing::open(folder.path(), true).unwrap();
        fs::remove_file(folder.path().join(LOG_FILE)).unwrap();
        assert!(matches!(
            KeyRing::open(folder.path(), false),
            Err(StoreError::KeysMissing)
        ));
        assert!(!folder.path().join(LOG_FILE).exists(), "nothing was made");
    }

    #[test]
    fn a_key_file_that_is_still_being_written_is_waited_for() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().to_owned();
        let first = KeyRing::open_or_create(&path).unwrap();
        let expected = *first.newest().unwrap().bytes();
        // The writer made the file but has not written it yet.
        let master = path.join(MASTER_FILE);
        let content = fs::read(&master).unwrap();
        fs::write(&master, "").unwrap();
        let writer = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            fs::write(&master, content).unwrap();
        });
        let second = KeyRing::open(&path, false).unwrap();
        writer.join().unwrap();
        assert_eq!(*second.newest().unwrap().bytes(), expected);
    }

    #[test]
    fn two_starts_at_the_same_moment_end_with_the_same_keys() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().to_owned();
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let path = path.clone();
                std::thread::spawn(move || {
                    let ring = KeyRing::open_or_create(&path).unwrap();
                    let key = ring.newest().unwrap();
                    (*key.bytes(), ring.log_key().sign(&[b"x"]).unwrap())
                })
            })
            .collect();
        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        assert!(results.windows(2).all(|pair| pair[0] == pair[1]));
    }

    #[test]
    fn adding_a_key_keeps_the_old_one_and_retiring_drops_unused_old_ones() {
        let folder = tempfile::tempdir().unwrap();
        let ring = KeyRing::open_or_create(folder.path()).unwrap();
        ring.add_key().unwrap();
        ring.add_key().unwrap();
        assert_eq!(ring.ids().unwrap(), [id(1), id(2), id(3)]);
        assert_eq!(
            KeyRing::open_or_create(folder.path())
                .unwrap()
                .ids()
                .unwrap()
                .len(),
            3
        );
        ring.retire_unused(&[id(1)]).unwrap();
        assert_eq!(
            ring.ids().unwrap(),
            [id(1), id(3)],
            "the newest key is never retired"
        );
    }

    #[test]
    fn a_ring_notices_when_another_process_rotated_the_keys() {
        let folder = tempfile::tempdir().unwrap();
        let server = KeyRing::open_or_create(folder.path()).unwrap();
        let command_line = KeyRing::open_or_create(folder.path()).unwrap();
        command_line.add_key().unwrap();
        command_line.retire_unused(&[]).unwrap();
        // The server never restarted, but it no longer encrypts with the retired key.
        assert_eq!(server.newest().unwrap().id(), &id(2));
        assert_eq!(server.ids().unwrap(), [id(2)]);
        assert!(server.get(&id(1)).unwrap().is_none());
    }

    #[test]
    fn bad_example_a_damaged_key_file_is_refused() {
        let zeros = "0".repeat(64);
        let ones = "1".repeat(64);
        for text in [
            String::new(),
            "k1\n".to_owned(),
            "k1 abcd\n".to_owned(),
            format!("K1 {zeros}\n"),
            format!("k1 {zeros}\nk1 {ones}\n"),
            format!("k1 {}\n", "zz".repeat(32)),
        ] {
            assert!(parse_master(&text).is_err(), "{text}");
        }
        assert!(parse_master(&format!("k1 {}\n", "ab".repeat(32))).is_ok());
        assert!(parse_log("abcd\n").is_err());
    }

    #[test]
    fn keys_never_show_in_debug_output() {
        let folder = tempfile::tempdir().unwrap();
        let ring = KeyRing::open_or_create(folder.path()).unwrap();
        let key = ring.newest().unwrap();
        let shown = format!("{ring:?} {key:?} {:?}", ring.log_key());
        let hex = to_hex(key.bytes());
        assert!(!shown.contains(hex.as_str()));
        assert!(shown.contains("<hidden>"));
    }

    #[test]
    fn the_signing_key_signs_and_checks_in_a_way_that_depends_on_every_byte() {
        let key = SigningKey::generate().unwrap();
        let signature = key.sign(&[b"a", b"bc"]).unwrap();
        assert!(key.verify(&[b"abc"], &signature));
        assert!(!key.verify(&[b"xyz"], &signature));
        assert!(
            !SigningKey::generate()
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
        for name in [MASTER_FILE, LOG_FILE] {
            let path = folder.path().join(name);
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
            fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
            assert!(KeyRing::open_or_create(folder.path()).is_err());
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        }
    }
}
