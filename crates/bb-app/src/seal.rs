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
//!
//! It lives here, beside the use cases, so the cursors that leave by the API and those that leave
//! by a tool of AI apps (`search_totals`) are sealed by the one key and the one rule
//! ([`seal_cursor`], [`open_cursor`]); `bb-api` builds its cursors and `ETag`s on it.

use std::sync::OnceLock;

use base64ct::{Base64UrlUnpadded, Encoding as _};
use bb_core::id::UserId;
use hmac::{Hmac, KeyInit as _, Mac as _};
use sha2::{Digest as _, Sha256};

type HmacSha256 = Hmac<Sha256>;

/// How many bytes of the HMAC are kept.
pub const TAG_BYTES: usize = 16;
const KEY_BYTES: usize = 32;

/// A key and what it seals.
pub struct Seal {
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
    #[must_use]
    pub fn tag(&self, purpose: &str, data: &[u8]) -> Option<[u8; TAG_BYTES]> {
        let full: [u8; 32] = self.mac(purpose, data)?.finalize().into_bytes().into();
        let mut tag = [0_u8; TAG_BYTES];
        tag.copy_from_slice(&full[..TAG_BYTES]);
        Some(tag)
    }

    /// Whether `tag` is the tag of `data` for `purpose`. Takes the same time whatever the tag is.
    #[must_use]
    pub fn verify(&self, purpose: &str, data: &[u8], tag: &[u8]) -> bool {
        tag.len() == TAG_BYTES
            && self
                .mac(purpose, data)
                .is_some_and(|mac| mac.verify_truncated_left(tag).is_ok())
    }
}

/// The purpose the tag of a cursor is bound to, before the name of the list.
const PURPOSE_PREFIX: &str = "cursor:";

/// The name of a list that cursors are sealed for: what kind of list it is (`search`, `totals`,
/// `timeline`, `board`), the person, and a hash of the parts of the request that make it this list,
/// so a cursor opens for nobody else and for no other list. Each part goes in after its length,
/// so two different splits of the same text are two lists; a part that is left out counts as
/// empty. The one name of every list.
#[must_use]
pub fn list_name(kind: &str, person: UserId, parts: &[Option<&str>]) -> String {
    let mut hash = Sha256::new();
    for part in parts {
        let bytes = part.unwrap_or_default().as_bytes();
        hash.update(u64::try_from(bytes.len()).unwrap_or(u64::MAX).to_be_bytes());
        hash.update(bytes);
    }
    format!(
        "{kind}:{person}:{}",
        Base64UrlUnpadded::encode_string(&hash.finalize())
    )
}

/// The purpose of the cursors of `list`.
#[must_use]
pub fn cursor_purpose(list: &str) -> String {
    format!("{PURPOSE_PREFIX}{list}")
}

/// The cursor text for a page of `list` that ends at `position`: the tag, then the position, as
/// URL-safe text. `None` when the system has no random bytes for the key of the seal.
#[must_use]
pub fn seal_cursor(list: &str, position: &str) -> Option<String> {
    let tag = process()?.tag(&cursor_purpose(list), position.as_bytes())?;
    let mut bytes = tag.to_vec();
    bytes.extend_from_slice(position.as_bytes());
    Some(Base64UrlUnpadded::encode_string(&bytes))
}

/// The position inside a cursor of `list`; `None` when the text is not a cursor this server run
/// made for this list.
#[must_use]
pub fn open_cursor(list: &str, cursor: &str) -> Option<String> {
    let bytes = Base64UrlUnpadded::decode_vec(cursor).ok()?;
    let (tag, position) = bytes.split_at_checked(TAG_BYTES)?;
    if !process().is_some_and(|seal| seal.verify(&cursor_purpose(list), position, tag)) {
        return None;
    }
    String::from_utf8(position.to_vec()).ok()
}

/// The seal of this server run: made on first use, the same until the server stops.
/// Only `bb-api` reads it itself, for its `ETag`s; cursors go through [`seal_cursor`] and
/// [`open_cursor`].
pub fn process() -> Option<&'static Seal> {
    static SEAL: OnceLock<Option<Seal>> = OnceLock::new();
    SEAL.get_or_init(Seal::new).as_ref()
}

#[cfg(test)]
mod tests {

    #[test]
    fn sec_list_names_of_two_splits_of_the_same_text_differ() {
        let person = UserId::from_random([7; 16]);
        let one = list_name("board", person, &[Some("ab"), Some("c")]);
        let other = list_name("board", person, &[Some("a"), Some("bc")]);
        assert_ne!(one, other);
        let nul = list_name("board", person, &[Some("a\u{0}"), Some("b")]);
        let split = list_name("board", person, &[Some("a"), Some("\u{0}b")]);
        assert_ne!(nul, split);
        assert_eq!(one, list_name("board", person, &[Some("ab"), Some("c")]));
    }

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
