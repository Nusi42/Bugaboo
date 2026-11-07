//! The port of the sign-in of AI apps: what the use cases of `oauth.rs` need from storage
//! (AI-22, ADR-0042).
//!
//! Like the other ports, reads take `&self` and **writes** are associated functions that run inside
//! `record_change`, so a change and its domain event are committed together (ARCH-EVENTS).

use std::future::Future;

use bb_core::id::{GrantId, OAuthClientId, ProjectId, UserId};
use bb_core::oauth::{GrantEnd, OAuthClient, OAuthCode, OAuthGrant, OAuthToken, OAuthWebAddress};
use bb_core::security_log::Digest;
use jiff::Timestamp;

use crate::ports::ChangeStore;
use crate::store_macros::{reads, writes};

/// A grant with the projects it is limited to; none for every project of its person.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrantRecord {
    /// The grant.
    pub grant: OAuthGrant,
    /// The projects it counts in, with their sub-projects.
    pub projects: Vec<ProjectId>,
}

/// What a clean-up of the sign-in of apps removed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OAuthCleaned {
    /// Codes and tokens that ran out.
    pub secrets: u64,
    /// Grants that ended long ago.
    pub grants: u64,
    /// Apps that registered and were never allowed.
    pub clients: u64,
}

/// The records of apps, grants, codes and tokens.
pub trait OAuthStore: ChangeStore + 'static {
    reads! {
        /// The app with this ID.
        fn oauth_client(id: OAuthClientId) -> Option<OAuthClient>;
        /// The grant with this ID, with its projects.
        fn oauth_grant(id: GrantId) -> Option<GrantRecord>;
        /// The grants of a person that have not ended, newest first, at most `limit`.
        fn grants_of(person: UserId, limit: u64) -> Vec<GrantRecord>;
        /// How many grants of a person have not ended.
        fn grant_count_of(person: UserId) -> i64;
        /// The token with this hash.
        fn oauth_token(hash: &Digest) -> Option<OAuthToken>;
        /// The code with this hash.
        fn oauth_code(hash: &Digest) -> Option<OAuthCode>;
        /// The web addresses administrators allowed, by address.
        fn oauth_web_addresses() -> Vec<OAuthWebAddress>;
        /// Records when a grant was used, to the minute. Bookkeeping, not a domain change.
        fn touch_grant(id: GrantId, at: Timestamp) -> ();
    }

    writes! {
        /// Adds an app.
        fn insert_oauth_client(client: &OAuthClient) -> ();
        /// Adds a grant with its projects.
        fn insert_grant(record: &GrantRecord) -> ();
        /// Adds a code.
        fn insert_oauth_code(code: &OAuthCode) -> ();
        /// Marks a code used and says whether it was unused until now.
        fn use_oauth_code(hash: &Digest, at: Timestamp) -> bool;
        /// Adds a token.
        fn insert_oauth_token(token: &OAuthToken) -> ();
        /// Marks a token used and says whether it was unused until now.
        fn use_oauth_token(hash: &Digest, at: Timestamp) -> bool;
        /// Ends a grant that has not ended, takes away its codes and tokens, and says whether it
        /// was ended by this call.
        fn end_grant(id: GrantId, how: GrantEnd, at: Timestamp) -> bool;
        /// Ends every grant of a person and takes away their codes and tokens; the IDs ended.
        fn end_grants_of(person: UserId, how: GrantEnd, at: Timestamp) -> Vec<GrantId>;
        /// Allows a web address (`false` when it was allowed already).
        fn insert_oauth_web_address(address: &OAuthWebAddress) -> bool;
        /// Takes a web address away (`false` when it was not there).
        fn delete_oauth_web_address(address: &str) -> bool;
        /// Deletes what ran out: codes and tokens past their time, grants that ended `ended_before`
        /// or earlier, and apps registered before `registered_before` that no grant belongs to.
        fn clean_oauth(
            now: Timestamp,
            ended_before: Timestamp,
            registered_before: Timestamp
        ) -> OAuthCleaned;
    }
}
