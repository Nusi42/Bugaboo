//! Confirming an e-mail address (M1-F38, M4-Q10, ADR-0033 §3): once mail is on, an account made by
//! signing up on one's own works only after the person has opened the link sent to the address.

use jiff::{SignedDuration, Timestamp};

use crate::id::UserId;
use crate::personal::Entity;
use crate::security_log::Digest;

/// How long the link of a confirmation works, in hours.
pub const HOURS: i64 = 24;

/// The path of the screen that takes the link (the secret follows the `#`, so it never reaches a
/// log).
pub const PATH: &str = "/confirm-address";

/// How long the link works.
#[must_use]
pub const fn lasts() -> SignedDuration {
    SignedDuration::from_hours(HOURS)
}

/// An account that waits for its address to be confirmed. A person with no such row has a
/// confirmed address, or was made another way (an invitation, the first start).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct AddressConfirmation {
    /// The person whose address waits.
    #[personal(category = "identity", basis = "contract")]
    pub person: UserId,
    /// The hash of the secret in the link. The secret is only in the e-mail.
    #[personal(category = "credentials", basis = "contract")]
    pub token_hash: Digest,
    /// When the account was made.
    #[not_personal]
    pub created_at: Timestamp,
    /// When the link stops working (24 hours after it was sent).
    #[not_personal]
    pub expires_at: Timestamp,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_the_link_works_for_twenty_four_hours() {
        assert_eq!(lasts().as_secs(), 24 * 60 * 60);
    }
}
