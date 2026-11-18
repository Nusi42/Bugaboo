//! What binds a backup to the keys of the installation that made it (ADR-0046, amendment):
//!
//! - the **seal**: an HMAC of the exact bytes of the manifest under the log key, kept in the member
//!   that follows the manifest. The manifest names the SHA-256 of every other member, so the seal
//!   covers the whole backup: a file somebody else wrote, or changed, is refused before anything
//!   of it is put back;
//! - a **check value** of each master key in the manifest, an HMAC of a fixed text under that
//!   key: a restore with a master key of another installation is refused before the replace, even
//!   when the backup holds no stored secret that would show it.
//!
//! Each HMAC starts with a text of its own, so neither can stand for the other, nor for a
//! signature of the security log under the same log key.

use bb_core::backup::manifest::KeyCheck;
use bb_core::hex;
use bb_core::security_log::Digest;

use crate::error::StoreError;
use crate::keys::{KeyRing, SigningKey};

/// What the seal of a backup signs before the manifest.
const SEAL_TEXT: &[u8] = b"bugaboo backup seal\0";
/// What the check value of a master key signs.
const CHECK_TEXT: &[u8] = b"bugaboo backup key check\0";

/// The seal of the manifest `manifest`, as the archive holds it: 64 hex digits.
pub(crate) fn seal(keys: &KeyRing, manifest: &[u8]) -> Result<String, StoreError> {
    Ok(hex::encode(&keys.log_key().sign(&[SEAL_TEXT, manifest])?))
}

/// Whether `seal` is the seal of `manifest` under the keys of this installation.
pub(crate) fn sealed(keys: &KeyRing, manifest: &[u8], seal: &[u8]) -> bool {
    std::str::from_utf8(seal)
        .ok()
        .and_then(|text| hex::decode::<{ Digest::LENGTH }>(text.trim()))
        .is_some_and(|seal| keys.log_key().verify(&[SEAL_TEXT, manifest], &seal))
}

/// The check value of every master key of the installation.
pub(crate) fn key_checks(keys: &KeyRing) -> Result<Vec<KeyCheck>, StoreError> {
    let mut checks = Vec::new();
    for id in keys.ids()? {
        if let Some(key) = keys.get(&id)? {
            let check = SigningKey::from_bytes(key.bytes()).sign(&[CHECK_TEXT])?;
            checks.push(KeyCheck {
                id: id.as_str().to_owned(),
                check: hex::encode(&check),
            });
        }
    }
    Ok(checks)
}

/// Whether the master keys of this installation fit the check values a backup holds: no key of
/// the same ID differs, and the installation has at least one of the backup's keys.
pub(crate) fn keys_fit(keys: &KeyRing, checks: &[KeyCheck]) -> Result<bool, StoreError> {
    let mine = key_checks(keys)?;
    let mut shared = false;
    for check in checks {
        if let Some(own) = mine.iter().find(|own| own.id == check.id) {
            if own.check != check.check {
                return Ok(false);
            }
            shared = true;
        }
    }
    Ok(shared || checks.is_empty())
}
