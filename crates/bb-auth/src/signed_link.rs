//! Signed links (ATT-07, ADR-0032): a few bytes that say what a link opens, for whom and until
//! when, with a keyed hash that only this server can make.
//!
//! The key is the CSRF key of the data folder (`keys/csrf.key`); every signature starts with a name
//! of its own ([`DOMAIN`]), so a token of one kind is never valid as the other. The link is not a secret that grants
//! access: it only names what may be asked for. Every use of a link is judged again against the
//! issue (`bb-app`), and the link is for one person.

use base64ct::{Base64UrlUnpadded, Encoding as _};
use hmac::{Hmac, KeyInit as _, Mac as _};
use sha2::Sha256;
use zeroize::Zeroizing;

use crate::csrf::KEY_LENGTH;

type HmacSha256 = Hmac<Sha256>;

/// Keeps the signature apart from any other use of the key.
const DOMAIN: &[u8] = b"bugaboo.link.v1";
/// The length of the signature in bytes.
const SIGNATURE_LENGTH: usize = 32;
/// The longest token that is read: the claim of a link is a few dozen bytes, so a longer one is
/// no link (and is not decoded at all).
const TOKEN_CHARS_MAX: usize = 256;

/// The key links are signed under.
#[derive(Clone)]
pub struct LinkKey(Zeroizing<[u8; KEY_LENGTH]>);

impl std::fmt::Debug for LinkKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("LinkKey(<hidden>)")
    }
}

impl LinkKey {
    /// The key made of these bytes.
    pub(crate) const fn from_bytes(bytes: Zeroizing<[u8; KEY_LENGTH]>) -> Self {
        Self(bytes)
    }

    fn mac(&self, claim: &[u8]) -> Option<HmacSha256> {
        let mut mac = HmacSha256::new_from_slice(self.0.as_slice()).ok()?;
        mac.update(DOMAIN);
        mac.update(claim);
        Some(mac)
    }

    /// The token for `claim`: the claim and its signature, as text for an address.
    #[must_use]
    pub fn sign(&self, claim: &[u8]) -> Option<String> {
        let signature = self.mac(claim)?.finalize().into_bytes();
        let mut bytes = Vec::with_capacity(claim.len() + SIGNATURE_LENGTH);
        bytes.extend_from_slice(claim);
        bytes.extend_from_slice(&signature);
        Some(Base64UrlUnpadded::encode_string(&bytes))
    }

    /// The claim of `token`, if the token is one this key signed. The comparison takes the same
    /// time whatever the token is.
    #[must_use]
    pub fn open(&self, token: &str) -> Option<Vec<u8>> {
        if token.len() > TOKEN_CHARS_MAX {
            return None;
        }
        let bytes = Base64UrlUnpadded::decode_vec(token).ok()?;
        let claim_length = bytes.len().checked_sub(SIGNATURE_LENGTH)?;
        let (claim, signature) = bytes.split_at(claim_length);
        self.mac(claim)?.verify_slice(signature).ok()?;
        Some(claim.to_vec())
    }
}

impl crate::csrf::CsrfKey {
    /// The key links are signed under: the bytes of this key, kept apart from its other use by the
    /// name that every signature starts with.
    #[must_use]
    pub fn link_key(&self) -> LinkKey {
        LinkKey::from_bytes(self.bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> LinkKey {
        LinkKey::from_bytes(Zeroizing::new([byte; KEY_LENGTH]))
    }

    #[test]
    fn att_07_a_signed_claim_is_read_back_and_nothing_else_is() {
        let key = key(3);
        let token = key.sign(b"attachment and person").unwrap();
        assert_eq!(
            key.open(&token).as_deref(),
            Some(&b"attachment and person"[..])
        );
        // Bound to the key.
        assert_eq!(self::key(4).open(&token), None);
        // Bound to the claim: any changed character breaks it.
        let mut changed = token.clone().into_bytes();
        changed[0] = if changed[0] == b'A' { b'B' } else { b'A' };
        assert_eq!(key.open(&String::from_utf8(changed).unwrap()), None);
        for bad in ["", "x", &token[1..], &format!("{token}A"), &"A".repeat(300)] {
            assert_eq!(key.open(bad), None, "{bad}");
        }
    }

    #[test]
    fn sec_16_the_key_never_shows_in_debug_output() {
        assert_eq!(format!("{:?}", key(1)), "LinkKey(<hidden>)");
    }
}
