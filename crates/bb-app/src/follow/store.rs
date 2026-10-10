//! The port of the follow use cases: what they need from storage beyond the issues port.

use std::future::Future;

use bb_core::follow::Follower;
use bb_core::id::{IssueId, UserId};

use crate::issues::IssueStore;
use crate::store_macros::{queries, writes};

/// What the follow use cases need from storage.
pub trait FollowStore: IssueStore + 'static {
    queries! {
        /// The people who follow an issue, in the order they started.
        fn followers_of(issue: IssueId) -> Vec<Follower> = followers_of_in;
    }

    writes! {
        /// Adds followers; one who follows already stays as they are.
        fn add_followers(followers: &[Follower]) -> ();
        /// Takes a follower away; `false` when the person did not follow.
        fn remove_follower(issue: IssueId, person: UserId) -> bool;
    }
}
