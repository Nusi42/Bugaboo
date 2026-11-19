//! The keyed digest (M4-F122, PRIV-06): what is kept in place of an e-mail address, a name or the
//! secret of a link, where only "the same one again" has to be recognized.
//!
//! A plain hash of an address lets anybody with a copy of the database confirm a guessed address.
//! This digest is an HMAC-SHA256 under a key of the server, so only the server can make it. It is
//! the same key as the CSRF key of the data folder (`keys/csrf.key`), kept apart by a name: every
//! digest starts with a name of its own, so there is no further key file to make, back up or
//! rotate. What depends on it is short-lived (the memory of sign-up e-mails and the links that
//! confirm a new address live an hour) or is only ever compared (the digests of old values in a
//! correction): if the key file is lost, they stop matching and nothing else breaks.

use bb_core::security_log::Digest;
use hmac::{Hmac, KeyInit as _, Mac as _};
use sha2::Sha256;
use zeroize::Zeroizing;

use crate::csrf::KEY_LENGTH;
use crate::error::AuthError;
use crate::token::{Purpose, SecretToken};

type HmacSha256 = Hmac<Sha256>;

/// Keeps the digest apart from any other use of the key.
const DOMAIN: &[u8] = b"bugaboo.digest.v1";

/// The key of the keyed digest.
#[derive(Clone)]
pub struct DigestKey(Zeroizing<[u8; KEY_LENGTH]>);

impl std::fmt::Debug for DigestKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("DigestKey(<hidden>)")
    }
}

impl DigestKey {
    /// The keyed digest of `bytes` for `purpose`: the one function that makes it. Digests of
    /// different purposes never match.
    ///
    /// # Errors
    /// Fails when the key cannot be used (it never happens with a key of [`KEY_LENGTH`] bytes).
    pub fn digest(&self, purpose: Purpose, bytes: &[u8]) -> Result<Digest, AuthError> {
        let mut mac =
            HmacSha256::new_from_slice(self.0.as_slice()).map_err(|_| AuthError::Hashing)?;
        mac.update(DOMAIN);
        mac.update(purpose.domain());
        mac.update(&[0]);
        mac.update(bytes);
        Ok(Digest::new(mac.finalize().into_bytes().into()))
    }

    /// The keyed digest of an e-mail address, in the form [`crate::EmailAddress`] keeps it.
    ///
    /// # Errors
    /// See [`Self::digest`].
    pub fn address(&self, address: &str) -> Result<Digest, AuthError> {
        self.digest(Purpose::Address, address.as_bytes())
    }

    /// The keyed digest that is stored in place of a secret.
    ///
    /// # Errors
    /// See [`Self::digest`].
    pub fn secret(&self, purpose: Purpose, secret: &SecretToken) -> Result<Digest, AuthError> {
        self.digest(purpose, secret.bytes())
    }
}

impl crate::csrf::CsrfKey {
    /// The key of the keyed digest: the bytes of this key, kept apart from its other uses by the
    /// name that every digest starts with.
    #[must_use]
    pub fn digest_key(&self) -> DigestKey {
        DigestKey(self.bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CsrfKey;

    fn key(byte: u8) -> DigestKey {
        CsrfKey::from_bytes(Zeroizing::new([byte; KEY_LENGTH])).digest_key()
    }

    #[test]
    fn sec_16_the_digest_of_an_address_needs_the_key_of_the_server() {
        let address = "eve@example.test";
        let digest = key(7).address(address).unwrap();
        assert_eq!(digest, key(7).address(address).unwrap());
        // Another server's key, and the plain hash a copy of the database could check a guess
        // against, give something else.
        assert_ne!(digest, key(8).address(address).unwrap());
        assert_ne!(digest, Purpose::Address.hash(address.as_bytes()));
        assert_ne!(digest, key(7).address("zed@example.test").unwrap());
    }

    #[test]
    fn sec_16_digests_of_different_purposes_never_match() {
        let bytes = b"same";
        let key = key(7);
        assert_ne!(
            key.digest(Purpose::Address, bytes).unwrap(),
            key.digest(Purpose::PersonName, bytes).unwrap()
        );
        let secret = SecretToken::generate().unwrap();
        assert_ne!(
            key.secret(Purpose::EmailChange, &secret).unwrap(),
            secret.hash(Purpose::EmailChange)
        );
    }

    #[test]
    fn sec_16_the_key_never_shows_in_debug_output() {
        assert_eq!(format!("{:?}", key(1)), "DigestKey(<hidden>)");
    }
}
