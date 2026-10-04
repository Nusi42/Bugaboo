//! Stored secrets (SEC-16): mail password, AI keys, OIDC client secrets, webhook signing keys.
//! The value is encrypted with a key of its own, and that key with the master key (envelope
//! encryption, architecture §7 "Secrets and keys").

use jiff::Timestamp;

use crate::personal::Entity;
use crate::technical::TechnicalName;

/// One encrypted secret, as stored. Nothing here is readable without the master key.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct StoredSecret {
    /// What the secret is for, like `mail.password`.
    #[not_personal]
    pub name: TechnicalName,
    /// Which master key wrapped the key of this secret.
    #[not_personal]
    pub key_id: TechnicalName,
    /// The key of this secret, encrypted with the master key.
    #[not_personal]
    pub wrapped_key: Vec<u8>,
    /// The nonce used to wrap the key.
    #[not_personal]
    pub wrapped_nonce: Vec<u8>,
    /// The nonce used to encrypt the value.
    #[not_personal]
    pub nonce: Vec<u8>,
    /// The encrypted value.
    #[not_personal]
    pub ciphertext: Vec<u8>,
    /// When the value was last changed.
    #[not_personal]
    pub updated_at: Timestamp,
}
