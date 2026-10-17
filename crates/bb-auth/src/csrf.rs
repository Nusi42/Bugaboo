//! The CSRF token (SEC-12, architecture §7 "CSRF", layer 3).
//!
//! The token of a session is an HMAC-SHA256 of the session secret under a key from the data folder
//! (`keys/csrf.key`). It is derived, never stored: the server can compute the right value for any
//! request that carries a session cookie, and a request is genuine only if the page sent the same
//! value in the `x-csrf-token` header. A new session secret means a new token, so the token is
//! renewed with every new session. The comparison takes the same time whatever the token is.

use base64ct::{Base64UrlUnpadded, Encoding as _};
use hmac::{Hmac, KeyInit as _, Mac as _};
use sha2::Sha256;
use zeroize::Zeroizing;

use crate::error::AuthError;
use crate::token::{ENCODED_LENGTH, LENGTH, SecretToken};

type HmacSha256 = Hmac<Sha256>;

/// Keeps the token apart from any other use of the key.
const DOMAIN: &[u8] = b"bugaboo.csrf.v1";
/// The length of the key in bytes (the content of `keys/csrf.key`).
pub const KEY_LENGTH: usize = LENGTH;

/// The key the token is derived under.
#[derive(Clone)]
pub struct CsrfKey(Zeroizing<[u8; KEY_LENGTH]>);

impl std::fmt::Debug for CsrfKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("CsrfKey(<hidden>)")
    }
}

impl CsrfKey {
    /// The key made of these bytes (the content of `keys/csrf.key`).
    #[must_use]
    pub const fn from_bytes(bytes: Zeroizing<[u8; KEY_LENGTH]>) -> Self {
        Self(bytes)
    }

    /// The bytes of the key, for the key of signed links that is derived from it.
    pub(crate) fn bytes(&self) -> Zeroizing<[u8; KEY_LENGTH]> {
        self.0.clone()
    }

    fn mac(&self, session: &SecretToken) -> Result<HmacSha256, AuthError> {
        let mut mac =
            HmacSha256::new_from_slice(self.0.as_slice()).map_err(|_| AuthError::Hashing)?;
        mac.update(DOMAIN);
        mac.update(session.bytes());
        Ok(mac)
    }

    /// The token for `session`, as text for the `__Host-bb_csrf` cookie.
    ///
    /// # Errors
    /// Fails when the key cannot be used (it never happens with a key of [`KEY_LENGTH`] bytes).
    pub fn token(&self, session: &SecretToken) -> Result<String, AuthError> {
        let bytes: [u8; LENGTH] = self.mac(session)?.finalize().into_bytes().into();
        Ok(Base64UrlUnpadded::encode_string(&bytes))
    }

    /// Whether `presented` (the `x-csrf-token` header) is the token of `session`. Anything that
    /// is not a token is simply `false`.
    #[must_use]
    pub fn verify(&self, session: &SecretToken, presented: &str) -> bool {
        if presented.len() != ENCODED_LENGTH {
            return false;
        }
        let mut bytes = [0_u8; LENGTH];
        let Ok(decoded) = Base64UrlUnpadded::decode(presented, &mut bytes) else {
            return false;
        };
        self.mac(session)
            .is_ok_and(|mac| mac.verify_slice(decoded).is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> CsrfKey {
        CsrfKey::from_bytes(Zeroizing::new([byte; KEY_LENGTH]))
    }

    #[test]
    fn sec_12_the_token_of_a_session_is_accepted_and_nothing_else_is() {
        let key = key(7);
        let session = SecretToken::generate().unwrap();
        let other = SecretToken::generate().unwrap();
        let token = key.token(&session).unwrap();
        assert_eq!(token.len(), ENCODED_LENGTH);
        assert!(key.verify(&session, &token));
        // Bound to the session: the token of another session does not fit.
        assert!(!key.verify(&other, &token));
        // Bound to the key.
        assert!(!self::key(8).verify(&session, &token));
        for bad in [
            "",
            "x",
            &token[..ENCODED_LENGTH - 1],
            &format!("{token}A"),
            &"A".repeat(ENCODED_LENGTH),
        ] {
            assert!(!key.verify(&session, bad), "{bad}");
        }
    }

    #[test]
    fn sec_12_every_new_session_has_a_new_token() {
        let key = key(7);
        let first = key.token(&SecretToken::generate().unwrap()).unwrap();
        let second = key.token(&SecretToken::generate().unwrap()).unwrap();
        assert_ne!(first, second);
    }

    #[test]
    fn sec_16_the_key_never_shows_in_debug_output() {
        assert_eq!(format!("{:?}", key(1)), "CsrfKey(<hidden>)");
    }
}
