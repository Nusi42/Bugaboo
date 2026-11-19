//! A link that works once, until it runs out (AUTH-06, PRIV-06): the one way Bugaboo makes the
//! links of invitations, of sign-ups and of a new e-mail address. Its secret is fresh randomness
//! that goes only into the link; what is kept is a digest of it and the moment it runs out. The
//! secret follows the `#` of the address, so it never reaches a log of a server.

use bb_auth::SecretToken;
use jiff::{SignedDuration, Timestamp};

use super::error::AppError;

/// A new link: its secret and when it stops working.
pub(crate) struct OneUseLink {
    /// The secret, which only the link holds.
    pub(crate) secret: SecretToken,
    /// When the link stops working.
    pub(crate) expires_at: Timestamp,
}

impl OneUseLink {
    /// A new link that works for `lasts` from `now`.
    pub(crate) fn new(now: Timestamp, lasts: SignedDuration) -> Result<Self, AppError> {
        Ok(Self {
            secret: SecretToken::generate()?,
            expires_at: now
                .checked_add(lasts)
                .map_err(|error| AppError::Internal(error.to_string()))?,
        })
    }

    /// The address of the link inside the app: the page that takes it, then the secret.
    pub(crate) fn path(&self, page: &str) -> String {
        format!("{page}#{}", self.secret.encode())
    }
}
