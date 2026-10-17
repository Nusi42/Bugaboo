//! Signing up on one's own (AUTH-06, M1-F38, ADR-0033 §3): the person gives only an address.
//! Bugaboo sends a link to it, and the link is an invitation like the one an administrator makes:
//! opening it asks for a name and a password and makes the account. So nobody can make an account
//! on an address they cannot read, and there is one way to join.

use jiff::{SignedDuration, Timestamp};

use crate::id::InvitationId;
use crate::personal::Entity;
use crate::security_log::Digest;
use crate::sign_in::Invitation;

/// How long the link of a sign-up works, in hours.
pub const HOURS: i64 = 24;

/// How long the link works.
#[must_use]
pub const fn lasts() -> SignedDuration {
    SignedDuration::from_hours(HOURS)
}

/// How many minutes a sign-up for an address waits for the one before it. Somebody who signs up for
/// an address again sooner gets no second e-mail, so signing up cannot be used to fill the inbox of
/// somebody else.
pub const RESEND_AFTER_MINUTES: i64 = 10;

/// How long after the link ran out its row stays before the job that clears sessions deletes it, in
/// hours.
pub const KEEP_HOURS: i64 = 24;

/// The last e-mail that went to an address for a sign-up, the one fact kept so that signing up
/// again and again cannot fill the inbox of the address, whether it has an account or not. The
/// address itself is not kept, only its digest.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct SignUpMail {
    /// The digest of the address.
    #[personal(category = "identity", basis = "contract")]
    pub address: Digest,
    /// The sign-up the e-mail was for, so a second try of the same job is not a second e-mail.
    #[not_personal]
    pub invitation: InvitationId,
    /// When it went.
    #[personal(category = "activity", basis = "contract")]
    pub sent_at: Timestamp,
}

/// The invitation a sign-up makes. It is read as an [`crate::sign_in::Invitation`] with no person
/// who invited.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct SignUpInvitation {
    /// The ID of the invitation.
    #[not_personal]
    pub id: InvitationId,
    /// The hash of the secret in the link. The secret is only in the e-mail.
    #[personal(category = "credentials", basis = "contract")]
    pub token_hash: Digest,
    /// The address the account will get.
    #[personal(category = "identity", basis = "contract")]
    pub email: String,
    /// When the link was made.
    #[personal(category = "activity", basis = "contract")]
    pub created_at: Timestamp,
    /// When it stops working (24 hours after it was made).
    #[not_personal]
    pub expires_at: Timestamp,
    /// When it was used. A used link never works again.
    #[personal(category = "activity", basis = "contract")]
    pub used_at: Option<Timestamp>,
}

/// A link as the invitation use cases read it: an invitation nobody made.
impl From<SignUpInvitation> for Invitation {
    fn from(link: SignUpInvitation) -> Self {
        Self {
            id: link.id,
            token_hash: link.token_hash,
            email: link.email,
            invited_by: None,
            created_at: link.created_at,
            expires_at: link.expires_at,
            used_at: link.used_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_the_link_works_for_twenty_four_hours() {
        assert_eq!(lasts().as_secs(), 24 * 60 * 60);
    }
}
