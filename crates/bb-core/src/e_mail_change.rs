//! A change of one's own e-mail address (PRIV-06, ADR-0044, decision 9): it waits until the person
//! opens the link that was sent to the new address. The old address keeps working until then.

use jiff::{SignedDuration, Timestamp};

use crate::id::UserId;
use crate::personal::Entity;
use crate::security_log::Digest;

/// How long the link that confirms a new address works, in hours.
pub const LINK_HOURS: i64 = 1;

/// How long the link works.
#[must_use]
pub const fn lasts() -> SignedDuration {
    SignedDuration::from_hours(LINK_HOURS)
}

/// The change of a person's address that waits for its link: at most one for each person, and a
/// new one replaces it. It goes when the link is opened, and the daily job deletes it once the
/// link has run out.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct EmailChange {
    /// Whose address changes.
    #[personal(category = "identity", basis = "contract")]
    #[person(subject)]
    pub person: UserId,
    /// The keyed digest of the secret in the link (`bb_auth::DigestKey`). The secret is only in
    /// the e-mail.
    #[personal(category = "credentials", basis = "contract")]
    pub token_digest: Digest,
    /// The address the person asked for.
    #[personal(category = "identity", basis = "contract")]
    pub new_email: String,
    /// When the person asked for it.
    #[personal(category = "activity", basis = "contract")]
    pub asked_at: Timestamp,
    /// When the link stops working ([`LINK_HOURS`] after it was sent).
    #[not_personal]
    pub expires_at: Timestamp,
}

impl EmailChange {
    /// Whether the link still works at `now`.
    #[must_use]
    pub fn works_at(&self, now: Timestamp) -> bool {
        now < self.expires_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::SequentialIds;

    #[test]
    fn priv_06_the_link_works_for_an_hour() {
        let asked_at = Timestamp::from_second(1_000).unwrap();
        let change = EmailChange {
            person: UserId::generate(&mut SequentialIds::default()),
            token_digest: Digest::new([0; 32]),
            new_email: "new@example.test".to_owned(),
            asked_at,
            expires_at: asked_at.checked_add(lasts()).unwrap(),
        };
        assert!(change.works_at(asked_at));
        let end = asked_at
            .checked_add(SignedDuration::from_hours(LINK_HOURS))
            .unwrap();
        assert!(!change.works_at(end));
    }
}
