//! Agent work (AI-31 to AI-35, ADR-0029): hand-overs, standing rules, the work on an issue with its
//! attempts, questions and plans, and the review. The rules themselves are in
//! `bb_core::agent_work`; this module has the port and the use cases.

mod attempt;
mod changes;
mod finish;
mod hand_over;
mod members;
mod narrow;
mod plan;
mod queue;
mod read;
mod review;
mod rules;
mod settings;
mod stop;
mod store;
mod take;
mod types;

pub use attempt::{ask_question, attach_result, end_attempt, report_attempt};
pub use changes::{ChangeSeen, ChangesPage, ChangesSinceQuery, MatchSeen, WorkSeen, changes_since};
pub use hand_over::hand_over;
pub(crate) use members::{event_member, payload as job_payload, waiting_for};
pub(crate) use narrow::issue_member;
pub use read::issue_agent_work;
pub use review::{answer_question, approve_plan, look_at_work, review_work};
pub use rules::{
    change_standing_rule, create_standing_rule, decide_proposal, propose_change, standing_rules,
};
#[cfg(test)]
pub(crate) use stop::TRIES;
pub use stop::{Closing, close_work_of};
pub use store::{AgentWorkRecords, GrantStore, WorkDetails};
pub use take::take_work;
pub use types::*;
