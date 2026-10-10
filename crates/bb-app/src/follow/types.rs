//! What the follow use cases take and give back (ISS-07).

use bb_core::follow::FollowReason;
use bb_policy::{Masked, PersonRef};
use serde::Serialize;

/// Follow an issue.
pub struct FollowIssue {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID or its internal ID.
    pub issue: String,
}

/// Stop following an issue.
pub struct UnfollowIssue {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID or its internal ID.
    pub issue: String,
}

/// Who follows an issue.
pub struct FollowersQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID or its internal ID.
    pub issue: String,
    /// Where the page starts: the `next_cursor` of the page before.
    pub cursor: Option<String>,
    /// How many followers the page holds at most.
    pub limit: u32,
}

/// The followers of an issue as one viewer sees them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Followers {
    /// Whether the viewer follows the issue.
    pub following: bool,
    /// Why the viewer follows it, when they do.
    pub reason: Option<FollowReason>,
    /// The followers, each as the kind *Followers* says for this viewer.
    pub items: Vec<PersonRef>,
    /// The cursor of the next page, if there is one.
    pub next_cursor: Option<String>,
}

impl Masked for Followers {}
