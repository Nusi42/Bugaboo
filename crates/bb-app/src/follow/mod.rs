//! Following issues (ISS-07): who follows, *Follow* and *Following*, and the one rule for who
//! starts following by reporting, being assigned or commenting (`bb_core::follow`). Delivery
//! (e-mail, the bell) comes in M4; in M3 followers are stored and shown.

mod store;
mod types;
mod use_cases;

pub use store::FollowStore;
pub use types::*;
pub use use_cases::{follow_issue, followers, following, unfollow_issue};

use bb_core::follow::{Follower, who_starts_following};
use bb_core::id::{IssueId, UserId};
use jiff::Timestamp;

/// The rows of the people who start following an issue because of what just happened, by the one
/// rule of `bb_core::follow`. The use cases that report, assign and comment write them in the
/// same change as the thing itself.
pub(crate) fn started_following(
    issue: IssueId,
    reporter: Option<UserId>,
    assigned: &[UserId],
    commenter: Option<UserId>,
    at: Timestamp,
) -> Vec<Follower> {
    who_starts_following(reporter, assigned, commenter)
        .into_iter()
        .map(|(person, reason)| Follower {
            issue,
            person,
            reason,
            since: at,
        })
        .collect()
}
