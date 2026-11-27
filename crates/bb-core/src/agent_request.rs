//! Requests for agents (AI-30, ADR-0029 amendment M9): a program asks a person for agents with a
//! reason; nothing is made until the person approves, and only the credential that asked collects
//! the keys, once.
//!
//! A request holds what was asked (the agents, what each is for, the days of their keys), who asked
//! (an access key of the person or the grant of an AI app) and where it stands. Whether it ran out
//! is never stored: it follows from the time ([`AgentRequest::stands_at`]).

use jiff::{SignedDuration, Timestamp};
use serde::{Deserialize, Serialize};

use crate::id::{AccessKeyId, AgentId, AgentRequestId, GrantId, UserId};
use crate::personal::Entity;

/// How long a request waits for the person's answer, and an approved one for its keys to be
/// collected, in hours.
pub const REQUEST_HOURS: i64 = 24;

/// How long a request is kept after it was asked, in days; then the daily job deletes it.
pub const REQUEST_KEEP_DAYS: i64 = 7;

/// The most requests one access key or grant may have waiting at once.
pub const MOST_WAITING_REQUESTS: usize = 3;

/// The most requests a person's list of requests shows: every key and grant has at most
/// [`MOST_WAITING_REQUESTS`] waiting, and the list reads back [`REQUEST_KEEP_DAYS`].
pub const REQUESTS_LISTED: u64 = 100;

/// The most requests one approval answers.
pub const MOST_APPROVED_AT_ONCE: usize = 10;

/// How long a request waits for an answer or for its keys.
#[must_use]
pub const fn waits() -> SignedDuration {
    SignedDuration::from_hours(REQUEST_HOURS)
}

words! {
    /// What was done with a request. Running out is not stored: it is the time (`stands_at`).
    pub enum RequestState {
        /// It waits for the person's answer.
        Waiting => "waiting",
        /// The person approved it: its agents are made, and its keys wait to be collected.
        Approved => "approved",
        /// The person declined it: nothing was made.
        Declined => "declined",
        /// The program that asked collected the keys.
        Collected => "collected",
    }
}

words! {
    /// Where a request stands at a moment, as the person and the program are told.
    pub enum RequestStands {
        /// It waits for the person's answer.
        Waiting => "waiting",
        /// Approved: the keys wait to be collected.
        Approved => "approved",
        /// Declined: nothing was made.
        Declined => "declined",
        /// The keys were collected.
        Collected => "collected",
        /// Nobody answered in time, or the keys were not collected in time.
        RanOut => "ran-out",
    }
}

crate::words_in_api!(
    RequestStands,
    "Where a request for agents stands: waiting, approved, declined, collected or ran out."
);

/// Who asked: an access key of the person, or the grant of an AI app. Only the same one collects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Asker {
    /// An access key of the person.
    Key(AccessKeyId),
    /// The grant of an AI app.
    Grant(GrantId),
}

/// One agent a program asks for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AskedAgent {
    /// The name the agent would have.
    pub name: String,
    /// What it is for, in a few words (never a role of permissions).
    pub purpose: String,
    /// How many days its key should last; none for the organization's default.
    pub key_days: Option<i64>,
    /// The agent the approval made; none before.
    pub agent: Option<AgentId>,
}

/// A request for agents. Every field carries its personal-data mark (ADR-0015).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct AgentRequest {
    /// The ID of the request.
    #[not_personal]
    pub id: AgentRequestId,
    /// The person asked: the owner of the key or the grant.
    #[personal(category = "identity", basis = "contract")]
    #[person(holder)]
    pub person: UserId,
    /// The access key that asked, if a key did.
    #[personal(category = "identity", basis = "contract")]
    #[names_no_person]
    pub asked_by_key: Option<AccessKeyId>,
    /// The grant of the AI app that asked, if an app did.
    #[personal(category = "identity", basis = "contract")]
    #[names_no_person]
    pub asked_by_grant: Option<GrantId>,
    /// The name of the app or the key that asked, as it is shown to the person.
    #[personal(category = "identity", basis = "contract")]
    pub asked_by_name: String,
    /// The web address the app that asked comes back to, as the person saw it when they allowed
    /// the app; none for a key and for an app that runs on the person's computer. The name of an
    /// app is its own choice, so the person sees where it lives beside it.
    #[personal(category = "identity", basis = "contract")]
    pub asked_from: Option<String>,
    /// Why the program asks.
    #[personal(category = "content", basis = "contract")]
    pub reason: String,
    /// The agents it asks for, in order, with the agents the approval made.
    #[personal(category = "content", basis = "contract")]
    pub agents: Vec<AskedAgent>,
    /// What was done with it.
    #[not_personal]
    pub state: RequestState,
    /// When it was asked.
    #[personal(category = "activity", basis = "contract")]
    pub asked_at: Timestamp,
    /// When the person answered, if they did.
    #[personal(category = "activity", basis = "contract")]
    pub answered_at: Option<Timestamp>,
}

impl AgentRequest {
    /// Who asked.
    #[must_use]
    pub fn asker(&self) -> Option<Asker> {
        match (self.asked_by_key, self.asked_by_grant) {
            (Some(key), None) => Some(Asker::Key(key)),
            (None, Some(grant)) => Some(Asker::Grant(grant)),
            _ => None,
        }
    }

    /// Until when the request waits: for the answer while it waits, for the pickup once approved.
    #[must_use]
    pub fn waits_until(&self) -> Timestamp {
        let from = match self.state {
            RequestState::Waiting => self.asked_at,
            _ => self.answered_at.unwrap_or(self.asked_at),
        };
        from.checked_add(waits()).unwrap_or(from)
    }

    /// Where it stands at `now`: a request that waited for an answer or its keys past
    /// [`REQUEST_HOURS`] ran out.
    #[must_use]
    pub fn stands_at(&self, now: Timestamp) -> RequestStands {
        match self.state {
            RequestState::Waiting | RequestState::Approved if now >= self.waits_until() => {
                RequestStands::RanOut
            }
            RequestState::Waiting => RequestStands::Waiting,
            RequestState::Approved => RequestStands::Approved,
            RequestState::Declined => RequestStands::Declined,
            RequestState::Collected => RequestStands::Collected,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::SequentialIds;

    fn request(state: RequestState, answered: Option<i64>) -> AgentRequest {
        let mut ids = SequentialIds::default();
        AgentRequest {
            id: AgentRequestId::generate(&mut ids).unwrap(),
            person: UserId::generate(&mut ids),
            asked_by_key: Some(AccessKeyId::generate(&mut ids)),
            asked_by_grant: None,
            asked_by_name: "Script".to_owned(),
            asked_from: None,
            reason: "A team".to_owned(),
            agents: Vec::new(),
            state,
            asked_at: Timestamp::from_second(0).unwrap(),
            answered_at: answered.map(|second| Timestamp::from_second(second).unwrap()),
        }
    }

    #[test]
    fn ai_30_a_request_runs_out_a_day_after_it_was_asked_or_approved() {
        let day = REQUEST_HOURS * 3600;
        let at = |second| Timestamp::from_second(second).unwrap();
        let waiting = request(RequestState::Waiting, None);
        assert_eq!(waiting.stands_at(at(day - 1)), RequestStands::Waiting);
        assert_eq!(waiting.stands_at(at(day)), RequestStands::RanOut);
        let approved = request(RequestState::Approved, Some(day - 10));
        assert_eq!(approved.stands_at(at(day)), RequestStands::Approved);
        assert_eq!(approved.stands_at(at(2 * day - 10)), RequestStands::RanOut);
        let declined = request(RequestState::Declined, Some(10));
        assert_eq!(declined.stands_at(at(3 * day)), RequestStands::Declined);
        assert_eq!(
            waiting.asker().map(|asker| matches!(asker, Asker::Key(_))),
            Some(true)
        );
    }
}
