//! Random 256-bit secrets and how they are stored: sessions (SEC-12), invitations (AUTH-06).
//!
//! A secret is shown or sent once (cookie, link) and only its hash is stored. The secrets are
//! 256 bits of fresh randomness, so a plain SHA-256 is enough: there is nothing to guess and
//! nothing to brute-force. Each purpose has its own hash domain, so the hash of a session secret
//! can never be taken for the hash of an invitation.

use base64ct::{Base64UrlUnpadded, Encoding as _};
use bb_core::security_log::Digest;
use sha2::{Digest as _, Sha256};
use subtle::ConstantTimeEq as _;
use zeroize::Zeroizing;

use crate::error::{AuthError, random};

/// Length of a secret in bytes (256 bits).
const LENGTH: usize = 32;
/// Length of a secret written as base64url without padding.
const ENCODED_LENGTH: usize = 43;

/// What a secret or a code is for. It is part of the hash, so hashes of different purposes never
/// match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Purpose {
    /// The secret in the session cookie.
    Session,
    /// The secret in an invitation link.
    Invitation,
    /// The setup code of the first start.
    SetupCode,
    /// A backup code.
    BackupCode,
    /// The secret of an access key.
    AccessKey,
}

impl Purpose {
    const fn domain(self) -> &'static [u8] {
        match self {
            Self::Session => b"bugaboo.session.v1",
            Self::Invitation => b"bugaboo.invitation.v1",
            Self::SetupCode => b"bugaboo.setup-code.v1",
            Self::BackupCode => b"bugaboo.backup-code.v1",
            Self::AccessKey => b"bugaboo.access-key.v1",
        }
    }

    /// The hash of `bytes` for this purpose.
    pub(crate) fn hash(self, bytes: &[u8]) -> Digest {
        let mut hasher = Sha256::new();
        hasher.update(self.domain());
        hasher.update([0_u8]);
        hasher.update(bytes);
        Digest::new(hasher.finalize().into())
    }
}

/// A random 256-bit secret. It is wiped from memory when dropped and never shows in `Debug`.
#[derive(Clone)]
pub struct SecretToken(Zeroizing<[u8; LENGTH]>);

impl std::fmt::Debug for SecretToken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SecretToken(<hidden>)")
    }
}

impl SecretToken {
    /// A new secret.
    ///
    /// # Errors
    /// Fails when the system gives no secure random bytes.
    pub fn generate() -> Result<Self, AuthError> {
        Ok(Self(Zeroizing::new(random()?)))
    }

    /// The secret as text for a cookie or a link (base64url, 43 characters).
    #[must_use]
    pub fn encode(&self) -> String {
        Base64UrlUnpadded::encode_string(self.0.as_slice())
    }

    /// The secret written by [`Self::encode`]; `None` for anything else.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        if text.len() != ENCODED_LENGTH {
            return None;
        }
        let mut bytes = Zeroizing::new([0_u8; LENGTH]);
        let decoded = Base64UrlUnpadded::decode(text, bytes.as_mut_slice()).ok()?;
        (decoded.len() == LENGTH).then_some(Self(bytes))
    }

    /// The hash that is stored in place of the secret.
    #[must_use]
    pub fn hash(&self, purpose: Purpose) -> Digest {
        purpose.hash(self.0.as_slice())
    }

    pub(crate) fn bytes(&self) -> &[u8; LENGTH] {
        &self.0
    }
}

/// 16 bytes made from the name of a fixed thing (a setting, the setup code of the first start), so
/// every event about it names the same record. Not a secret; only a stable ID.
#[must_use]
pub fn stable_id(kind: &str, name: &str) -> [u8; 16] {
    let mut hasher = Sha256::new();
    hasher.update(b"bugaboo.fixed-record.v1");
    hasher.update([0_u8]);
    hasher.update(kind.as_bytes());
    hasher.update([0_u8]);
    hasher.update(name.as_bytes());
    let hash = hasher.finalize();
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&hash[..16]);
    bytes
}

/// Whether two hashes are the same; takes the same time whatever they are.
#[must_use]
pub fn same_digest(a: &Digest, b: &Digest) -> bool {
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adr_0007_a_fixed_record_has_one_id_that_depends_on_its_kind_and_name() {
        assert_eq!(stable_id("setting", "a"), stable_id("setting", "a"));
        assert_ne!(stable_id("setting", "a"), stable_id("setting", "b"));
        assert_ne!(stable_id("setting", "a"), stable_id("setup_code", "a"));
    }

    #[test]
    fn sec_12_a_secret_survives_text_and_is_different_every_time() {
        let first = SecretToken::generate().unwrap();
        let second = SecretToken::generate().unwrap();
        assert_ne!(first.encode(), second.encode());
        assert_eq!(first.encode().len(), ENCODED_LENGTH);
        let again = SecretToken::parse(&first.encode()).unwrap();
        assert_eq!(first.hash(Purpose::Session), again.hash(Purpose::Session));
    }

    #[test]
    fn sec_12_text_that_is_not_a_secret_is_refused() {
        let good = SecretToken::generate().unwrap().encode();
        for bad in [
            "",
            "short",
            &format!("{good}A"),
            &good[..42],
            &good.replace(&good[..1], "+"),
        ] {
            assert!(SecretToken::parse(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn sec_12_hashes_of_different_purposes_never_match() {
        let secret = SecretToken::generate().unwrap();
        assert_ne!(
            secret.hash(Purpose::Session),
            secret.hash(Purpose::Invitation)
        );
        assert!(same_digest(
            &secret.hash(Purpose::Session),
            &secret.hash(Purpose::Session)
        ));
        assert!(!same_digest(
            &secret.hash(Purpose::Session),
            &secret.hash(Purpose::Invitation)
        ));
    }

    #[test]
    fn sec_16_a_secret_never_shows_in_debug_output() {
        let secret = SecretToken::generate().unwrap();
        assert!(!format!("{secret:?}").contains(&secret.encode()));
    }
}
