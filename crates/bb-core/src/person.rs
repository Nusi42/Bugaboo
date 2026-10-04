//! A person with an account: the first entity, and the one the others point to.

use jiff::Timestamp;

use crate::id::UserId;
use crate::personal::Entity;

/// A person who can sign in to Bugaboo.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Person {
    /// The person's ID, fully random so it does not show when the account was made. It is a
    /// pseudonym for the person; after the person is removed it is kept only as the link to the
    /// "Former user …" tombstone.
    #[personal(category = "identity", basis = "legitimate_interests")]
    pub id: UserId,
    /// The name others see, as far as *Who sees people* allows (ADR-0022).
    #[personal(category = "identity", basis = "contract")]
    pub display_name: String,
    /// The e-mail address used to sign in and to send notifications.
    #[personal(category = "identity", basis = "contract")]
    pub email: String,
    /// When the account was made.
    #[personal(category = "activity", basis = "contract")]
    pub created_at: Timestamp,
    /// Whether the person is an administrator. Roles replace this with the permissions engine
    /// (M2); until then it is the only difference between people.
    #[not_personal]
    pub administrator: bool,
}
