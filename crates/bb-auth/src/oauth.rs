//! The secrets of an AI app signing in (AI-22, ADR-0042): the code, the access token and the
//! renewing token, and the PKCE proof that ties a code to the app that asked for it.
//!
//! Each secret is 256 random bits like a session. A short start tells the kinds apart and keeps an
//! access token from being taken for a session cookie or an access key (`bb_`). Each has its own
//! hash domain, so the hash of a code can never be taken for the hash of a token.

use base64ct::{Base64UrlUnpadded, Encoding as _};
use bb_core::oauth::TokenKind;
use bb_core::security_log::Digest;
use sha2::{Digest as _, Sha256};
use subtle::ConstantTimeEq as _;

use crate::error::AuthError;
use crate::token::{Purpose, SecretToken};

/// What a secret of the sign-in of an app is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OAuthKind {
    /// The code an app gets after *Allow*; it works once.
    Code,
    /// The token an app sends with every call.
    Access,
    /// The token that renews the access token.
    Refresh,
}

impl OAuthKind {
    /// How a secret of this kind starts.
    #[must_use]
    pub const fn start(self) -> &'static str {
        match self {
            Self::Code => "bbc_",
            Self::Access => "bba_",
            Self::Refresh => "bbr_",
        }
    }

    const fn purpose(self) -> Purpose {
        match self {
            Self::Code => Purpose::OAuthCode,
            Self::Access => Purpose::OAuthAccess,
            Self::Refresh => Purpose::OAuthRefresh,
        }
    }
}

impl From<TokenKind> for OAuthKind {
    fn from(kind: TokenKind) -> Self {
        match kind {
            TokenKind::Access => Self::Access,
            TokenKind::Refresh => Self::Refresh,
        }
    }
}

/// Whether `text` starts like a secret of the sign-in of an app. It says nothing about whether it
/// works.
#[must_use]
pub fn looks_like_oauth_secret(text: &str) -> bool {
    [OAuthKind::Code, OAuthKind::Access, OAuthKind::Refresh]
        .iter()
        .any(|kind| text.starts_with(kind.start()))
}

/// Whether `text` starts like an access token (what an app sends in `Authorization: Bearer`).
#[must_use]
pub fn looks_like_oauth_access_token(text: &str) -> bool {
    text.starts_with(OAuthKind::Access.start())
}

/// A code or a token. Wiped from memory when dropped and never shown in `Debug` output.
#[derive(Clone)]
pub struct OAuthSecret {
    kind: OAuthKind,
    token: SecretToken,
}

impl std::fmt::Debug for OAuthSecret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("OAuthSecret(<hidden>)")
    }
}

impl OAuthSecret {
    /// A new secret of `kind`.
    ///
    /// # Errors
    /// Fails when the system gives no secure random bytes.
    pub fn generate(kind: OAuthKind) -> Result<Self, AuthError> {
        Ok(Self {
            kind,
            token: SecretToken::generate()?,
        })
    }

    /// The secret as sent once: its start and the 43 characters of the secret.
    #[must_use]
    pub fn encode(&self) -> String {
        format!("{}{}", self.kind.start(), self.token.encode())
    }

    /// The secret written by [`Self::encode`], if it is one of `kind`.
    #[must_use]
    pub fn parse(kind: OAuthKind, text: &str) -> Option<Self> {
        let rest = text.strip_prefix(kind.start())?;
        Some(Self {
            kind,
            token: SecretToken::parse(rest)?,
        })
    }

    /// The hash that is stored in place of the secret.
    #[must_use]
    pub fn hash(&self) -> Digest {
        self.token.hash(self.kind.purpose())
    }
}

/// The challenge of a PKCE verifier by the method `S256`: the SHA-256 of the verifier, written as
/// base64url without padding (RFC 7636 §4.2).
#[must_use]
pub fn pkce_challenge(verifier: &str) -> String {
    Base64UrlUnpadded::encode_string(&Sha256::digest(verifier.as_bytes()))
}

/// Whether `verifier` is a well-formed PKCE verifier (43 to 128 characters of the unreserved
/// set, RFC 7636 §4.1).
#[must_use]
pub fn is_pkce_verifier(verifier: &str) -> bool {
    (43..=128).contains(&verifier.len())
        && verifier
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~'))
}

/// Whether `challenge` looks like what [`pkce_challenge`] makes: 43 characters of base64url.
#[must_use]
pub fn is_pkce_challenge(challenge: &str) -> bool {
    challenge.len() == 43
        && challenge
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

/// Whether the `verifier` proves the `challenge`, in constant time.
#[must_use]
pub fn pkce_proves(challenge: &str, verifier: &str) -> bool {
    is_pkce_verifier(verifier)
        && pkce_challenge(verifier)
            .as_bytes()
            .ct_eq(challenge.as_bytes())
            .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_22_a_secret_has_its_start_and_a_hash_of_its_own_kind() {
        let secret = OAuthSecret::generate(OAuthKind::Access).unwrap();
        let text = secret.encode();
        assert!(text.starts_with("bba_"));
        assert!(looks_like_oauth_access_token(&text));
        assert!(!crate::looks_like_access_key(&text));
        let again = OAuthSecret::parse(OAuthKind::Access, &text).unwrap();
        assert_eq!(secret.hash(), again.hash());
        assert!(OAuthSecret::parse(OAuthKind::Refresh, &text).is_none());
        let other = OAuthSecret::parse(OAuthKind::Access, &text).unwrap();
        assert_ne!(
            other.hash(),
            OAuthSecret::generate(OAuthKind::Access).unwrap().hash()
        );
    }

    #[test]
    fn ai_22_the_example_of_rfc_7636_is_proved() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
        assert_eq!(pkce_challenge(verifier), challenge);
        assert!(pkce_proves(challenge, verifier));
        assert!(!pkce_proves(
            challenge,
            "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXl"
        ));
        assert!(!pkce_proves(challenge, "short"));
    }
}
