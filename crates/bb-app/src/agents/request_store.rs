//! The port of requests for agents (ADR-0029, amendment M9): what a program asked for, and where it
//! stands.
//!
//! Like the other ports, reads take `&self` and **writes** are associated functions that run inside
//! `record_change`, so a change and its domain event are committed together (ARCH-EVENTS).

use std::future::Future;

use bb_core::agent_request::{AgentRequest, AskedAgent, Asker, RequestState};
use bb_core::id::{AgentRequestId, UserId};
use jiff::Timestamp;

use crate::ports::ChangeStore;
use crate::store_macros::{queries, reads, writes};

/// What the use cases of requests for agents need from storage.
pub trait AgentRequestStore: ChangeStore + 'static {
    queries! {
        /// The request with this ID.
        fn agent_request(id: AgentRequestId) -> Option<AgentRequest> = agent_request_in;
        /// How many requests of this key or grant wait for an answer at `now` (asked after
        /// `since`, the moment before which a waiting request ran out).
        fn waiting_requests_of(asker: Asker, since: Timestamp) -> i64 = waiting_requests_of_in;
    }

    reads! {
        /// The requests asked of a person since `since`, newest first, at most `limit`.
        fn agent_requests_of(person: UserId, since: Timestamp, limit: u64) -> Vec<AgentRequest>;
    }

    writes! {
        /// Adds a request.
        fn insert_agent_request(request: &AgentRequest) -> ();
        /// Moves a request from the state `from` to `to`, answered at `at`, with its agents; says
        /// whether it was in `from` (two answers at the same moment cannot both pass).
        fn answer_agent_request(id: AgentRequestId, from: RequestState, to: RequestState, at: Timestamp, agents: &[AskedAgent]) -> bool;
        /// Deletes at most `limit` requests asked before `before`; how many went.
        fn delete_old_agent_requests(before: Timestamp, limit: u64) -> u64;
    }
}
