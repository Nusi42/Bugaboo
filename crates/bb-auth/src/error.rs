//! What can go wrong inside this crate. These texts are for the log, never for a person: the API
//! answers with an error code from `bb-core` (architecture §7, "Errors").

use std::fmt;

/// Something failed in a mechanism of signing in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthError {
    /// The system gave no secure random bytes.
    NoRandomness,
    /// Hashing a password failed (the parameters or the memory).
    Hashing,
    /// The code app secret or its address could not be made.
    Totp,
    /// The passkey library refused: the answer of the device did not check out, or the settings
    /// of the relying party are not usable.
    Passkey,
    /// The passkey challenge is unknown, used already, or too old.
    Challenge,
    /// Too many passkey challenges are waiting.
    TooManyChallenges,
    /// The address of the server is not usable as the origin of passkeys.
    RelyingParty,
}

impl fmt::Display for AuthError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NoRandomness => "the system gave no secure random bytes",
            Self::Hashing => "hashing a password failed",
            Self::Totp => "the code app secret could not be made",
            Self::Passkey => "the passkey could not be checked",
            Self::Challenge => "the passkey challenge is unknown, used or too old",
            Self::TooManyChallenges => "too many passkey challenges are waiting",
            Self::RelyingParty => "the public address is not usable for passkeys",
        })
    }
}

impl std::error::Error for AuthError {}

/// Fresh secure random bytes.
pub(crate) fn random<const N: usize>() -> Result<[u8; N], AuthError> {
    let mut bytes = [0_u8; N];
    getrandom::fill(&mut bytes).map_err(|_| AuthError::NoRandomness)?;
    Ok(bytes)
}
