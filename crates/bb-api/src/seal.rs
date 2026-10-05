//! The seal: a keyed checksum for what the server hands to a client and takes back (PERF-02,
//! PERF-03).
//!
//! A cursor and an `ETag` carry a tag made with `HMAC-SHA256` (cut to 16 bytes) under a random key
//! that is made when the server starts and kept in memory only. A cursor or tag made before a
//! restart is simply not recognised afterwards: a client asks for the first page again and
//! fetches the answer again, which costs nothing that matters. The key is why a client cannot make
//! a cursor of its own, and why a tag tells nothing about the content it stands for.
//!
//! The seal says "this server made this text", not "this person may see this": permissions come
//! from the request every time.

use std::sync::OnceLock;

use hmac::{Hmac, KeyInit as _, Mac as _};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// How many bytes of the HMAC are kept.
pub(crate) const TAG_BYTES: usize = 16;
const KEY_BYTES: usize = 32;

/// A key and what it seals.
pub(crate) struct Seal {
    key: [u8; KEY_BYTES],
}

impl Seal {
    /// A seal with a new random key, or `None` when the system has no random bytes to give.
    fn new() -> Option<Self> {
        let mut key = [0_u8; KEY_BYTES];
        getrandom::fill(&mut key).ok()?;
        Some(Self { key })
    }

    /// A seal with a key of the caller's choosing, for tests that need to tell two keys apart.
    #[cfg(test)]
    pub(crate) const fn with_key(key: [u8; KEY_BYTES]) -> Self {
        Self { key }
    }

    fn mac(&self, purpose: &str, data: &[u8]) -> Option<HmacSha256> {
        let mut mac = HmacSha256::new_from_slice(&self.key).ok()?;
        // The purpose keeps a tag made for one use (a cursor of the invitations, an ETag) from
        // fitting another.
        mac.update(purpose.as_bytes());
        mac.update(&[0]);
        mac.update(data);
        Some(mac)
    }

    /// The tag of `data` for `purpose`.
    pub(crate) fn tag(&self, purpose: &str, data: &[u8]) -> Option<[u8; TAG_BYTES]> {
        let full: [u8; 32] = self.mac(purpose, data)?.finalize().into_bytes().into();
        let mut tag = [0_u8; TAG_BYTES];
        tag.copy_from_slice(&full[..TAG_BYTES]);
        Some(tag)
    }

    /// Whether `tag` is the tag of `data` for `purpose`. Takes the same time whatever the tag is.
    pub(crate) fn verify(&self, purpose: &str, data: &[u8], tag: &[u8]) -> bool {
        tag.len() == TAG_BYTES
            && self
                .mac(purpose, data)
                .is_some_and(|mac| mac.verify_truncated_left(tag).is_ok())
    }
}

/// The seal of this server run: made on first use, the same until the server stops.
pub(crate) fn process() -> Option<&'static Seal> {
    static SEAL: OnceLock<Option<Seal>> = OnceLock::new();
    SEAL.get_or_init(Seal::new).as_ref()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perf_02_a_tag_fits_only_its_data_its_purpose_and_its_key() {
        let seal = Seal::with_key([1; KEY_BYTES]);
        let tag = seal.tag("cursor", b"position").unwrap();
        assert!(seal.verify("cursor", b"position", &tag));
        assert!(!seal.verify("cursor", b"other position", &tag));
        assert!(!seal.verify("etag", b"position", &tag));
        assert!(!Seal::with_key([2; KEY_BYTES]).verify("cursor", b"position", &tag));
        assert!(!seal.verify("cursor", b"position", &tag[..TAG_BYTES - 1]));
        assert!(!seal.verify("cursor", b"position", &[]));
    }

    #[test]
    fn perf_02_the_seal_of_a_run_is_made_once() {
        let first = process().unwrap();
        let again = process().unwrap();
        assert!(std::ptr::eq(first, again));
    }
}
