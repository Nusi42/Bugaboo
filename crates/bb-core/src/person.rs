//! A person with an account: the first entity, and the one the others point to.

use jiff::Timestamp;

use crate::id::UserId;
use crate::personal::Entity;

/// A person who can sign in to Bugaboo.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Person {
    /// The person's ID. It stays the same when the person is removed.
    #[not_personal]
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
}
