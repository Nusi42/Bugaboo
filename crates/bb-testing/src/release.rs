//! Keys made for a test, to sign files the way a release is signed (SEC-18, ADR-0047 decision 2:
//! tests make their own key pairs and never touch the real keys).

use minisign::{KeyPair, PError, PublicKey, SecretKey};

/// A key pair that lives only as long as the test; its secret key is never written anywhere.
pub struct TestKey {
    public: PublicKey,
    secret: SecretKey,
}

impl TestKey {
    /// A new key pair.
    ///
    /// # Errors
    /// When the system gives no random bytes.
    pub fn new() -> Result<Self, PError> {
        let KeyPair { pk, sk } = KeyPair::generate_unencrypted_keypair()?;
        Ok(Self {
            public: pk,
            secret: sk,
        })
    }

    /// The public key in the minisign format, as `RELEASE_PUBLIC_KEYS` holds one.
    #[must_use]
    pub fn public(&self) -> String {
        self.public.to_base64()
    }

    /// The text of the `.minisig` file that signs `bytes` with `trusted_comment`.
    ///
    /// # Errors
    /// When the system gives no random bytes.
    pub fn sign(&self, bytes: &[u8], trusted_comment: &str) -> Result<String, PError> {
        Ok(minisign::sign(
            Some(&self.public),
            &self.secret,
            bytes,
            Some(trusted_comment),
            None,
        )?
        .into_string())
    }
}
