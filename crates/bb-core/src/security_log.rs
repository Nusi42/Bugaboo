//! The security log (SEC-15): who did what that matters for security, in an append-only chain.
//!
//! Every entry stores the hash of the entry before it, so a changed, removed or inserted entry
//! breaks the chain. Old entries are deleted in whole blocks, leaving a signed checkpoint (the
//! hash of the last deleted entry) so the rest still verifies. The hashing is in `bb-store`.

use core::fmt;

use jiff::Timestamp;

use crate::event::EntityRef;
use crate::id::UserId;
use crate::network::NetworkPrefix;
use crate::personal::Entity;
use crate::technical::TechnicalName;

/// A SHA-256 hash or a signature of the same size.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Digest([u8; 32]);

impl Digest {
    /// The size in bytes.
    pub const LENGTH: usize = 32;

    /// The digest made of exactly these bytes.
    #[must_use]
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// The digest of a stored value, if it has the right size.
    #[must_use]
    pub fn from_slice(bytes: &[u8]) -> Option<Self> {
        <[u8; 32]>::try_from(bytes).ok().map(Self)
    }

    /// The bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for Digest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0[..4] {
            write!(formatter, "{byte:02x}")?;
        }
        formatter.write_str("..")
    }
}

// Bytes of a hash; nothing in them says anything about a person.
impl crate::personal::NotPersonal for Digest {}

/// One entry of the security log.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct SecurityLogEntry {
    /// The place in the chain, counting from 1 without gaps.
    #[not_personal]
    pub seq: i64,
    /// When it happened.
    #[not_personal]
    pub time: Timestamp,
    /// Who did it, or `None` when nobody was signed in (for example a failed sign-in).
    #[personal(category = "identity", basis = "legal_obligation")]
    pub person: Option<UserId>,
    /// What happened, like `sign_in.failed`.
    #[not_personal]
    pub action: TechnicalName,
    /// What it was about, if one record.
    #[not_personal]
    pub subject: Option<EntityRef>,
    /// Where it came from, shortened (IPv4 `/24`, IPv6 `/48`).
    #[personal(category = "technical", basis = "legal_obligation")]
    pub network: Option<NetworkPrefix>,
    /// The hash of the entry before this one (all zeros for the first entry ever).
    #[not_personal]
    pub previous_hash: Digest,
    /// The hash of this entry, made from the previous hash and all fields above.
    #[not_personal]
    pub hash: Digest,
}

/// What is left of the deleted entries: the last deleted entry's place and hash, signed so nobody
/// can invent a start for the chain.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct SecurityLogCheckpoint {
    /// The place of the last deleted entry. The first remaining entry is the next one.
    #[not_personal]
    pub seq: i64,
    /// The hash of the last deleted entry.
    #[not_personal]
    pub hash: Digest,
    /// The signature over place and hash, made with a key derived from the master key.
    #[not_personal]
    pub signature: Digest,
    /// When the entries were deleted.
    #[not_personal]
    pub created_at: Timestamp,
}
